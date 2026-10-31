import { readFileSync, writeFileSync } from "node:fs";
import { prepareLinuxPackage } from "./prepare-linux-package.mjs";

const version = process.argv[2]?.trim();

if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version || "")) {
  throw new Error("Usage: node scripts/set-release-version.mjs <semver-version>");
}

function updateJson(path) {
  const value = JSON.parse(readFileSync(path, "utf8"));
  value.version = version;
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

updateJson("package.json");
updateJson("src-tauri/tauri.conf.json");
const lock = JSON.parse(readFileSync("package-lock.json", "utf8"));
lock.version = version;
if (lock.packages?.[""]) lock.packages[""].version = version;
writeFileSync("package-lock.json", `${JSON.stringify(lock, null, 2)}\n`);

const cargoPath = "src-tauri/Cargo.toml";
const cargo = readFileSync(cargoPath, "utf8");
const packageVersion = /^(\[package\][\s\S]*?^version\s*=\s*)"[^"]+"/m;
if (!packageVersion.test(cargo)) throw new Error("No Cargo package version found.");
const updatedCargo = cargo.replace(
  packageVersion,
  `$1"${version}"`,
);

writeFileSync(cargoPath, updatedCargo, "utf8");
prepareLinuxPackage();
console.log(`Prepared release version ${version}.`);
