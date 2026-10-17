# GitSwamp Auto-Updater

How GitSwamp checks for, downloads, and installs updates, and exactly what you must
build, sign, and publish to every GitHub release for it to work.

---

## 1. How it works

| Part | Location | Responsibility |
| --- | --- | --- |
| Release discovery + update check | `src-tauri/src/commands/updates.rs` (`check_for_update`) | Queries the GitHub Releases API, picks the right release, then asks `tauri-plugin-updater` whether it is newer |
| Install | `src-tauri/src/commands/updates.rs` (`install_update`) | Downloads the signed artifact and installs it, streaming progress over an IPC `Channel` |
| UI | `src/features/updater/AppUpdatePanel.vue` + `useAppUpdate.ts` | Bottom-left panel with **Install** / **Dismiss**; only rendered when an update exists |
| State | `PendingUpdate` (managed in `lib.rs`) | Holds the downloaded-and-verified `Update` between the check and the install |
| Update manifest | `latest.json` uploaded to each GitHub release | Announces `version`, `notes`, `pub_date`, and per-platform `url` + `signature` |
| Signing key | `~/.tauri/gitswamp.key` (private) / `tauri.conf.json` `plugins.updater.pubkey` (public) | Signs artifacts and verifies them in the app |

**Flow:** on every app start `AppView.vue` calls `checkForAppUpdate()` → Rust fetches
`https://api.github.com/repos/mgrabovac22/gitswamp/releases?per_page=30` → picks the
newest usable release (see §6) → points `tauri-plugin-updater` at that release's
`latest.json` → if the manifest version is newer than the running app, the panel
appears. **Install** downloads the artifact, verifies its signature against the embedded
public key, installs it, and relaunches.

The panel is intentionally hidden unless there is a newer release. Dismissing stores the
tag in `localStorage` (`gitswamp-dismissed-update`) so it stays hidden for that version.

---

## 2. One-time setup (already done in this repo)

### 2.1 Signing keypair

An update is only accepted if its artifact signature matches the public key embedded in
the app. The keypair must be generated once and then **kept forever** — losing it (or
changing it) breaks updates for everyone who already installed a build.

```bash
npx tauri signer generate -w ~/.tauri/gitswamp.key
```

This creates:

- `~/.tauri/gitswamp.key` — **private key. Never commit it, never lose it.**
- `~/.tauri/gitswamp.key.pub` — public key (a base64 blob).

Put the **contents** of the `.pub` file into `src-tauri/tauri.conf.json`:

```json
{
  "plugins": {
    "updater": {
      "pubkey": "<contents of ~/.tauri/gitswamp.key.pub>"
    }
  }
}
```

> The `pubkey` value is literally the file contents (base64), **not** a path to the file.

To regenerate (only before any release is public, or to rotate keys — see §9):

```bash
npx tauri signer generate -w ~/.tauri/gitswamp.key --force
# then paste the new ~/.tauri/gitswamp.key.pub contents into tauri.conf.json
```

### 2.2 Repository references

- Repository: `https://github.com/mgrabovac22/gitswamp`
- Releases API: `https://api.github.com/repos/mgrabovac22/gitswamp/releases?per_page=30`
- Per-tag manifest URL: `https://github.com/mgrabovac22/gitswamp/releases/download/<tag>/latest.json`
- Static fallback endpoint: `https://github.com/mgrabovac22/gitswamp/releases/latest/download/latest.json`

These live in `src-tauri/src/commands/updates.rs` (constants), `tauri.conf.json`
(`plugins.updater.endpoints`), and `scripts/generate-update-manifest.mjs` (`REPO`).

### 2.3 Required config (already present)

`src-tauri/tauri.conf.json`:

```json
{
  "plugins": {
    "updater": {
      "pubkey": "…",
      "endpoints": [
        "https://github.com/mgrabovac22/gitswamp/releases/latest/download/latest.json"
      ],
      "windows": { "installMode": "passive" }
    }
  },
  "bundle": { "createUpdaterArtifacts": true }
}
```

`src-tauri/Cargo.toml`:

```toml
tauri-plugin-updater = { version = "2.14", default-features = false, features = ["rustls-tls", "system-proxy"] }
tauri-plugin-process = "2"
```

`src-tauri/capabilities/default.json` must include `"updater:default"` and
`"process:default"` (the latter provides `allow-restart`, used by `relaunch()`).

> **Why `default-features = false`?** `tauri-plugin-updater`'s default `zip` feature pulls
> in a `zip` crate version that is fully yanked in the registry. Disabling it removes the
> dependency entirely. Windows still installs via the NSIS `.exe` and Linux via the
> AppImage (see §5), both of which work without the `zip` feature — but this is exactly
> why we must publish `createUpdaterArtifacts: true` artifacts and **not** the older
> `"v1Compatible"` `.zip`/`.tar.gz` wrappers.

---

## 3. Bump the version

The updater compares against the version baked into the app from
`src-tauri/tauri.conf.json`. Keep all three in sync:

- `package.json` → `"version"`
- `src-tauri/tauri.conf.json` → `"version"`
- `src-tauri/Cargo.toml` → `[package] version`

Use `X.Y.Z` (no leading `v`). The Git tag may include a `v` (e.g. `v0.5.0`).

---

## 4. Build and sign a release

Set the private key in the environment so the bundler produces `.sig` files:

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/gitswamp.key)"
# or point at the file instead:
# export TAURI_SIGNING_PRIVATE_KEY_PATH="$HOME/.tauri/gitswamp.key"

# only if the key is password-protected:
# export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="your-password"

npm run tauri build
```

If the key is missing, `TAURI_SIGNING_PRIVATE_KEY` is unset, or the key is wrong, the
build produces no `.sig` files and `latest.json` cannot be generated.

---

## 5. Updater artifacts (what the bundler produces)

With `createUpdaterArtifacts: true`, Tauri signs the updater artifact. Linux uses an
`AppImage.tar.gz` updater archive, while Windows and macOS use their platform-specific
updater artifacts. Default locations are under `src-tauri/target/release/bundle/`.

| Platform | Bundle folder | Updater artifact | Signature |
| --- | --- | --- | --- |
| Linux (x64) | `bundle/appimage/` | `GitSwamp_<version>_amd64.AppImage.tar.gz` | `GitSwamp_<version>_amd64.AppImage.tar.gz.sig` |
| Linux (arm64) | `bundle/appimage/` | `GitSwamp_<version>_aarch64.AppImage.tar.gz` | `…aarch64.AppImage.tar.gz.sig` |
| Windows (x64) | `bundle/nsis/` | `GitSwamp_<version>_x64-setup.exe` | `GitSwamp_<version>_x64-setup.exe.sig` |
| Windows (arm64) | `bundle/nsis/` | `GitSwamp_<version>_arm64-setup.exe` | `…arm64-setup.exe.sig` |
| macOS (Intel) | `bundle/macos/` | `GitSwamp.app.tar.gz` → **rename to** `GitSwamp_<version>_x64.app.tar.gz` | `…_x64.app.tar.gz.sig` |
| macOS (Apple Silicon) | `bundle/macos/` | `GitSwamp.app.tar.gz` → **rename to** `GitSwamp_<version>_aarch64.app.tar.gz` | `…_aarch64.app.tar.gz.sig` |

Notes:

- **Linux auto-update requires the AppImage.** A `.deb`/`.rpm` installation cannot be
  replaced by the AppImage updater because the running installation format controls how
  Tauri installs the update. Install the published `.AppImage` once; later updates use
  the signed `.AppImage.tar.gz` updater artifact and replace that AppImage in place.
- **Windows uses the NSIS `-setup.exe`.** Do not rely on the `.msi` for updates.
- **macOS tarballs have identical names for both architectures**, so you must rename each
  one to include `_x64` or `_aarch64` before uploading, otherwise the two collide and the
  manifest script cannot tell them apart.

---

## 6. Which release the app picks

The app filters out drafts and any release **without a `latest.json` asset**, then:

1. Finds the newest **stable** release (by `published_at`).
2. Finds the newest **prerelease** (by `published_at`).
3. Uses the prerelease **only if it is chronologically newer** than the newest stable;
   otherwise it uses the stable release.

So a newly published prerelease is offered to users, but an older prerelease is ignored in
favour of a newer stable. Releases built before the updater existed (no `latest.json`) are
skipped — auto-update effectively starts from the first release that includes `latest.json`.

---

## 7. Generate `latest.json`

Run the helper after building (it scans the bundle folder for `*.sig` files):

```bash
npm run updater:manifest -- \
  --tag v0.5.0 \
  --notes "What's new in 0.5.0"
```

What it does:

- Reads the version from `--tag` (strips a leading `v`; override with `--version`) and
  warns if it does not match `src-tauri/tauri.conf.json`.
- Walks `src-tauri/target/release/bundle` (override with `--bundle-dir`).
- For every `<artifact>.sig`, pairs the artifact with its signature and maps it to a Tauri
  platform key: `linux-x86_64`, `linux-aarch64`, `windows-x86_64`, `windows-aarch64`,
  `darwin-x86_64`, `darwin-aarch64`.
- Writes the GitHub download URL for each asset:
  `https://github.com/mgrabovac22/gitswamp/releases/download/<tag>/<asset-name>`.
- Writes `latest.json` (override the path with `--out`).

Example output:

```json
{
  "version": "0.5.0",
  "notes": "…",
  "pub_date": "2026-10-16T13:06:34.506Z",
  "platforms": {
    "linux-x86_64": {
      "signature": "<contents of GitSwamp_0.5.0_amd64.AppImage.tar.gz.sig>",
      "url": "https://github.com/mgrabovac22/gitswamp/releases/download/v0.5.0/GitSwamp_0.5.0_amd64.AppImage.tar.gz"
    },
    "windows-x86_64": {
      "signature": "<contents of …-setup.exe.sig>",
      "url": "https://github.com/mgrabovac22/gitswamp/releases/download/v0.5.0/GitSwamp_0.5.0_x64-setup.exe"
    }
  }
}
```

Key requirements:

- The `tag` passed here **must match the Git tag of the GitHub release**, because it is
  embedded in every asset URL.
- The asset **file names must match exactly** what you upload (including the macOS
  `_x64`/`_aarch64` rename).
- `signature` must be the **contents** of the `.sig` file (the script does this), never a
  path or URL.

---

## 8. What to upload to each GitHub release

Upload the updater artifact **and its `.sig`** for every platform you built, plus
`latest.json`. The standard (non-updater) installers are optional.

Checklist:

- [ ] `GitSwamp_<version>_amd64.AppImage.tar.gz` + `GitSwamp_<version>_amd64.AppImage.tar.gz.sig` (Linux updater)
- [ ] `GitSwamp_<version>_x64-setup.exe` + `GitSwamp_<version>_x64-setup.exe.sig` (Windows)
- [ ] `GitSwamp_<version>_x64.app.tar.gz` + `.sig` (macOS Intel, after rename)
- [ ] `GitSwamp_<version>_aarch64.app.tar.gz` + `.sig` (macOS ARM, after rename)
- [ ] `latest.json`
- [ ] (optional) `.deb`, `.rpm`, `.msi` — not used by the updater

Do **not** upload the private key. The `.sig` files and `latest.json` are public.

### Upload with the GitHub CLI

Prerelease:

```bash
gh release create v0.5.0 \
  --title "GitSwamp v0.5.0" \
  --generate-notes \
  --prerelease \
  "src-tauri/target/release/bundle/appimage/GitSwamp_0.5.0_amd64.AppImage" \
  "src-tauri/target/release/bundle/appimage/GitSwamp_0.5.0_amd64.AppImage.sig" \
  "src-tauri/target/release/bundle/nsis/GitSwamp_0.5.0_x64-setup.exe" \
  "src-tauri/target/release/bundle/nsis/GitSwamp_0.5.0_x64-setup.exe.sig" \
  latest.json
```

Stable — same command without `--prerelease`.

For an existing release, upload with:

```bash
gh release upload v0.5.0 latest.json \
  "src-tauri/target/release/bundle/appimage/GitSwamp_0.5.0_amd64.AppImage" \
  "src-tauri/target/release/bundle/appimage/GitSwamp_0.5.0_amd64.AppImage.sig" \
  --clobber
```

### Upload with the web UI

1. GitHub → repository → **Releases** → **Draft a new release**.
2. Choose/create the tag (e.g. `v0.5.0`). The tag name must equal the `--tag` you used for
   `latest.json`.
3. Set the title and paste the release notes.
4. Tick **Set as a pre-release** only if this is a prerelease.
5. Attach the artifacts and their `.sig` files, plus `latest.json` (drag & drop).
6. **Publish release** — a draft is invisible to the app.

After publishing, open
`https://github.com/mgrabovac22/gitswamp/releases/download/v0.5.0/latest.json`
in a browser to confirm it is reachable and correct.

---

## 9. Testing an update

1. Build a "current" install with the version you already have (e.g. `0.4.0`) and install
   it.
2. Bump to the next version, build + sign, generate `latest.json`, publish the release
   (as a prerelease if you want).
3. Start the installed `0.4.0` app. On startup it checks, finds the newer release, and
   shows the bottom-left update panel.
4. Click **Install** — the app downloads, verifies the signature, installs, and relaunches
   into the new version.

Tips:

- A release with an **older/same** version than the running app produces no update.
- A release **without** `latest.json` is ignored.
- Dismiss is remembered per version; clear `gitswamp-dismissed-update` in the webview's
  localStorage (or install a newer version) to see the panel again for the same version.

---

## 10. Troubleshooting

| Symptom | Cause / fix |
| --- | --- |
| Panel never appears | No release has `latest.json`; or the manifest version is not newer; or the release is a **draft**. Verify the per-tag `latest.json` URL loads. |
| `Update check failed: … 404` | The `latest.json` asset for that tag is missing, or the tag in asset URLs doesn't match the release tag. |
| `Update check failed: … 403` | GitHub API rate limit (unauthenticated: 60 requests/hour). Retry later. |
| `Update install failed: signature` | The artifact was not signed with the same key whose public key is embedded in `tauri.conf.json`, or `latest.json`'s `signature` doesn't match the uploaded artifact. |
| Build produced no `.sig` | `TAURI_SIGNING_PRIVATE_KEY` (or `_PATH`) not set, or the password missing/wrong. |
| `failed to select a version for … zip` | `tauri-plugin-updater` was added with default features. Use `default-features = false, features = ["rustls-tls", "system-proxy"]`. |
| Update downloads but fails to install on Linux | The manifest points at the AppImage but the app was installed from a `.deb`/`.rpm` (or vice-versa). Use the AppImage as the update channel. |
| macOS builds collide on upload | Rename the tarballs to include `_x64` / `_aarch64` (§5) before generating the manifest. |

---

## 11. Security notes

- **Never commit the private key.** It lives outside the repo at `~/.tauri/gitswamp.key`;
  `.gitignore` also blocks `*.key`, `*.key.pub`, and `*.pem`.
- **Back the private key up** somewhere safe (password manager / secret store). If it is
  lost, no new update can be signed for existing installs.
- On CI, store it as an encrypted secret and export
  `TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` for the build step
  (mirrors how `tauri-action` works).
- `latest.json` itself is fetched over HTTPS but is **not signed**; only the downloaded
  artifacts are signature-verified. Optional hardening: set
  `"requireSignedVersion": true` in `plugins.updater` to also bind the signature to a
  version (verify this is set on every artifact before enabling it, and re-sign older
  releases users may still update from).
- Rotating the signing key means every already-installed app keeps the **old** embedded
  public key and will reject releases signed with the new key — plan a migration release
  signed with the old key before switching.

---

## 12. Release-run cheat sheet

```bash
# 1. Bump version in package.json, src-tauri/tauri.conf.json, src-tauri/Cargo.toml

# 2. Build + sign
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/gitswamp.key)"
npm run tauri build

# 3. (macOS only) rename GitSwamp.app.tar.gz to include _x64 / _aarch64

# 4. Generate the manifest
npm run updater:manifest -- --tag v0.5.0 --notes "What's new in 0.5.0"

# 5. Publish
gh release create v0.5.0 --title "GitSwamp v0.5.0" --generate-notes \
  "src-tauri/target/release/bundle/appimage/GitSwamp_0.5.0_amd64.AppImage" \
  "src-tauri/target/release/bundle/appimage/GitSwamp_0.5.0_amd64.AppImage.sig" \
  "src-tauri/target/release/bundle/nsis/GitSwamp_0.5.0_x64-setup.exe" \
  "src-tauri/target/release/bundle/nsis/GitSwamp_0.5.0_x64-setup.exe.sig" \
  latest.json
```

---

## 13. Manual releases (no CI) — make the updater work with physical uploads

You do **not** need GitHub Actions, a build pipeline, or any repository secrets for the
updater to work. The update flow only depends on what is attached to a **published GitHub
release**:

1. A signed artifact per platform (`…AppImage`, `…-setup.exe`, `…app.tar.gz`).
2. Each artifact's `.sig` file.
3. A `latest.json` manifest that lists them, with asset URLs whose tag matches the release.

Everything else (checking, downloading, verifying, installing) happens inside the app.

### 13.1 What the GitHub repository needs

Nothing special — no secrets, no Actions, no settings. Just make sure:

- The release is **published** (not a draft). Drafts are invisible to the GitHub API.
- `latest.json` is attached as a release **asset** so its URL is
  `…/releases/download/<tag>/latest.json`.
- The artifact file **names** in `latest.json` match the uploaded asset names exactly.

Your public signing key is already committed in `tauri.conf.json`, so users can verify what
you sign with the private key you keep locally.

### 13.2 The manual release steps

```bash
# 1. Bump version in package.json, src-tauri/tauri.conf.json, src-tauri/Cargo.toml

# 2. Build locally, signing with your private key (this creates the .sig files)
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/gitswamp.key)"
npm run tauri build

# 3. (macOS only) rename GitSwamp.app.tar.gz to ..._x64.app.tar.gz / ..._aarch64.app.tar.gz

# 4. Generate latest.json for the tag you are about to publish
npm run updater:manifest -- --tag v0.5.0 --notes "What's new in 0.5.0"

# 5. Create the release and upload artifacts + .sig + latest.json
gh release create v0.5.0 --title "GitSwamp v0.5.0" --generate-notes \
  "src-tauri/target/release/bundle/appimage/GitSwamp_0.5.0_amd64.AppImage" \
  "src-tauri/target/release/bundle/appimage/GitSwamp_0.5.0_amd64.AppImage.sig" \
  "src-tauri/target/release/bundle/nsis/GitSwamp_0.5.0_x64-setup.exe" \
  "src-tauri/target/release/bundle/nsis/GitSwamp_0.5.0_x64-setup.exe.sig" \
  latest.json

# 6. Verify the manifest is reachable
curl -sL https://github.com/mgrabovac22/gitswamp/releases/download/v0.5.0/latest.json
```

Prefer the browser? Use "Upload with the web UI" in §8 — the crucial part is attaching the
`.sig` files and `latest.json` to the release. For an existing release, add files with:

```bash
gh release upload v0.5.0 latest.json \
  "src-tauri/target/release/bundle/appimage/GitSwamp_0.5.0_amd64.AppImage" \
  "src-tauri/target/release/bundle/appimage/GitSwamp_0.5.0_amd64.AppImage.sig" \
  --clobber
```

### 13.3 Building for multiple platforms

`npm run updater:manifest` scans a single bundle folder, so it only knows about the
artifacts present on the machine where you run it. If you build each OS on its own machine:

- **Easiest:** copy every platform's artifacts (and `.sig` files) into one folder on one
  machine, then run the script once so `latest.json` lists all platforms.
- **Or:** run the script per machine and merge the `platforms` objects by hand into a single
  `latest.json` before uploading.

> A `latest.json` with only `linux-x86_64` means Windows/macOS users get **no** update —
> each platform you care about must have an entry.

### 13.4 Stable vs prerelease (per release)

The **Set as a pre-release** checkbox (or `--prerelease`) decides how it is treated:

- **Prerelease** — the app offers it only when it is newer than the newest stable release.
- **Stable** (checkbox off) — always offered over older releases.

Publishing an update is just: build → generate `latest.json` → publish the release with the
assets. No pipeline required.

> Automating this with CI is **optional** and covered in §14. The workflow file is **not**
> part of this repository — add it yourself only if/when you want automation.

---

## 14. Optional: CI setup (GitHub Actions + Secrets)

Use this only if you later want releases to build and publish automatically on a tag push.
The manual path in §13 needs none of this.

> Repository used throughout: `mgrabovac22/gitswamp`. Adjust if it changes.

### 14.1 Checklist of steps you must take

1. **Create the signing secrets** (once) — §14.2.
2. **Allow the workflow to write releases** (once) — §14.3.
3. **Add the workflow file** `.github/workflows/release.yml` (provided in §14.4).
4. **Push a version tag** (e.g. `v0.5.0`) — §14.6.
5. **Verify** the release contains the artifacts, their `.sig` files, and `latest.json`
   — §14.7.

The signing **public** key is already committed in `tauri.conf.json`; you do **not** need a
secret for it. Only the **private** key goes into secrets.

### 14.2 Create the GitHub secrets

The build runner needs the private key to produce `.sig` files.

Required secret:

| Secret name | Value |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | The **entire contents** of `~/.tauri/gitswamp.key` (not the `.pub`) |

Optional secret — only if your private key is password-protected:

| Secret name | Value |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The key's password |

**Via the web UI:** repository → **Settings** → **Secrets and variables** → **Actions** →
**New repository secret** → add the name(s) above and paste the value.

**Via the GitHub CLI:**

```bash
# from the repo, stores the file content exactly (preserves newlines)
gh secret set TAURI_SIGNING_PRIVATE_KEY --repo mgrabovac22/gitswamp < ~/.tauri/gitswamp.key

# only if the key has a password (you'll be prompted, nothing is echoed)
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo mgrabovac22/gitswamp
```

Notes:

- Do **not** create `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` if the key was generated without a
  password (as `~/.tauri/gitswamp.key` currently is). An empty/mismatched password will make
  the signing step fail.
- For better CI hygiene, regenerate the key **with** a password before shipping any release,
  then update the `pubkey` in `tauri.conf.json` to match (§2.1):
  ```bash
  npx tauri signer generate -w ~/.tauri/gitswamp.key
  # paste ~/.tauri/gitswamp.key.pub into tauri.conf.json, commit, then set both secrets
  ```
- Confirm the secrets exist:
  ```bash
  gh secret list --repo mgrabovac22/gitswamp
  ```

### 14.3 Allow the workflow to publish releases

`GITHUB_TOKEN` needs write access to **Contents**. The workflow declares this itself with
`permissions: contents: write`, but the repository must allow it:

**Settings** → **Actions** → **General** → **Workflow permissions** → select
**Read and write permissions** → **Save**.

(If your org forbids it, create a PAT with `repo` scope and store it as a secret instead.)

### 14.4 The workflow file

Save the following at `.github/workflows/release.yml` (repo root,
**not** inside `software/gitswamp`). It builds every platform, signs the artifacts,
publishes them to a GitHub Release, and uploads/merges `latest.json`.

```yaml
name: Release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

jobs:
  build-and-release:
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: ubuntu-22.04
            args: ""
          - platform: windows-latest
            args: ""
          - platform: macos-latest
            args: "--target aarch64-apple-darwin"
          - platform: macos-latest
            args: "--target x86_64-apple-darwin"

    runs-on: ${{ matrix.platform }}

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Linux dependencies
        if: matrix.platform == 'ubuntu-22.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: 20

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ startsWith(matrix.platform, 'macos') && 'aarch64-apple-darwin,x86_64-apple-darwin' || '' }}

      - name: Cache Rust
        uses: swatinem/rust-cache@v2
        with:
          workspaces: "software/gitswamp/src-tauri -> target"

      - name: Install frontend dependencies
        working-directory: software/gitswamp
        run: npm ci

      - name: Build, sign and publish
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          projectPath: software/gitswamp
          tagName: ${{ github.ref_name }}
          releaseName: "GitSwamp ${{ github.ref_name }}"
          releaseBody: "See the assets below to download and install this version."
          releaseDraft: false
          # tags like v0.5.0-beta.1 (contain a hyphen) become prereleases
          prerelease: ${{ contains(github.ref_name, '-') }}
          includeUpdaterJson: true
          # our updater installs the NSIS .exe on Windows (no `zip` feature)
          updaterJsonPreferNsis: true
          args: ${{ matrix.args }}
```

Why these specifics:

- **`updaterJsonPreferNsis: true`** — the manifest's `windows-x86_64` entry must point at
  the NSIS `-setup.exe`, not the `.msi`, because GitSwamp's updater is built without the
  `zip` feature and runs the installer directly (§5).
- **`includeUpdaterJson: true`** — tauri-action generates `latest.json`, and each matrix job
  merges its platform entry into the existing manifest on the release. If you find the
  manifest only lists one platform, the jobs raced: re-run the failed/older jobs, or merge
  manually with `npm run updater:manifest` and `gh release upload … --clobber`.
- **macOS** builds run twice (arm64 + x64) and tauri-action renames each `.app.tar.gz` to
  include the architecture automatically, avoiding the name collision from §5.
- No extra env is needed for the zip issue — the fix lives in `src-tauri/Cargo.toml`.

### 14.5 Prerequisites in the repository

- `package-lock.json` must be committed (`npm ci` requires it) — it is.
- `src-tauri/Cargo.lock` should be committed so CI resolves the same dependency graph —
  commit it after this feature.
- The version in `tauri.conf.json` must be the version you tag. To automate the bump, keep
  the three files in sync before tagging (§3).

### 14.6 Cut a release with CI

```bash
# 1. bump version in package.json, src-tauri/tauri.conf.json, src-tauri/Cargo.toml
git add -A && git commit -m "Release v0.5.0"
git push

# 2. tag and push -> triggers the workflow
git tag v0.5.0
git push origin v0.5.0
```

For a prerelease just use a hyphenated tag (`v0.5.0-beta.1`); the workflow marks it as a
prerelease and GitSwamp will offer it only if it is chronologically newer than the newest
stable release (§6).

Watch the run:

```bash
gh run list --repo mgrabovac22/gitswamp
gh run watch --repo mgrabovac22/gitswamp
```

### 14.7 Verify the published release

```bash
gh release view v0.5.0 --repo mgrabovac22/gitswamp
```

The release **must** contain, at minimum:

- `GitSwamp_0.5.0_amd64.AppImage` **and** `…AppImage.sig`
- `GitSwamp_0.5.0_x64-setup.exe` **and** `…-setup.exe.sig`
- `GitSwamp_0.5.0_aarch64.app.tar.gz` / `…_x64.app.tar.gz` (+ their `.sig`), if macOS built
- `latest.json`

Then confirm the manifest lists every platform and loads over the public URL:

```bash
curl -sL https://github.com/mgrabovac22/gitswamp/releases/download/v0.5.0/latest.json
```

If a platform is missing from `platforms`, or `latest.json` is absent, the app will not see
that update — re-run the CI job or upload the missing asset.

### 14.8 How to create `latest.json` and the `.sig` files

This is the format reference for the two files the updater needs. It applies whether you
build manually (§13) or via CI (§14).

#### The `.sig` files

When you build with `createUpdaterArtifacts: true` **and** the private key available, Tauri
writes a `<artifact>.sig` file next to each updater artifact. On Linux, the updater
artifact is the `.AppImage.tar.gz` archive, not the standalone `.AppImage` bundle:

| Artifact | Signature file |
| --- | --- |
| `GitSwamp_0.5.0_amd64.AppImage.tar.gz` | `GitSwamp_0.5.0_amd64.AppImage.tar.gz.sig` |
| `GitSwamp_0.5.0_x64-setup.exe` | `GitSwamp_0.5.0_x64-setup.exe.sig` |
| `GitSwamp_0.5.0_aarch64.app.tar.gz` | `GitSwamp_0.5.0_aarch64.app.tar.gz.sig` |

- It is a single line of Base64 text — a minisign signature of the artifact, produced with
  your private key (`~/.tauri/gitswamp.key`) and verified in the app against the `pubkey` in
  `tauri.conf.json`.
- It is **public** and safe to upload; it does **not** contain the private key.
- **Do not edit it.** Rebuilding the artifact changes the signature.
- If a `.sig` file is missing, the build was not signed — set `TAURI_SIGNING_PRIVATE_KEY`
  (and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` if the key is encrypted) and rebuild.

What the contents look like (truncated; it is one long line):

```text
dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hHRhdXJpIHNlY3JldCBrZXkK...
```

You never type this by hand — you copy it into the manifest. With the helper you don't even
do that: the script reads the `.sig` for you.

#### The `latest.json` manifest

The app fetches `latest.json` from the release and picks the entry for its own platform.
Required shape:

```json
{
  "version": "0.5.0",
  "notes": "What's new in 0.5.0",
  "pub_date": "2026-10-16T14:03:43.696Z",
  "platforms": {
    "linux-x86_64": {
      "signature": "dW50cnVzdGVkIGNvbW1lbnQ6...",
      "url": "https://github.com/mgrabovac22/gitswamp/releases/download/v0.5.0/GitSwamp_0.5.0_amd64.AppImage.tar.gz"
    },
    "windows-x86_64": {
      "signature": "dW50cnVzdGVkIGNvbW1lbnQ6...",
      "url": "https://github.com/mgrabovac22/gitswamp/releases/download/v0.5.0/GitSwamp_0.5.0_x64-setup.exe"
    },
    "darwin-x86_64": {
      "signature": "dW50cnVzdGVkIGNvbW1lbnQ6...",
      "url": "https://github.com/mgrabovac22/gitswamp/releases/download/v0.5.0/GitSwamp_0.5.0_x64.app.tar.gz"
    },
    "darwin-aarch64": {
      "signature": "dW50cnVzdGVkIGNvbW1lbnQ6...",
      "url": "https://github.com/mgrabovac22/gitswamp/releases/download/v0.5.0/GitSwamp_0.5.0_aarch64.app.tar.gz"
    }
  }
}
```

Fields:

| Field | Required | Notes |
| --- | --- | --- |
| `version` | yes | Semver. Must be **greater** than the installed app version, otherwise the update is ignored. |
| `notes` | no | Release notes shown in the update panel. |
| `pub_date` | yes | ISO-8601 UTC timestamp (e.g. `new Date().toISOString()`). |
| `platforms` | yes | Object keyed by Tauri platform target; include at least the platform you built. |
| `platforms.<key>.signature` | yes | The **exact contents** of that artifact's `.sig` file (the Base64 string, trimmed). |
| `platforms.<key>.url` | yes | Public direct download URL of the artifact (not the `.sig`). |

Platform keys (Tauri v2):

| Key | Artifact |
| --- | --- |
| `linux-x86_64` | `*.AppImage` |
| `linux-aarch64` | `*.AppImage` (arm64) |
| `windows-x86_64` | `*-setup.exe` (NSIS) or `*.msi` |
| `windows-aarch64` | `*-setup.exe` (arm64) |
| `darwin-x86_64` | `*.app.tar.gz` (Intel) |
| `darwin-aarch64` | `*.app.tar.gz` (Apple Silicon) |

Only the platforms present in `platforms` can update. A macOS-only `latest.json` will not
update Windows/Linux, and vice versa (§13.3).

#### Create it automatically (recommended)

```bash
npm run updater:manifest -- --tag v0.5.0 --notes "What's new in 0.5.0"
```

It scans `src-tauri/target/release/bundle`, reads every `*.sig`, fills in `signature` and
`url`, and writes `latest.json`. Options: `--tag`, `--version`, `--bundle-dir`, `--out`,
`--notes`, `--notes-file`.

#### Create it by hand

1. Build and sign (this generates the `.sig` files).
2. For each artifact, put the `.sig` contents into `signature` and build the asset URL:

```bash
TAG=v0.5.0
ART=src-tauri/target/release/bundle/appimage/GitSwamp_0.5.0_amd64.AppImage
NAME=$(basename "$ART")
SIG=$(tr -d '\n' < "$ART.sig")
printf 'signature=%s\nurl=https://github.com/mgrabovac22/gitswamp/releases/download/%s/%s\n' \
  "$SIG" "$TAG" "$NAME"
```

Copy those two values into the matching platform entry.

Common mistakes that break updates:

- `version` equal to (or lower than) the installed version → the app never offers it.
- Forgetting to bump `src-tauri/tauri.conf.json` too → detection compares against that.
- `url` pointing at the `.sig`, or at the wrong tag → the app gets a 404 when downloading.
- `signature` edited or taken from a different build → signature verification fails.
- Release left as a **draft** → the GitHub API hides it from updater checks.
- Asset filename differing from the URL (e.g. spaces/encoding) → 404. The helper URL-encodes
  names; keep the uploaded name identical.
- A missing platform entry → that OS silently sees no update.
