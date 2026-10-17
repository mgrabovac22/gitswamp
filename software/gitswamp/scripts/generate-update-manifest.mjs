import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join, basename, extname } from "node:path";

const REPO = "mgrabovac22/gitswamp";
const OWNER_REPO = process.env.GITSWAMP_REPO || REPO;

function parseArgs(argv) {
  const args = {};
  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i];
    if (!token.startsWith("--")) continue;
    const key = token.slice(2);
    const next = argv[i + 1];
    if (next && !next.startsWith("--")) {
      args[key] = next;
      i += 1;
    } else {
      args[key] = true;
    }
  }
  return args;
}

function readPackageVersion() {
  const conf = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
  return conf.version;
}

function walk(dir) {
  const out = [];
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return out;
  }
  for (const entry of entries) {
    const full = join(dir, entry);
    const info = statSync(full);
    if (info.isDirectory()) out.push(...walk(full));
    else out.push(full);
  }
  return out;
}

function platformFor(file) {
  const name = basename(file);
  const lower = name.toLowerCase();
  const full = file.toLowerCase();

  const isArm =
    /aarch64|arm64/.test(lower) || /aarch64-apple-darwin/.test(full);
  const isUniversal = /universal/.test(lower) || /universal-apple-darwin/.test(full);

  if (lower.endsWith(".appimage.tar.gz")) {
    return isArm ? "linux-aarch64" : "linux-x86_64";
  }
  if (lower.endsWith(".app.tar.gz")) {
    // A universal macOS bundle cannot be expressed as a single platform key;
    // build and publish one tarball per architecture instead.
    if (isUniversal) return null;
    return isArm ? "darwin-aarch64" : "darwin-x86_64";
  }
  if (lower.endsWith("-setup.exe")) {
    return isArm ? "windows-aarch64" : "windows-x86_64";
  }
  return null;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const configVersion = readPackageVersion();
  const tag = args.tag || `v${configVersion}`;
  const version = args.version || tag.replace(/^v/, "");
  const bundleDir = args["bundle-dir"] || "src-tauri/target/release/bundle";
  const out = args.out || "latest.json";

  if (version !== configVersion) {
    console.warn(
      `Warning: manifest version "${version}" does not match src-tauri/tauri.conf.json "${configVersion}". ` +
        "Bump the app version so the running app can detect the update.",
    );
  }

  const notes = args["notes-file"]
    ? readFileSync(args["notes-file"], "utf8").trim()
    : typeof args.notes === "string"
      ? args.notes
      : "";

  const files = walk(bundleDir);
  const platforms = {};
  const used = new Set();

  for (const file of files) {
    if (!file.endsWith(".sig")) continue;
    const artifact = file.slice(0, -4);
    const platform = platformFor(artifact);
    if (!platform) continue;
    const signature = readFileSync(file, "utf8").trim();
    const assetName = basename(artifact);
    platforms[platform] = {
      signature,
      url: `https://github.com/${OWNER_REPO}/releases/download/${tag}/${encodeURIComponent(assetName)}`,
    };
    used.add(assetName);
  }

  if (Object.keys(platforms).length === 0) {
    console.error(
      `No signed updater artifacts found in ${bundleDir}. ` +
        "Build with `createUpdaterArtifacts: true` and TAURI_SIGNING_PRIVATE_KEY set.",
    );
    process.exit(1);
  }

  const manifest = {
    version,
    notes,
    pub_date: new Date().toISOString(),
    platforms,
  };

  writeFileSync(out, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  console.log(`Wrote ${out} for ${tag} (${Object.keys(platforms).join(", ")})`);
  for (const asset of used) console.log(`  asset: ${asset}`);
}

main();
