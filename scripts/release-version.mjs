#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { readFileSync, renameSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|[a-zA-Z-][0-9a-zA-Z-]*))*))?$/;
const ARCHMAGE_PACKAGES = new Set(['archmage-core', 'archmage-cli', 'archmage-desktop']);

export function parseVersion(value) {
  const version = String(value ?? '').trim();
  if (!SEMVER.test(version)) {
    throw new Error(`invalid semantic version: ${value}`);
  }
  return version;
}

function paths(root) {
  return {
    cargo: join(root, 'Cargo.toml'),
    cargoLock: join(root, 'Cargo.lock'),
    npm: join(root, 'apps', 'archmage-desktop', 'package.json'),
    npmLock: join(root, 'apps', 'archmage-desktop', 'package-lock.json'),
    tauri: join(root, 'apps', 'archmage-desktop', 'src-tauri', 'tauri.conf.json'),
  };
}

function workspaceVersion(text) {
  const section = text.match(/\[workspace\.package\]([\s\S]*?)(?=\n\[|$)/);
  const version = section?.[1].match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];
  if (!version) throw new Error('workspace package version is missing');
  return version;
}

function cargoLockVersions(text) {
  const versions = [];
  for (const block of text.split(/(?=\[\[package\]\])/)) {
    const name = block.match(/^name\s*=\s*"([^"]+)"\s*$/m)?.[1];
    if (!name || !ARCHMAGE_PACKAGES.has(name)) continue;
    const version = block.match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];
    if (!version) throw new Error(`Cargo.lock package ${name} has no version`);
    versions.push([name, version]);
  }
  if (versions.length === 0) throw new Error('Cargo.lock has no Archmage packages');
  return versions;
}

export function readVersions(root) {
  const files = paths(resolve(root));
  const npm = JSON.parse(readFileSync(files.npm, 'utf8'));
  const npmLock = JSON.parse(readFileSync(files.npmLock, 'utf8'));
  const tauri = JSON.parse(readFileSync(files.tauri, 'utf8'));
  if (npmLock.version !== npmLock.packages?.['']?.version) {
    throw new Error('package-lock root versions disagree');
  }
  return {
    workspace: workspaceVersion(readFileSync(files.cargo, 'utf8')),
    npm: npm.version,
    npmLock: npmLock.version,
    tauri: tauri.version,
  };
}

export function checkVersions(root, tag) {
  const absolute = resolve(root);
  const versions = readVersions(absolute);
  const unique = new Set(Object.values(versions));
  if (unique.size !== 1) {
    throw new Error(`version mismatch: ${JSON.stringify(versions)}`);
  }
  const version = parseVersion(versions.workspace);
  for (const [name, locked] of cargoLockVersions(readFileSync(paths(absolute).cargoLock, 'utf8'))) {
    if (locked !== version) {
      throw new Error(`version mismatch: Cargo.lock ${name} is ${locked}, expected ${version}`);
    }
  }
  if (tag !== undefined) {
    const expected = `v${version}`;
    if (tag !== expected) throw new Error(`tag ${tag} does not match ${expected}`);
  }
  return version;
}

function atomicJson(path, value) {
  const temporary = `${path}.release-version-${process.pid}`;
  writeFileSync(temporary, `${JSON.stringify(value, null, 2)}\n`, { mode: statSync(path).mode & 0o777 });
  renameSync(temporary, path);
}

export function setVersion(root, requested) {
  const version = parseVersion(requested);
  const absolute = resolve(root);
  const files = paths(absolute);
  const cargo = readFileSync(files.cargo, 'utf8');
  const sectionPattern = /(\[workspace\.package\][\s\S]*?^version\s*=\s*")[^"]+("\s*$)/m;
  if (!sectionPattern.test(cargo)) throw new Error('workspace package version is missing');
  const npm = JSON.parse(readFileSync(files.npm, 'utf8'));
  const npmLock = JSON.parse(readFileSync(files.npmLock, 'utf8'));
  const tauri = JSON.parse(readFileSync(files.tauri, 'utf8'));
  if (!npmLock.packages?.['']) throw new Error('package-lock root package is missing');

  writeFileSync(files.cargo, cargo.replace(sectionPattern, `$1${version}$2`));
  npm.version = version;
  npmLock.version = version;
  npmLock.packages[''].version = version;
  tauri.version = version;
  atomicJson(files.npm, npm);
  atomicJson(files.npmLock, npmLock);
  atomicJson(files.tauri, tauri);
  return version;
}

function git(root, args, options = {}) {
  try {
    return execFileSync('git', args, {
      cwd: root,
      encoding: 'utf8',
      stdio: options.quiet ? ['ignore', 'pipe', 'pipe'] : ['ignore', 'pipe', 'inherit'],
    }).trim();
  } catch (error) {
    if (options.allowFailure) return null;
    throw error;
  }
}

export function createReleaseTag(root, requested) {
  const absolute = resolve(root);
  const version = parseVersion(requested);
  const tag = `v${version}`;
  if (git(absolute, ['status', '--porcelain'], { quiet: true }) !== '') {
    throw new Error('release tagging requires a clean working tree');
  }
  if (git(absolute, ['branch', '--show-current'], { quiet: true }) !== 'main') {
    throw new Error('release tagging requires the main branch');
  }
  if (git(absolute, ['rev-parse', '--verify', '--quiet', `refs/tags/${tag}`], { quiet: true, allowFailure: true })) {
    throw new Error(`tag ${tag} already exists`);
  }
  const head = git(absolute, ['rev-parse', 'HEAD'], { quiet: true });
  const remote = git(absolute, ['rev-parse', '--verify', 'refs/remotes/origin/main'], { quiet: true });
  if (head !== remote) throw new Error('main must exactly match origin/main before tagging');
  checkVersions(absolute, tag);
  git(absolute, ['tag', '-a', tag, '-m', `Archmage ${version}`]);
  return tag;
}

function parseCli(argv) {
  const args = [...argv];
  let root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  const rootIndex = args.indexOf('--root');
  if (rootIndex >= 0) {
    if (!args[rootIndex + 1]) throw new Error('--root requires a path');
    root = resolve(args[rootIndex + 1]);
    args.splice(rootIndex, 2);
  }
  return { root, args };
}

function main(argv) {
  const { root, args } = parseCli(argv);
  const command = args.shift();
  if (command === 'check') {
    const tagIndex = args.indexOf('--tag');
    const tag = tagIndex >= 0 ? args[tagIndex + 1] : undefined;
    if (tagIndex >= 0 && !tag) throw new Error('--tag requires a value');
    if (args.length !== (tagIndex >= 0 ? 2 : 0)) throw new Error('unexpected check arguments');
    process.stdout.write(`${checkVersions(root, tag)}\n`);
    return;
  }
  if (command === 'set' && args.length === 1) {
    process.stdout.write(`${setVersion(root, args[0])}\n`);
    return;
  }
  if (command === 'tag' && args.length === 1) {
    process.stdout.write(`${createReleaseTag(root, args[0])}\n`);
    return;
  }
  throw new Error('usage: release-version.mjs check [--tag vX.Y.Z] [--root PATH] | set X.Y.Z [--root PATH] | tag X.Y.Z [--root PATH]');
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(`release-version: ${error.message}\n`);
    process.exitCode = 1;
  }
}
