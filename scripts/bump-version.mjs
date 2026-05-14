#!/usr/bin/env node
// Usage:
//   node scripts/bump-version.mjs <x.y.z>              # bump files + git add/commit/tag (no push)
//   node scripts/bump-version.mjs <x.y.z> --push       # also: git push origin HEAD && git push origin v<x.y.z>
//   node scripts/bump-version.mjs <x.y.z> --no-git     # only bump files (legacy)
//
// npm: npm run bump -- 0.1.7
//      npm run bump -- 0.1.7 --push

import { readFileSync, writeFileSync, renameSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { spawnSync } from 'child_process';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');

const raw = process.argv.slice(2);
const flags = new Set(raw.filter((a) => a === '--no-git' || a === '--push'));
const positional = raw.filter((a) => !a.startsWith('--'));
const newVersion = positional.find((a) =>
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(a),
);

if (!newVersion) {
  console.error('Usage: npm run bump -- <x.y.z> [--push] [--no-git]');
  process.exit(1);
}

const doGit = !flags.has('--no-git');
const doPush = flags.has('--push');

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

function gitOk() {
  return spawnSync('git', ['rev-parse', '--git-dir'], { cwd: root, encoding: 'utf8' }).status === 0;
}

function runGit(gitArgs) {
  const r = spawnSync('git', gitArgs, { cwd: root, stdio: 'inherit' });
  if (r.status !== 0) {
    console.error(`\nerror: git ${gitArgs.join(' ')} exited ${r.status ?? 'unknown'}`);
    process.exit(r.status ?? 1);
  }
}

const oldPkg = bumpPackageJson(newVersion);
const oldCargo = bumpCargoToml(newVersion);
const oldConf = bumpTauriConf(newVersion);

console.log(`package.json        ${oldPkg} → ${newVersion}`);
console.log(`Cargo.toml          ${oldCargo} → ${newVersion}`);
console.log(`tauri.conf.json     ${oldConf} → ${newVersion}`);

if (doGit) {
  if (!gitOk()) {
    console.error('\nwarning: not a git repo — skipped git add / commit / tag');
  } else {
    const paths = ['package.json', 'src-tauri/Cargo.toml', 'src-tauri/tauri.conf.json'];
    const tag = `v${newVersion}`;
    const tagCheck = spawnSync('git', ['rev-parse', tag], { cwd: root, encoding: 'utf8' });
    if (tagCheck.status === 0) {
      console.error(`\nerror: tag ${tag} already exists. Delete it or pick another version.`);
      process.exit(1);
    }
    runGit(['add', ...paths]);
    const staged = spawnSync('git', ['diff', '--cached', '--quiet'], { cwd: root });
    if (staged.status === 0) {
      console.log('\ngit: nothing to commit (files already at this version?) — skipped commit/tag/push');
    } else {
      runGit([
        'commit',
        '-m',
        `chore: bump version to ${newVersion}`,
        '--',
        ...paths,
      ]);
      runGit(['tag', tag]);
      console.log(`\ngit: committed and tagged ${tag}`);
      if (doPush) {
        runGit(['push', 'origin', 'HEAD']);
        runGit(['push', 'origin', tag]);
        console.log('git: pushed HEAD and tag to origin');
      } else {
        console.log('\nNext: push when ready:');
        console.log(`  git push origin HEAD && git push origin ${tag}`);
        console.log('\nOr next time: npm run bump -- <x.y.z> --push');
      }
    }
  }
} else {
  console.log('\nNext steps (--no-git):');
  console.log(`  git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json`);
  console.log(`  git commit -m "chore: bump version to ${newVersion}"`);
  console.log(`  git tag v${newVersion}`);
  console.log(`  git push origin HEAD && git push origin v${newVersion}`);
}
