[简体中文](../../../zh-CN/superpowers/plans/2026-09-28-one-command-desktop-dev.md) | English

# One-command Desktop Development Implementation Plan


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add one executable repository-root script that checks prerequisites, installs missing frontend dependencies, and starts the complete Tauri desktop development application.

**Architecture:** A focused Bash entry point resolves the repository root from its own path and delegates lifecycle management to the existing Tauri CLI. Tauri remains responsible for launching Vite and the embedded Rust Runtime; no backend daemon or duplicate frontend process is introduced.

**Tech Stack:** Bash, npm, Vite, Tauri 2, Rust/Cargo.

---

### Task 1: Add the one-command launcher

**Files:**
- Create: `scripts/dev-desktop.sh`

- [ ] **Step 1: Add the launcher**

Create an executable strict-mode Bash script that resolves the repository root, checks `git`, `cargo`, `node`, `npm`, and `codex`, installs dependencies only when `apps/archmage-desktop/node_modules` is absent, then uses `exec` for the existing Tauri command:

```bash
#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"
desktop_dir="$repo_root/apps/archmage-desktop"

for executable in git cargo node npm codex; do
  if ! command -v "$executable" >/dev/null 2>&1; then
    printf 'Archmage desktop requires %s on PATH.\n' "$executable" >&2
    exit 1
  fi
done

if [[ ! -d "$desktop_dir/node_modules" ]]; then
  npm install --prefix "$desktop_dir"
fi

cd "$repo_root"
exec npm run tauri dev --prefix "$desktop_dir"
```

- [ ] **Step 2: Make it executable and verify syntax**

Run:

```bash
chmod +x scripts/dev-desktop.sh
bash -n scripts/dev-desktop.sh
```

Expected: exit code 0 with no output.

- [ ] **Step 3: Verify a missing prerequisite is explicit**

Create a temporary PATH containing links for all required executables except `codex`, then invoke the script with that PATH.

Expected: exit code 1 and stderr contains `Archmage desktop requires codex on PATH.` The Tauri command must not start.

### Task 2: Document the recommended entry point

**Files:**
- Modify: `README.md`
- Modify: `docs/runbooks/desktop-s0.md`

- [ ] **Step 1: Update README**

Make this the primary desktop startup example:

```bash
./scripts/dev-desktop.sh
```

State that it starts both the Vite UI and the embedded Rust Runtime. Preserve the two underlying npm commands in a troubleshooting subsection.

- [ ] **Step 2: Update the desktop runbook**

Use the same recommended command, explain first-run dependency installation/compilation, preserve `ARCHMAGE_HOME` pass-through, and retain manual npm commands for diagnosis.

- [ ] **Step 3: Check documentation and diff formatting**

Run:

```bash
rg -n "dev-desktop|tauri dev" README.md docs/runbooks/desktop-s0.md
git diff --check
```

Expected: both documents mention the one-command launcher and manual Tauri command; `git diff --check` exits 0.

### Task 3: Smoke-test and commit

**Files:**
- Verify: `scripts/dev-desktop.sh`
- Verify: `README.md`
- Verify: `docs/runbooks/desktop-s0.md`

- [ ] **Step 1: Run a bounded launch smoke test**

Run `./scripts/dev-desktop.sh`, observe Vite becoming ready and Tauri starting its Rust build/application, then terminate the development process. Do not create a Run or invoke the model.

Expected: output includes the Vite local URL and the Tauri `Running` line; termination leaves no launcher-owned process running.

- [ ] **Step 2: Commit the implementation**

```bash
git add scripts/dev-desktop.sh README.md docs/runbooks/desktop-s0.md
git commit -m "feat: add one-command desktop startup"
```
