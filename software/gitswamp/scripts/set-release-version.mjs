import { readFileSync, writeFileSync } from "node:fs";

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

const cargoPath = "src-tauri/Cargo.toml";
const cargo = readFileSync(cargoPath, "utf8");
const updatedCargo = cargo.replace(
  /^(\[package\][\s\S]*?^version\s*=\s*)"[^"]+"/m,
  `$1"${version}"`,
);

if (updatedCargo === cargo) {
  throw new Error("Could not update the package version in src-tauri/Cargo.toml.");
}

writeFileSync(cargoPath, updatedCargo, "utf8");
console.log(`Prepared release version ${version}.`);
