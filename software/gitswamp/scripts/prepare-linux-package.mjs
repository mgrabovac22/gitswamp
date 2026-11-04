import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const escapeXml = value => String(value).replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&apos;");

export function linuxMetadata(version, date = new Date()) {
  if (!/^\d+\.\d+\.\d+(?:-[\w.-]+)?(?:\+[\w.-]+)?$/.test(version)) throw Error("Invalid package version.");
  const description = `GitSwamp ${version}: visual Git history, staging, file editing, conflict resolution, Time Machine and repository analytics.`;
  const guidance = "Install this package over an older GitSwamp installation, then close and reopen GitSwamp. If the same version is already installed, no reinstall is needed. Repositories and user settings are not removed by a normal upgrade.";
  const summary = "GitSwamp update with fixes and improvements.";
  return {
    description: `${description}\n\n${summary}\n\n${guidance}`,
    notes: `GitSwamp ${version}\n\n${summary}\n\n${guidance}\n`,
    changelog: `GitSwamp ${version}\nReleased: ${date.toISOString()}\n\n${summary}\n`,
    appstream: `<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>com.wortex.gitswamp</id>
  <metadata_license>CC0-1.0</metadata_license>
  <project_license>AGPL-3.0-only</project_license>
  <name>GitSwamp</name>
  <summary>Visual Git client with conflict resolution and repository analytics</summary>
  <description><p>${escapeXml(description)}</p><p>${escapeXml(guidance)}</p></description>
  <launchable type="desktop-id">GitSwamp.desktop</launchable>
  <provides><binary>gitswamp</binary></provides>
  <categories><category>Development</category><category>RevisionControl</category></categories>
  <url type="homepage">https://mgrabovac22.github.io/gitswamp/</url>
  <url type="bugtracker">https://github.com/mgrabovac22/gitswamp/issues</url>
  <releases><release version="${escapeXml(version)}" date="${date.toISOString().slice(0,10)}"><description><p>${summary}</p></description></release></releases>
</component>
`,
  };
}

export function prepareLinuxPackage(root = process.cwd(), date = new Date()) {
  const configPath = resolve(root, "src-tauri/tauri.conf.json");
  const config = JSON.parse(readFileSync(configPath, "utf8"));
  const directory = resolve(root, "src-tauri/packaging");
  const metadata = linuxMetadata(config.version, date);
  mkdirSync(directory, { recursive: true });
  writeFileSync(resolve(directory, "changelog"), metadata.changelog);
  writeFileSync(resolve(directory, "RELEASE-NOTES.txt"), metadata.notes);
  writeFileSync(resolve(directory, "com.wortex.gitswamp.metainfo.xml"), metadata.appstream);
  config.bundle.longDescription = metadata.description;
  writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) prepareLinuxPackage();
