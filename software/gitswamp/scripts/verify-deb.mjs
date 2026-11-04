import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const file = process.argv[2];
if (!file) throw Error("Usage: node scripts/verify-deb.mjs <package.deb>");
const expected = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8")).version;
const fields = execFileSync("dpkg-deb", ["--show", "--showformat=${Package}\n${Version}\n${Architecture}\n", file], {encoding:"utf8"}).trim().split("\n");
if (fields[0] !== "git-swamp" || fields[1] !== expected || fields[2] !== "amd64") throw Error(`Unexpected Debian metadata: ${fields.join(", ")}; expected git-swamp ${expected} amd64.`);
const description = execFileSync("dpkg-deb", ["--field", file, "Description"], {encoding:"utf8"});
if (!description.includes(expected) || !description.includes("GitSwamp update with fixes and improvements.")) throw Error("Debian description is missing the release version or update information.");
const contents = execFileSync("dpkg-deb", ["--contents", file], {encoding:"utf8"});
for (const name of ["com.wortex.gitswamp.metainfo.xml", "RELEASE-NOTES.txt", "USER-GUIDE.md", "changelog.gz"]) {
  if (!contents.includes(name)) throw Error(`Missing Debian metadata file: ${name}`);
}
console.log(`Verified ${fields.join(" ")} with release notes and user guide.`);
