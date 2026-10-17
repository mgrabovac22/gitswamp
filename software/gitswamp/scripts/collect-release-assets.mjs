import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  rmSync,
  statSync,
} from "node:fs";
import { basename, join } from "node:path";

const [sourceDir = "release-artifacts", outputDir = "release-files"] = process.argv.slice(2);

function walk(dir) {
  const entries = readdirSync(dir, { withFileTypes: true });
  return entries.flatMap((entry) => {
    const path = join(dir, entry.name);
    return entry.isDirectory() ? walk(path) : [path];
  });
}

function updaterPlatform(file) {
  const lower = file.toLowerCase();
  const isArm = /aarch64|arm64/.test(lower);

  if (lower.endsWith(".appimage.tar.gz")) return isArm ? "linux-aarch64" : "linux-x86_64";
  if (lower.endsWith(".app.tar.gz")) return isArm ? "darwin-aarch64" : "darwin-x86_64";
  if (lower.endsWith("-setup.exe")) return isArm ? "windows-aarch64" : "windows-x86_64";
  return null;
}

function releaseAssetName(file, platform) {
  const name = basename(file);
  if (name.toLowerCase().endsWith(".app.tar.gz")) {
    return `${name.slice(0, -".app.tar.gz".length)}_${platform}.app.tar.gz`;
  }
  if (name.toLowerCase().endsWith(".appimage.tar.gz")) {
    return `${name.slice(0, -".AppImage.tar.gz".length)}_${platform}.AppImage.tar.gz`;
  }
  if (name.toLowerCase().endsWith("-setup.exe")) {
    return `${name.slice(0, -"-setup.exe".length)}_${platform}-setup.exe`;
  }
  throw new Error(`Unsupported updater artifact: ${file}`);
}

if (!existsSync(sourceDir) || !statSync(sourceDir).isDirectory()) {
  throw new Error(`Artifact directory does not exist: ${sourceDir}`);
}

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(outputDir, { recursive: true });

const seenPlatforms = new Set();
const seenInstallers = new Set();
const copied = [];

function copyInstaller(file) {
  const installerName = basename(file);
  if (seenInstallers.has(installerName)) return;
  copyFileSync(file, join(outputDir, installerName));
  const signature = `${file}.sig`;
  if (existsSync(signature)) {
    copyFileSync(signature, join(outputDir, `${installerName}.sig`));
  }
  seenInstallers.add(installerName);
}

for (const file of walk(sourceDir).sort()) {
  if (file.endsWith(".sig")) continue;

  const lower = file.toLowerCase();
  if (lower.endsWith(".appimage") && !lower.endsWith(".appimage.tar.gz")) {
    copyInstaller(file);
    continue;
  }

  if (lower.endsWith(".dmg")) {
    copyInstaller(file);
    continue;
  }

  if (lower.endsWith(".deb")) {
    copyInstaller(file);
    continue;
  }

  const platform = updaterPlatform(file);
  if (!platform || seenPlatforms.has(platform)) continue;

  const signature = `${file}.sig`;
  if (!existsSync(signature)) {
    throw new Error(`Missing updater signature for ${file}`);
  }

  const outputName = releaseAssetName(file, platform);
  copyFileSync(file, join(outputDir, outputName));
  copyFileSync(signature, join(outputDir, `${outputName}.sig`));
  seenPlatforms.add(platform);
  copied.push(`${platform}: ${outputName}`);
}

if (copied.length === 0) {
  throw new Error("No signed updater artifacts were found.");
}

console.log("Prepared release assets:");
for (const item of copied) console.log(`  ${item}`);
