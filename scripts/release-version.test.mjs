import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import {
  checkVersions,
  createReleaseTag,
  readVersions,
  setVersion,
} from './release-version.mjs';

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function fixture({ cargo = '0.1.0', npm = cargo, npmLock = npm, tauri = cargo } = {}) {
  const root = mkdtempSync(join(tmpdir(), 'archmage-release-version-'));
  const desktop = join(root, 'apps', 'archmage-desktop');
  const tauriDir = join(desktop, 'src-tauri');
  mkdirSync(tauriDir, { recursive: true });
  writeFileSync(join(root, 'Cargo.toml'), `[workspace]\nmembers = []\n\n[workspace.package]\nversion = "${cargo}"\nedition = "2024"\n`);
  writeFileSync(join(root, 'Cargo.lock'), `version = 4\n\n[[package]]\nname = "archmage-core"\nversion = "${cargo}"\n`);
  writeJson(join(desktop, 'package.json'), { name: 'archmage-desktop', version: npm });
  writeJson(join(desktop, 'package-lock.json'), {
    name: 'archmage-desktop',
    version: npmLock,
    lockfileVersion: 3,
    packages: { '': { name: 'archmage-desktop', version: npmLock } },
  });
  writeJson(join(tauriDir, 'tauri.conf.json'), { productName: 'Archmage', version: tauri });
  return root;
}

function git(root, ...args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
}

function gitFixture(version = '1.0.0') {
  const root = fixture({ cargo: version });
  git(root, 'init', '-q', '-b', 'main');
  git(root, 'config', 'user.name', 'Archmage Test');
  git(root, 'config', 'user.email', 'archmage@example.invalid');
  git(root, 'add', '.');
  git(root, 'commit', '-qm', 'initial');
  git(root, 'update-ref', 'refs/remotes/origin/main', 'HEAD');
  return root;
}

test('check rejects mismatched workspace, npm, lockfile and Tauri versions', () => {
  const root = fixture({ cargo: '1.0.0', npm: '0.1.0', npmLock: '0.2.0', tauri: '1.0.0' });
  assert.throws(() => checkVersions(root), /version mismatch/);
});

test('setVersion updates every public version source', () => {
  const root = fixture();
  const packagePath = join(root, 'apps', 'archmage-desktop', 'package.json');
  const originalMode = statSync(packagePath).mode & 0o777;
  setVersion(root, '1.0.0');
  assert.deepEqual(readVersions(root), {
    workspace: '1.0.0',
    npm: '1.0.0',
    npmLock: '1.0.0',
    tauri: '1.0.0',
  });
  const lock = JSON.parse(readFileSync(join(root, 'apps', 'archmage-desktop', 'package-lock.json')));
  assert.equal(lock.packages[''].version, '1.0.0');
  assert.equal(statSync(packagePath).mode & 0o777, originalMode);
});

test('check validates an optional v-prefixed tag', () => {
  const root = fixture({ cargo: '1.0.0' });
  assert.doesNotThrow(() => checkVersions(root, 'v1.0.0'));
  assert.throws(() => checkVersions(root, 'v1.0.1'), /does not match/);
});

test('setVersion rejects invalid semantic versions without changing files', () => {
  const root = fixture();
  const before = readFileSync(join(root, 'Cargo.toml'), 'utf8');
  assert.throws(() => setVersion(root, 'version one'), /invalid semantic version/);
  assert.equal(readFileSync(join(root, 'Cargo.toml'), 'utf8'), before);
});

test('tag command rejects a dirty working tree', () => {
  const root = gitFixture();
  writeFileSync(join(root, 'dirty.txt'), 'dirty');
  assert.throws(() => createReleaseTag(root, '1.0.0'), /clean working tree/);
});

test('tag command rejects a non-main branch', () => {
  const root = gitFixture();
  git(root, 'switch', '-qc', 'feature');
  assert.throws(() => createReleaseTag(root, '1.0.0'), /main branch/);
});

test('tag command rejects an existing tag', () => {
  const root = gitFixture();
  git(root, 'tag', '-a', 'v1.0.0', '-m', 'existing');
  assert.throws(() => createReleaseTag(root, '1.0.0'), /already exists/);
});

test('tag command creates a local annotated tag at synchronized main', () => {
  const root = gitFixture();
  createReleaseTag(root, '1.0.0');
  assert.equal(git(root, 'tag', '--list', 'v1.0.0'), 'v1.0.0');
  assert.equal(git(root, 'cat-file', '-t', 'v1.0.0'), 'tag');
});
