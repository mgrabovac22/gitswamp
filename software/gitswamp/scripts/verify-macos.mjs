import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

export function verifyMacosApp(app, { version, architecture, run = execFileSync } = {}) {
  const contents = join(app, "Contents");
  const seal = join(contents, "_CodeSignature", "CodeResources");
  if (!existsSync(seal)) throw new Error(`Missing signed resource seal: ${seal}`);
  const info = JSON.parse(run("/usr/bin/plutil", ["-convert", "json", "-o", "-", join(contents, "Info.plist")], { encoding: "utf8" }));
  const executable = info.CFBundleExecutable;
  if (!executable || /[/\\]/.test(executable) || executable === "." || executable === "..") {
    throw new Error(`Invalid CFBundleExecutable in ${app}`);
  }
  const binary = join(contents, "MacOS", executable);
  if (!existsSync(binary) || !statSync(binary).isFile()) throw new Error(`Missing bundle executable: ${binary}`);
  if (process.platform === "darwin" && !(statSync(binary).mode & 0o111)) throw new Error(`Bundle executable lost its execute permission: ${binary}`);
  if (version && info.CFBundleShortVersionString !== version) throw new Error(`Unexpected app version in ${app}`);
  if (info.CFBundleIdentifier !== "com.wortex.gitswamp") throw new Error(`Unexpected app identifier in ${app}`);
  if (architecture) {
    const arches = run("/usr/bin/lipo", ["-archs", binary], { encoding: "utf8" }).trim().split(/\s+/);
    if (!arches.includes(architecture)) throw new Error(`Wrong executable architecture in ${app}: ${arches.join(", ")}`);
  }
  run("/usr/bin/codesign", ["--verify", "--deep", "--strict", "--verbose=2", app], { stdio: "inherit" });
  return createHash("sha256").update(readFileSync(seal)).digest("hex");
}

export function verifyMacosBundles(bundle, options) {
  const original = join(bundle, "macos", "GitSwamp.app");
  const seal = verifyMacosApp(original, options);
  const archive = join(bundle, "macos", "GitSwamp.app.tar.gz");
  if (!existsSync(archive) || !existsSync(`${archive}.sig`)) throw new Error("Missing macOS updater archive or its updater signature.");
  const dmgs = readdirSync(join(bundle, "dmg")).filter(name => name.endsWith(".dmg"));
  if (!dmgs.length) throw new Error("Missing macOS DMG installer.");
  const checkCopy = app => {
    if (verifyMacosApp(app, options) !== seal) throw new Error(`Packaged resource seal differs from the built app: ${app}`);
  };
  const directory = mkdtempSync(join(tmpdir(), "gitswamp-macos-verify-"));
  let mounted = false;
  const mount = join(directory, "dmg");
  try {
    mkdirSync(mount);
    execFileSync("/usr/bin/tar", ["-xzf", archive, "-C", directory], { stdio: "inherit" });
    checkCopy(join(directory, basename(original)));
    for (const name of dmgs) {
      execFileSync("/usr/bin/hdiutil", ["attach", "-readonly", "-nobrowse", "-mountpoint", mount, join(bundle, "dmg", name)], { stdio: "inherit" });
      mounted = true;
      try { checkCopy(join(mount, basename(original))); }
      finally {
        execFileSync("/usr/bin/hdiutil", ["detach", mount], { stdio: "inherit" });
        mounted = false;
      }
    }
  } finally {
    // Never recursively remove a still-mounted disk image if detach failed.
    if (!mounted) rmSync(directory, { recursive: true, force: true });
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  if (process.platform !== "darwin") throw new Error("The macOS bundle verifier must run on macOS.");
  const [bundle, architecture] = process.argv.slice(2);
  if (!bundle || !["x86_64", "arm64"].includes(architecture)) throw new Error("Usage: node scripts/verify-macos.mjs BUNDLE_DIRECTORY x86_64|arm64");
  const { version } = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
  verifyMacosBundles(resolve(bundle), { version, architecture });
  console.log("Verified macOS app, updater archive and DMG: resource seals, signatures, version and architecture.");
}
