import assert from "node:assert/strict";
import { readFileSync, mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import test from "node:test";
import ts from "typescript";
import { linuxMetadata } from "../scripts/prepare-linux-package.mjs";

const require = createRequire(import.meta.url);
const root = fileURLToPath(new URL("..", import.meta.url));
function updater(respond) {
  const calls=[],opened=[],toasts=[];
  let restarts=0;
  const mocks={
    "@tauri-apps/api/core": {invoke:async(command,args)=>{calls.push({command,args});return respond(command,args);},Channel:class{}},
    "@tauri-apps/plugin-process":{relaunch:async()=>{restarts++;}},
    "@tauri-apps/plugin-opener":{openPath:async path=>opened.push(path),openUrl:async()=>{}},
    "@/shared/notifications/useToast":{useToast:()=>({action:(...args)=>toasts.push(args),error:()=>{}})},
    "@/app/storage/safeStorage":{safeStorageGet:()=>null,safeStorageSet:()=>{}},
  };
  const source=readFileSync(join(root,"src/features/updater/useAppUpdate.ts"),"utf8");
  const code=ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
  const module={exports:{}};
  new Function("require","module","exports",code)(id=>mocks[id]??require(id),module,module.exports);
  return {api:module.exports.useAppUpdate(),calls,opened,toasts,restarts:()=>restarts};
}
const info={debInstallation:true,alreadyInstalled:false,tag:"v0.4.123",version:"0.4.123",currentVersion:"0.4.122",notes:"Fixes",htmlUrl:"https://github.com/mgrabovac22/gitswamp/releases/tag/v0.4.123"};

test("installed Debian update offers restart rather than another install",async()=>{
  const u=updater(()=>({...info,alreadyInstalled:true}));await u.api.check(true);
  assert.equal(u.api.status.value,"installed");assert.equal(u.api.isVisible.value,true);
  await u.api.restart();assert.equal(u.restarts(),1);assert.equal(u.calls.length,1);
});
test("Debian downloads the exact offered release and never invokes AppImage install",async()=>{
  const u=updater(command=>command==="check_for_update"?info:{path:"/Downloads/update.deb",version:info.version,alreadyInstalled:false});
  await u.api.check(true);await u.api.install();
  assert.equal(u.api.status.value,"downloaded");assert.equal(u.api.isVisible.value,true);
  assert.deepEqual(u.calls.map(call=>call.command),["check_for_update","download_deb_update"]);
  assert.deepEqual(u.calls[1].args,{tag:info.tag,version:info.version});
  assert.equal(u.restarts(),0);await u.api.openDownloadedPackage();assert.deepEqual(u.opened,["/Downloads/update.deb"]);
});
test("install detects an update installed since the original check",async()=>{
  const u=updater(command=>command==="check_for_update"?info:{path:null,version:info.version,alreadyInstalled:true});
  await u.api.check(true);await u.api.install();assert.equal(u.api.status.value,"installed");assert.deepEqual(u.opened,[]);
});
test("non-Debian native installation and relaunch are unchanged",async()=>{
  const u=updater(command=>command==="check_for_update"?{...info,debInstallation:false}:undefined);
  await u.api.check(true);await u.api.install();assert.equal(u.restarts(),1);
  assert.deepEqual(u.calls.map(call=>call.command),["check_for_update","install_update"]);
});
test("download errors remain retryable, and empty checks clear stale update info",async()=>{
  let empty=false;
  const u=updater(command=>{if(command==="check_for_update")return empty?null:info;throw Error("Wrong package version");});
  await u.api.check(true);await u.api.install();assert.equal(u.api.status.value,"available");assert.match(u.api.errorMessage.value,/Wrong package/);
  empty=true;await u.api.check(true);assert.equal(u.api.info.value,null);assert.equal(u.api.isVisible.value,false);
});
test("Linux metadata uses one version and standard update text",()=>{
  const result=linuxMetadata("0.4.123",new Date("2026-10-31T00:00:00Z"));
  assert.match(result.description,/0\.4\.123/);assert.match(result.description,/GitSwamp update with fixes and improvements/);
  assert.match(result.appstream,/GitSwamp.desktop/);
  assert.match(result.changelog,/0\.4\.123/);assert.match(result.notes,/restart|reopen/);
  assert.throws(()=>linuxMetadata("invalid"));
});
test("version preparation is repeatable and keeps package, lock, Cargo and metadata aligned",()=>{
  const directory=mkdtempSync(join(tmpdir(),"gitswamp-release-test-"));
  try {
    mkdirSync(join(directory,"src-tauri/packaging"),{recursive:true});
    writeFileSync(join(directory,"package.json"),JSON.stringify({version:"0.4.0"}));
    writeFileSync(join(directory,"package-lock.json"),JSON.stringify({version:"0.4.0",packages:{"":{version:"0.4.0"}}}));
    writeFileSync(join(directory,"src-tauri/tauri.conf.json"),JSON.stringify({version:"0.4.0",bundle:{}}));
    writeFileSync(join(directory,"src-tauri/Cargo.toml"),'[package]\nname = "gitswamp"\nversion = "0.4.0"\n');
    for(let i=0;i<2;i++)execFileSync(process.execPath,[resolve(root,"scripts/set-release-version.mjs"),"0.4.123"],{cwd:directory});
    for(const file of ["package.json","package-lock.json","src-tauri/tauri.conf.json"])assert.equal(JSON.parse(readFileSync(join(directory,file))).version,"0.4.123");
    assert.equal(JSON.parse(readFileSync(join(directory,"package-lock.json"))).packages[""].version,"0.4.123");
    assert.match(readFileSync(join(directory,"src-tauri/Cargo.toml"),"utf8"),/version = "0.4.123"/);
    assert.match(readFileSync(join(directory,"src-tauri/packaging/RELEASE-NOTES.txt"),"utf8"),/GitSwamp update with fixes and improvements/);
  }finally{rmSync(directory,{recursive:true,force:true});}
});
