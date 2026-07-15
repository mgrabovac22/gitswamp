# GitSwamp Mobile

GitSwamp Mobile is the native Android companion for GitSwamp. It keeps the core Git workflow on the device: open or clone a repository, inspect its commit graph and diffs, manage working changes, commit, switch branches, and synchronize with a remote.

The application is built with Kotlin, Jetpack Compose, and JGit. It has no WebView, JavaScript runtime, application server, telemetry, or cloud database. HTTPS clone, fetch, pull, and push connect directly to the remote selected by the user; every other operation is local.

## Install on a phone

The ready-to-install local APK is generated at:

```text
dist/GitSwamp-Mobile-local.apk
```

Android 8.0 (API 26) or newer is supported.

### USB installation

1. Enable Developer options and USB debugging on the phone.
2. Connect it over USB and accept the authorization prompt.
3. From this directory run:

```powershell
.\scripts\install-local-apk.ps1
```

To install an APK that is already built:

```powershell
.\scripts\install-local-apk.ps1 -SkipBuild
```

### Installation without ADB

Transfer `dist/GitSwamp-Mobile-local.apk` to the phone, open it in the Files application, and allow installation from that source when Android asks. No server is required after installation.

The `local` build uses the install id `com.gitswamp.mobile.local`, so it can be installed alongside older GitSwamp Mobile builds even when those builds use a different signing key. It is minified and resource-shrunk like a release build, then signed by the build script with a persistent local release certificate stored outside Git. A public release must use the production id and a private production key.

The script also creates `dist/GitSwamp-Mobile-debug.apk` with the standard Android Studio debug certificate and package `com.gitswamp.mobile.debug`. Use that variant with `adb install` during development; the minified local release APK is the normal sideload candidate.

## Core workflow

- **Start screen:** clone an HTTPS repository, browse a device folder, initialize a repository, search recent repositories, or reopen one instantly.
- **Commit graph:** Canvas-rendered branch lanes, commit refs, message, author, relative time, SHA, commit search, selectable rows, and bounded incremental loading.
- **Commit details:** metadata, parent SHAs, changed-file list, and file-specific unified diff.
- **Working changes:** separate staged, unstaged, and conflict groups with stage, unstage, per-file diff, and commit actions.
- **Safe discard:** only unstaged content is discarded. The operation waits five seconds and exposes Undo before touching disk, so staged changes remain intact.
- **Files:** lazy directory navigation and a bounded text preview; large and binary files are not loaded into memory as text.
- **Branches:** local and remote branch list, checkout, and local branch creation.
- **Remotes:** fetch with remote-ref pruning, pull, and push over HTTPS with optional username/PAT credentials.
- **Identity:** repository-local author name and email setup when a commit has no configured identity.
- **Adaptive UI:** bottom navigation and full-screen detail sheets on phones, navigation rail and side details on wider devices.

## Local repository access

Android's Storage Access Framework is used for Browse. When a document provider exposes a usable filesystem path, GitSwamp opens it directly. Otherwise, the repository is streamed into the app's private workspace with visible progress. This fallback avoids retaining an entire repository in RAM and keeps access valid under Android scoped-storage rules.

Cloned and initialized repositories live in private application storage. Uninstalling the application can remove that private data, so push important commits or export the repository first.

## Architecture

```text
app/src/main/java/com/gitswamp/mobile/
|-- core/
|   |-- model/          Immutable UI/domain models
|   |-- git/            GitRepository contract, JGit adapter, graph layout
|   |-- storage/        SAF resolver, recent repos, preferences, token vault
|   `-- designsystem/   GitSwamp colors and reusable Compose controls
|-- feature/
|   |-- home/           Start workflow and clone/import progress
|   |-- repository/     Session state, cancellation, and repository shell
|   |-- settings/       Small mobile-only preferences
|   `-- repository/components/
|       Graph, diff, changes, files, branches, credentials, and identity
|-- di/AppContainer.kt  Process-scoped dependency wiring
`-- GitSwampApp.kt      Adaptive navigation root
```

`GitRepository` is the boundary between UI orchestration and JGit. Compose components never issue Git commands directly. `RepositoryViewModel` owns one cancellable repository session, and changing repositories cancels stale graph, diff, and tree work before the next session starts.

## Performance and memory policy

- Initial history is limited to 100, 200, or 500 commits according to the preference; Load more is capped at 5,000 visible commits.
- The graph uses one Canvas for lanes and lazy rows for commit content instead of one object hierarchy per line segment.
- Commit details, diffs, file trees, and previews load on demand and only the current selection is retained.
- Diff and file reads have hard byte limits and stream into fixed buffers.
- SAF imports stream file-by-file rather than buffering a repository.
- Repository coroutines are canceled when the active repository changes or the screen is disposed.
- Release/local builds enable R8 minification and Android resource shrinking.

## Security

- Personal access tokens are encrypted with a non-exportable Android Keystore key before being stored.
- Tokens are keyed by normalized remote host and are never written into repository config.
- App backup and device-transfer extraction are disabled for private repository and credential data.
- Repository paths are normalized and constrained before tree reads or file previews.
- HTTPS certificate verification is mandatory. Repositories that disable `http.sslVerify` are rejected before network access.
- The application requests only the Internet permission. Folder access is granted explicitly through Android's system picker.

## Build and verify

The repository includes a Gradle wrapper. Set `sdk.dir` in `local.properties` or open the project in Android Studio, then run:

```powershell
.\gradlew.bat lintDebug testDebugUnitTest assembleDebug
.\scripts\build-local-apk.ps1
```

The test suite covers graph-lane stability and JGit integration, including clone/diff behavior, path traversal rejection, unstage of a new file, the staged-versus-unstaged discard contract, insecure remote rejection, mandatory TLS verification, local push, and pruning of deleted remote branches.

## Deliberate mobile scope

This companion focuses on the everyday Git client workflow. Desktop-only tools such as Terminal, Galaxy, Repository City, Time Machine, Productivity Arena, Burnout Analytics, and provider dashboards are intentionally excluded so the mobile process stays predictable and lightweight. SSH remotes are not enabled in this first version; use HTTPS and a provider token for authenticated remote operations.
