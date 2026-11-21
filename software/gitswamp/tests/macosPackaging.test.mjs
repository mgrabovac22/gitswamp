import assert from "node:assert/strict";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { verifyMacosApp } from "../scripts/verify-macos.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));

function fixture(t) {
  const directory = mkdtempSync(join(tmpdir(), "gitswamp-macos-test-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const app = join(directory, "GitSwamp.app");
  const contents = join(app, "Contents");
  mkdirSync(join(contents, "MacOS"), { recursive: true });
  mkdirSync(join(contents, "_CodeSignature"));
  const seal = join(contents, "_CodeSignature", "CodeResources");
  writeFileSync(seal, "signed resources");
  writeFileSync(join(contents, "MacOS", "gitswamp"), "test executable");
  chmodSync(join(contents, "MacOS", "gitswamp"), 0o755);
  const info = { CFBundleExecutable: "gitswamp", CFBundleIdentifier: "com.wortex.gitswamp", CFBundleShortVersionString: "0.5.22" };
  const calls = [];
  const run = (command, args) => {
    calls.push({ command, args });
    if (command.endsWith("plutil")) return JSON.stringify(info);
    if (command.endsWith("lipo")) return "arm64\n";
    if (command.endsWith("codesign")) return "";
    throw new Error("Unexpected command: " + command);
  };
  return { app, contents, seal, info, calls, options: { version: "0.5.22", architecture: "arm64", run } };
}

test("macOS signs the assembled bundle even without an Apple certificate", () => {
  const config = JSON.parse(readFileSync(join(root, "src-tauri/tauri.macos.conf.json"), "utf8"));
  assert.equal(config.bundle.macOS.signingIdentity, "-");
});

test("verifies the resource seal, executable, version, architecture and strict signature", t => {
  const f = fixture(t);
  assert.match(verifyMacosApp(f.app, f.options), /^[0-9a-f]{64}$/);
  assert.deepEqual(f.calls.at(-1), { command: "/usr/bin/codesign", args: ["--verify", "--deep", "--strict", "--verbose=2", f.app] });
});

test("rejects the reported missing CodeResources regression before packaging", t => {
  const f = fixture(t);
  rmSync(f.seal);
  assert.throws(() => verifyMacosApp(f.app, f.options), /Missing signed resource seal/);
  assert.equal(f.calls.length, 0);
});

test("rejects the wrong version, architecture and executable path", t => {
  const f = fixture(t);
  assert.throws(() => verifyMacosApp(f.app, { ...f.options, version: "0.5.21" }), /Unexpected app version/);
  assert.throws(() => verifyMacosApp(f.app, { ...f.options, architecture: "x86_64" }), /Wrong executable architecture/);
  f.info.CFBundleExecutable = "../gitswamp";
  assert.throws(() => verifyMacosApp(f.app, f.options), /Invalid CFBundleExecutable/);
  f.info.CFBundleExecutable = "missing";
  assert.throws(() => verifyMacosApp(f.app, f.options), /Missing bundle executable/);
});

test("does not ignore a broken codesign verification", t => {
  const f = fixture(t);
  const run = (command, args, options) => {
    if (command.endsWith("codesign")) throw new Error("code has no resources but signature indicates they must be present");
    return f.options.run(command, args, options);
  };
  assert.throws(() => verifyMacosApp(f.app, { ...f.options, run }), /code has no resources/);
});
