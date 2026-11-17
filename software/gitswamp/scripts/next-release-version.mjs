import { execFileSync } from "node:child_process";
import { appendFileSync, readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const stable = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const parse = value => {
  const match = stable.exec(value);
  if (!match) return null;
  const numbers = match.slice(1).map(Number);
  return numbers.every(Number.isSafeInteger) ? numbers : null;
};
const compare = (a, b) => a[0] - b[0] || a[1] - b[1] || a[2] - b[2];
const parseTag = tag => tag.startsWith("v") ? parse(tag.slice(1)) : null;

export function nextReleaseVersion(configured, tags, headTags = []) {
  const base = parse(configured);
  if (!base) throw new Error(`Expected a stable semver version, received ${configured}.`);
  const latest = tags.map(parseTag).filter(Boolean).sort(compare).at(-1);
  const sameCommit = headTags.map(parseTag).filter(Boolean).sort(compare).at(-1);
  if (sameCommit && sameCommit[0] === base[0] && sameCommit[1] === base[1] && compare(sameCommit, base) >= 0 && (!latest || compare(sameCommit, latest) === 0)) {
    return sameCommit.join(".");
  }
  if (!latest || base[0] > latest[0] || (base[0] === latest[0] && base[1] > latest[1])) return base.join(".");
  if (base[0] < latest[0] || base[1] < latest[1]) {
    throw new Error(`The configured version ${configured} is behind the published release line ${latest[0]}.${latest[1]}. Update tauri.conf.json.`);
  }
  if (sameCommit && compare(sameCommit, latest) < 0) throw new Error("This commit already has an older release tag. Do not re-release old history.");
  const patch = Math.max(base[2], latest[2] + 1);
  if (!Number.isSafeInteger(patch)) throw new Error("Release patch number is out of range.");
  return `${base[0]}.${base[1]}.${patch}`;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const tags = (...args) => execFileSync("git", ["tag", "--list", ...args], { encoding: "utf8" }).split(/\r?\n/);
  const { version: configured } = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
  const version = nextReleaseVersion(configured, tags(), tags("--points-at", "HEAD"));
  appendFileSync(process.env.GITHUB_OUTPUT, `version=${version}\ntag=v${version}\n`);
  console.log(`Release version: ${version}`);
}
