# Archmage macOS arm64 Distribution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish Archmage 1.0.0 as a reproducible unsigned macOS 14+ Apple Silicon DMG with checksum, GitHub Pre-release automation, and bilingual installation guidance.

**Architecture:** Keep the existing Tauri 2 desktop runtime. Add one Node-based version contract, one macOS-only DMG verifier, static release contract tests, and a two-job GitHub Actions workflow that separates read-only building from write-enabled Release publication. A pushed `v1.0.0` annotated tag is the only public-release trigger; manual dispatch builds an artifact without publishing.

**Tech Stack:** Rust 1.95, Tauri 2, React 19, TypeScript/Vitest, Node.js 22, npm, macOS `hdiutil`/`plutil`/`lipo`, GitHub Actions and GitHub CLI.

---

## File structure

- Create `scripts/release-version.mjs`: pure version parsing/updating helpers plus `check`, `set`, and guarded `tag` CLI commands.
- Create `scripts/release-version.test.mjs`: Node built-in tests for version synchronization and tag safety using disposable fixture repositories.
- Create `scripts/verify-macos-release.sh`: mount and validate a DMG, app metadata, architecture, minimum OS, and forbidden payloads.
- Create `apps/archmage-desktop/src/releasePackaging.test.ts`: repository-level contract test for Tauri packaging, workflow permissions/triggers, verification commands, and public artifact names.
- Modify `Cargo.toml`, `Cargo.lock`, `apps/archmage-desktop/package.json`, `apps/archmage-desktop/package-lock.json`, and `apps/archmage-desktop/src-tauri/tauri.conf.json`: synchronize version 1.0.0 and activate arm64 DMG packaging for macOS 14+.
- Create `.github/workflows/release-macos.yml`: manual artifact build and tag-triggered Pre-release publication.
- Create `.github/release-notes/v1.0.0.md`: bilingual known limitations and installation summary consumed by the workflow.
- Create `docs/en/runbooks/macos-installation.md` and `docs/zh-CN/runbooks/macos-installation.md`: semantically aligned installation, upgrade, data, and troubleshooting instructions.
- Modify `README.md`, `docs/en/README.md`, and `docs/zh-CN/README.md`: expose the macOS download and installation path.

### Task 1: Add a tested 1.0.0 version contract

**Files:**
- Create: `scripts/release-version.mjs`
- Create: `scripts/release-version.test.mjs`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `apps/archmage-desktop/package.json`
- Modify: `apps/archmage-desktop/package-lock.json`
- Modify: `apps/archmage-desktop/src-tauri/tauri.conf.json`

- [ ] **Step 1: Write the failing version-helper tests**

Use Node's built-in test runner. The fixture must contain a workspace `Cargo.toml`, desktop `package.json`, Tauri config and a minimal `Cargo.lock`. Assert:

```js
test('check rejects mismatched workspace, npm and Tauri versions', () => {
  const root = fixture({ cargo: '1.0.0', npm: '0.1.0', tauri: '1.0.0' });
  assert.throws(() => checkVersions(root), /version mismatch/);
});

test('setVersion updates every public version source', () => {
  const root = fixture({ cargo: '0.1.0', npm: '0.1.0', tauri: '0.1.0' });
  setVersion(root, '1.0.0');
  assert.deepEqual(readVersions(root), {
    workspace: '1.0.0', npm: '1.0.0', npmLock: '1.0.0', tauri: '1.0.0',
  });
});

test('tag command rejects dirty, non-main and existing tags', () => {
  const root = gitFixture('1.0.0');
  writeFileSync(join(root, 'dirty.txt'), 'dirty');
  assert.throws(() => createReleaseTag(root, '1.0.0'), /clean working tree/);
});
```

The CLI contract is:

```text
node scripts/release-version.mjs check [--root PATH] [--tag v1.0.0]
node scripts/release-version.mjs set 1.0.0 [--root PATH]
node scripts/release-version.mjs tag 1.0.0 [--root PATH]
```

- [ ] **Step 2: Run the test to verify RED**

Run: `node --test scripts/release-version.test.mjs`

Expected: FAIL because `scripts/release-version.mjs` does not exist.

- [ ] **Step 3: Implement the version helper**

Export `parseVersion`, `readVersions`, `checkVersions`, `setVersion`, and `createReleaseTag`. Accept only SemVer values matching `MAJOR.MINOR.PATCH` with an optional legal prerelease suffix. Update both the top-level `version` and `packages[""].version` in `package-lock.json`, and write JSON with two-space indentation and a final newline. Update the workspace version in TOML without broad text replacement. Run `cargo check --workspace` after setting so Cargo refreshes `Cargo.lock` rather than hand-editing lockfile dependency records.

`createReleaseTag` must execute Git with argv, never a shell string. It must verify:

```text
current branch == main
git status --porcelain is empty
HEAD == refs/remotes/origin/main
refs/tags/v<version> does not exist
checkVersions(root, tag) succeeds
```

Then create only a local annotated tag using:

```text
git tag -a v<version> -m "Archmage <version>"
```

It must not push the tag; publication remains an explicit operator action.

- [ ] **Step 4: Verify GREEN on disposable fixtures**

Run: `node --test scripts/release-version.test.mjs`

Expected: all version synchronization and guarded-tag tests pass without modifying the real repository.

- [ ] **Step 5: Set the real repository version to 1.0.0**

Run:

```bash
node scripts/release-version.mjs set 1.0.0
cargo check --workspace
node scripts/release-version.mjs check
```

Expected: workspace, npm and Tauri versions report `1.0.0`, and the affected `Cargo.lock`/`package-lock.json` package records also use 1.0.0.

- [ ] **Step 6: Commit the version contract**

```bash
git add scripts/release-version.mjs scripts/release-version.test.mjs Cargo.toml Cargo.lock \
  apps/archmage-desktop/package.json apps/archmage-desktop/package-lock.json \
  apps/archmage-desktop/src-tauri/tauri.conf.json
git commit -m "build: establish 1.0.0 release version contract"
```

### Task 2: Enable and test Tauri arm64 DMG packaging

**Files:**
- Modify: `apps/archmage-desktop/src-tauri/tauri.conf.json`
- Create: `apps/archmage-desktop/src/releasePackaging.test.ts`

- [ ] **Step 1: Write the failing Tauri packaging contract test**

Read the config from the test and assert exact public packaging behavior:

```ts
expect(config.version).toBe('1.0.0');
expect(config.identifier).toBe('dev.archmage.workbench');
expect(config.bundle.active).toBe(true);
expect(config.bundle.targets).toEqual(['dmg']);
expect(config.bundle.macOS.minimumSystemVersion).toBe('14.0');
expect(config.plugins?.updater).toBeUndefined();
```

Also assert the icon list still includes `icons/icon.icns` so enabling bundle cannot silently regress the approved app icon.

- [ ] **Step 2: Run the test to verify RED**

Run: `cd apps/archmage-desktop && npm test -- --run src/releasePackaging.test.ts`

Expected: FAIL because `bundle.active` is false and DMG/minimum-system settings are absent.

- [ ] **Step 3: Implement the minimal Tauri bundle configuration**

Set:

```json
"bundle": {
  "active": true,
  "targets": ["dmg"],
  "macOS": { "minimumSystemVersion": "14.0" },
  "icon": [
    "icons/32x32.png",
    "icons/128x128.png",
    "icons/128x128@2x.png",
    "icons/icon.icns",
    "icons/icon.ico"
  ]
}
```

Do not add updater, signing identity, entitlements, Intel target or Universal target.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cd apps/archmage-desktop
npm test -- --run src/releasePackaging.test.ts src/iconAssets.test.ts
npm run build
```

Expected: packaging and icon contract tests pass; the production frontend builds.

- [ ] **Step 5: Commit Tauri packaging**

```bash
git add apps/archmage-desktop/src-tauri/tauri.conf.json \
  apps/archmage-desktop/src/releasePackaging.test.ts
git commit -m "build: enable macOS arm64 DMG packaging"
```

### Task 3: Add a fail-closed macOS DMG verifier

**Files:**
- Create: `scripts/verify-macos-release.sh`
- Modify: `apps/archmage-desktop/src/releasePackaging.test.ts`

- [ ] **Step 1: Extend the failing contract test for the verifier**

Assert the script exists, is executable, uses strict shell mode, and contains the mandatory platform checks:

```ts
expect(mode & 0o111).not.toBe(0);
expect(script).toContain('set -euo pipefail');
expect(script).toContain('hdiutil verify');
expect(script).toContain('hdiutil attach');
expect(script).toContain('lipo -archs');
expect(script).toContain('CFBundleIdentifier');
expect(script).toContain('CFBundleShortVersionString');
expect(script).toContain('LSMinimumSystemVersion');
```

- [ ] **Step 2: Run the test to verify RED**

Run: `cd apps/archmage-desktop && npm test -- --run src/releasePackaging.test.ts`

Expected: FAIL because `scripts/verify-macos-release.sh` does not exist.

- [ ] **Step 3: Implement the verifier**

The interface is:

```text
scripts/verify-macos-release.sh <dmg-path> <version>
```

It must:

- reject non-macOS hosts and missing arguments;
- use `mktemp -d` for its mount point and always detach/remove it in a trap;
- run `hdiutil verify`, then attach read-only and non-browsing;
- locate exactly one `Archmage.app`;
- extract `CFBundleIdentifier`, `CFBundleShortVersionString`, `CFBundleExecutable`, and `LSMinimumSystemVersion` with `plutil`;
- require identifier `dev.archmage.workbench`, the supplied version, and minimum system `14.0`;
- require `lipo -archs` to return exactly `arm64`;
- reject `.env`, `.git`, `*.db`, `*.sqlite`, `node_modules`, `target`, fixture directories, and obvious credential filenames inside the app bundle;
- print the observed code-signing metadata for audit, but do not claim Developer ID or notarization success.

- [ ] **Step 4: Verify the script's local failure behavior**

Run:

```bash
scripts/verify-macos-release.sh
scripts/verify-macos-release.sh /does/not/exist.dmg 1.0.0
```

Expected: both commands fail nonzero with actionable messages and leave no mount behind.

- [ ] **Step 5: Verify the static contract and commit**

Run: `cd apps/archmage-desktop && npm test -- --run src/releasePackaging.test.ts`

```bash
git add scripts/verify-macos-release.sh apps/archmage-desktop/src/releasePackaging.test.ts
git commit -m "build: verify macOS release bundles"
```

### Task 4: Build and publish GitHub Pre-releases safely

**Files:**
- Create: `.github/workflows/release-macos.yml`
- Create: `.github/release-notes/v1.0.0.md`
- Modify: `apps/archmage-desktop/src/releasePackaging.test.ts`

- [ ] **Step 1: Extend the failing workflow contract test**

Read `.github/workflows/release-macos.yml` and assert it includes all required boundaries:

```ts
expect(workflow).toContain("tags:\n      - 'v*'");
expect(workflow).toContain('workflow_dispatch:');
expect(workflow).toContain('runs-on: macos-15');
expect(workflow).toContain('permissions:\n  contents: read');
expect(workflow).toContain('cargo clippy --workspace --all-targets -- -D warnings');
expect(workflow).toContain('npm run tauri -- build --target aarch64-apple-darwin --bundles dmg');
expect(workflow).toContain('scripts/verify-macos-release.sh');
expect(workflow).toContain('shasum -a 256');
expect(workflow).toContain('prerelease: true');
```

Assert the publish job has `contents: write`, depends on the build job, downloads only the named build artifact, and is conditioned on `refs/tags/v`. Assert the manual-dispatch path cannot reach the publish job.

- [ ] **Step 2: Run the test to verify RED**

Run: `cd apps/archmage-desktop && npm test -- --run src/releasePackaging.test.ts`

Expected: FAIL because the workflow and release notes do not exist.

- [ ] **Step 3: Implement the two-job workflow**

The `build` job uses default `contents: read` and `macos-15`. It must:

```text
checkout
setup Node 22 with npm cache
rustup show (honor rust-toolchain.toml)
npm ci in apps/archmage-desktop
node scripts/release-version.mjs check --tag "$GITHUB_REF_NAME" (tag builds)
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm test -- --run
npm run build
npm run tauri -- build --target aarch64-apple-darwin --bundles dmg
copy exactly one DMG to dist/release/Archmage_<version>_aarch64.dmg
scripts/verify-macos-release.sh <dmg> <version>
shasum -a 256 <dmg> > <dmg>.sha256
upload only the DMG and checksum as artifact archmage-macos-arm64-<version>
```

For manual dispatch, derive the version from the checked-in contract and build an Actions Artifact only.

The `release` job depends on `build`, runs only for `refs/tags/v*`, and alone has `contents: write`. It must download the exact named artifact, create or update a Draft GitHub Release using `.github/release-notes/v1.0.0.md`, upload both files with `--clobber`, and publish it as a Pre-release only after both assets exist. A failed upload must leave a Draft, not a partial public release.

- [ ] **Step 4: Write the bilingual v1.0.0 release notes**

Include in both languages:

```text
Apple Silicon only
macOS 14+
unsigned and not notarized
Finder right-click Open / Privacy & Security Open Anyway
Provider installation and authentication are separate
manual upgrade only
DMG SHA-256 verification command
```

- [ ] **Step 5: Verify GREEN and workflow syntax contract**

Run:

```bash
cd apps/archmage-desktop
npm test -- --run src/releasePackaging.test.ts
cd ../..
node scripts/release-version.mjs check
git diff --check
```

Expected: the workflow contract test passes and no whitespace errors remain.

- [ ] **Step 6: Commit release automation**

```bash
git add .github/workflows/release-macos.yml .github/release-notes/v1.0.0.md \
  apps/archmage-desktop/src/releasePackaging.test.ts
git commit -m "ci: publish macOS arm64 preview releases"
```

### Task 5: Document download, installation and manual upgrades bilingually

**Files:**
- Create: `docs/en/runbooks/macos-installation.md`
- Create: `docs/zh-CN/runbooks/macos-installation.md`
- Modify: `README.md`
- Modify: `docs/en/README.md`
- Modify: `docs/zh-CN/README.md`
- Modify: `apps/archmage-desktop/src/releasePackaging.test.ts`

- [ ] **Step 1: Extend the failing documentation contract test**

Assert both runbooks exist and contain the same operational anchors:

```ts
for (const document of [english, chinese]) {
  expect(document).toContain('Archmage_1.0.0_aarch64.dmg');
  expect(document).toContain('macOS 14');
  expect(document).toContain('Apple Silicon');
  expect(document).toContain('shasum -a 256');
  expect(document).toContain('GitHub Releases');
}
```

Use language-specific assertions for unsigned/Gatekeeper instructions, separate Provider installation, Application Support data preservation, manual upgrade, rollback limits, and app-only versus data deletion. Assert all three README files link to the appropriate runbook and `https://github.com/WeifeiMeng/archmage/releases` rather than a nonexistent stable `releases/latest` URL.

- [ ] **Step 2: Run the test to verify RED**

Run: `cd apps/archmage-desktop && npm test -- --run src/releasePackaging.test.ts`

Expected: FAIL because the installation runbooks and README download links are absent.

- [ ] **Step 3: Write semantically aligned English and Chinese runbooks**

Both documents must cover, in the same order:

1. supported system and Preview trust status;
2. download and SHA-256 verification;
3. DMG install and first open through Finder or Privacy & Security;
4. why removing quarantine is not the normal path;
5. first launch and separate Provider install/auth/verification states;
6. manual upgrade while preserving Application Support data;
7. backup and downgrade/schema warning;
8. app-only uninstall versus explicit data deletion;
9. failure diagnosis for wrong architecture, unsupported OS, Gatekeeper, damaged download and missing Provider.

Do not claim the package is signed, notarized, auto-updating, Intel-compatible, or bundled with a Provider.

- [ ] **Step 4: Add README download entry points**

The root Chinese README links to the Chinese runbook. `docs/en/README.md` and `docs/zh-CN/README.md` link to their matching runbook. All use the Releases index because GitHub's `latest` redirect excludes Pre-releases.

- [ ] **Step 5: Verify GREEN and semantic parity**

Run:

```bash
cd apps/archmage-desktop
npm test -- --run src/releasePackaging.test.ts
cd ../..
rg -n "Apple Silicon|macOS 14|aarch64|SHA-256|Provider|Application Support" \
  docs/en/runbooks/macos-installation.md docs/zh-CN/runbooks/macos-installation.md
git diff --check
```

Expected: tests pass and every operational topic appears in both documents.

- [ ] **Step 6: Commit installation documentation**

```bash
git add README.md docs/en/README.md docs/zh-CN/README.md \
  docs/en/runbooks/macos-installation.md docs/zh-CN/runbooks/macos-installation.md \
  apps/archmage-desktop/src/releasePackaging.test.ts
git commit -m "docs: add macOS preview installation guide"
```

### Task 6: Build and inspect the real local DMG

**Files:**
- No source changes expected; fix defects through a new failing regression test before changing source.

- [ ] **Step 1: Run the complete non-live release gate**

Run from repository root:

```bash
node --test scripts/release-version.test.mjs
node scripts/release-version.mjs check
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/archmage-desktop
npm ci
npm test -- --run
npm run build
npm run tauri -- build --target aarch64-apple-darwin --bundles dmg
```

Expected: every command exits zero. Existing ignored live tests remain explicitly reported as ignored, not counted as real Provider acceptance.

- [ ] **Step 2: Normalize and verify the real DMG**

Run from repository root after locating exactly one built DMG:

```bash
mkdir -p dist/release
cp target/aarch64-apple-darwin/release/bundle/dmg/*.dmg \
  dist/release/Archmage_1.0.0_aarch64.dmg
scripts/verify-macos-release.sh dist/release/Archmage_1.0.0_aarch64.dmg 1.0.0
shasum -a 256 dist/release/Archmage_1.0.0_aarch64.dmg \
  > dist/release/Archmage_1.0.0_aarch64.dmg.sha256
shasum -a 256 -c dist/release/Archmage_1.0.0_aarch64.dmg.sha256
```

Expected: DMG, metadata and arm64 checks pass, and the checksum verifies. Keep `dist/release` untracked.

- [ ] **Step 3: Perform a manual installed-app smoke test**

Mount the DMG, copy `Archmage.app` to a disposable temporary Applications-like directory, launch it once, and verify the main window opens without Rust, Node or the source tree being involved. Confirm missing Providers produce installation guidance rather than an application crash. Do not claim the full clean-machine gate unless this is performed on a separate clean Apple Silicon macOS 14+ machine.

- [ ] **Step 4: Run a release-focused code review**

Review the diff against `docs/superpowers/specs/2026-11-20-macos-arm64-distribution-design.md`, focusing on tag safety, workflow permissions, artifact provenance, path quoting, DMG mount cleanup, accidental secrets, unsigned wording and bilingual parity. Fix Critical/Important findings with regression tests and rerun the affected gates.

- [ ] **Step 5: Commit any review fixes**

If fixes were required:

```bash
git add scripts/release-version.mjs scripts/release-version.test.mjs \
  scripts/verify-macos-release.sh .github/workflows/release-macos.yml \
  .github/release-notes/v1.0.0.md Cargo.toml Cargo.lock README.md \
  apps/archmage-desktop/package.json apps/archmage-desktop/package-lock.json \
  apps/archmage-desktop/src-tauri/tauri.conf.json \
  apps/archmage-desktop/src/releasePackaging.test.ts \
  docs/en/README.md docs/zh-CN/README.md \
  docs/en/runbooks/macos-installation.md \
  docs/zh-CN/runbooks/macos-installation.md
git commit -m "fix: harden macOS release pipeline"
```

If no fixes were required, do not create an empty commit.

### Task 7: Prepare, but do not prematurely publish, v1.0.0

**Files:**
- No source changes expected.

- [ ] **Step 1: Verify repository and remote state**

Run:

```bash
git status --short
git log --oneline --decorate -8
git fetch origin
git rev-list --left-right --count origin/main...HEAD
node scripts/release-version.mjs check
```

Expected before integration: feature worktree is clean, every implementation commit is visible, and version is 1.0.0. Do not create a tag while HEAD is not the verified `main` publication commit.

- [ ] **Step 2: Complete the development branch workflow**

Use `superpowers:finishing-a-development-branch`. Present merge/push choices and only integrate after the user selects one. The publication path must result in a clean local `main` exactly matching pushed `origin/main`.

- [ ] **Step 3: Create and inspect the local annotated tag only after integration approval**

Run on clean synchronized `main`:

```bash
node scripts/release-version.mjs tag 1.0.0
git show --no-patch --decorate v1.0.0
```

Expected: annotated `v1.0.0` points to the verified release commit. This step does not push.

- [ ] **Step 4: Push the tag only with explicit publication confirmation**

Run:

```bash
git push origin v1.0.0
```

Then monitor the `release-macos.yml` run to completion and inspect the GitHub Pre-release assets and SHA-256 before stating that Archmage 1.0.0 is published. A local tag or successful local DMG is not a published release.

## Final verification matrix

Before claiming completion, record each item separately:

| Claim | Required evidence |
|---|---|
| Source implementation complete | Clean feature worktree and reviewed diff |
| Automated tests pass | Fresh Rust, Node helper and Vitest output with zero failures |
| Local package builds | Fresh Tauri arm64 DMG command exits zero |
| Local package is valid | Verifier and SHA-256 check exit zero |
| Install smoke passes | Observed launch from copied `.app` outside build tree |
| Clean-machine acceptance passes | Separate macOS 14+ Apple Silicon host with no dev toolchain |
| v1.0.0 is published | Tag pushed, GitHub workflow complete, public Pre-release assets inspected |

Do not collapse these states into one “done” claim.
