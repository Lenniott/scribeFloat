#!/usr/bin/env node
// Usage: node scripts/bump-version.mjs <new-version>
// Updates version in package.json, src-tauri/Cargo.toml, and src-tauri/tauri.conf.json.
// Does not commit or tag — do that manually after running this script.

import { readFileSync, writeFileSync, renameSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');

const newVersion = process.argv[2];
if (!newVersion || !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(newVersion)) {
  console.error('Usage: node scripts/bump-version.mjs <x.y.z>');
  process.exit(1);
}

function atomicWrite(path, content) {
  const tmp = path + '.tmp';
  writeFileSync(tmp, content, 'utf8');
  renameSync(tmp, path);
}

function bumpPackageJson(version) {
  const path = resolve(root, 'package.json');
  const pkg = JSON.parse(readFileSync(path, 'utf8'));
  const old = pkg.version;
  pkg.version = version;
  atomicWrite(path, JSON.stringify(pkg, null, '\t') + '\n');
  return old;
}

function bumpCargoToml(version) {
  const path = resolve(root, 'src-tauri/Cargo.toml');
  const text = readFileSync(path, 'utf8');
  // Only replace the version under [package], not dependency versions.
  const updated = text.replace(/^(\[package\][^[]*version\s*=\s*)"[^"]*"/m, `$1"${version}"`);
  const old = text.match(/^(\[package\][^[]*version\s*=\s*)"([^"]*)"/m)?.[2] ?? '?';
  atomicWrite(path, updated);
  return old;
}

function bumpTauriConf(version) {
  const path = resolve(root, 'src-tauri/tauri.conf.json');
  const conf = JSON.parse(readFileSync(path, 'utf8'));
  const old = conf.version;
  conf.version = version;
  atomicWrite(path, JSON.stringify(conf, null, 2) + '\n');
  return old;
}

const oldPkg = bumpPackageJson(newVersion);
const oldCargo = bumpCargoToml(newVersion);
const oldConf = bumpTauriConf(newVersion);

console.log(`package.json        ${oldPkg} → ${newVersion}`);
console.log(`Cargo.toml          ${oldCargo} → ${newVersion}`);
console.log(`tauri.conf.json     ${oldConf} → ${newVersion}`);
console.log('');
console.log('Next steps:');
console.log(`  git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json`);
console.log(`  git commit -m "chore: bump version to ${newVersion}"`);
console.log(`  git tag v${newVersion}`);
console.log(`  git push origin main --tags`);
