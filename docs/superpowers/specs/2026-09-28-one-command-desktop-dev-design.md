# One-command Desktop Development Design

**Status:** Approved approach, pending implementation

## Goal

Provide one repository-root command that launches the Archmage desktop frontend and its embedded Rust Runtime for local development, while keeping the underlying commands visible for diagnosis.

## Command

Add an executable repository script:

```bash
./scripts/dev-desktop.sh
```

The script may be run from any working directory. It resolves the repository root from its own location and starts the desktop application from that root.

## Behavior

The script performs these steps in order:

1. Verify that `git`, `cargo`, `node`, `npm`, and `codex` are available on `PATH`.
2. Print a concise actionable error and exit nonzero when a prerequisite is missing.
3. Install frontend dependencies with `npm install --prefix apps/archmage-desktop` only when `apps/archmage-desktop/node_modules` is absent.
4. Replace the script process with `npm run tauri dev --prefix apps/archmage-desktop`.

Tauri's development command owns both sides of the application: it starts the Vite frontend and compiles/runs the Rust shell containing `archmage-core`. The script must not launch an additional backend daemon or a second Vite process.

`ARCHMAGE_HOME` and other caller environment variables pass through unchanged. The script does not select an account, change Codex configuration, or run an online model probe before launch.

## Documentation

The root README presents `./scripts/dev-desktop.sh` as the recommended desktop development command. It also explains that the Rust Runtime is embedded in the Tauri process and preserves the underlying manual commands for installation and troubleshooting:

```bash
npm install --prefix apps/archmage-desktop
npm run tauri dev --prefix apps/archmage-desktop
```

The desktop runbook references the same one-command entry point so the two guides do not diverge.

## Error handling and verification

The script uses strict shell mode and quotes all paths. Prerequisite failures identify the missing executable. Dependency-install or Tauri failures propagate their original nonzero exit code.

Verification remains small:

- a shell syntax check with `bash -n scripts/dev-desktop.sh`;
- a lightweight prerequisite-failure check with a restricted `PATH`;
- a short launch smoke check proving the command reaches Vite/Tauri startup, terminated after observing startup rather than running a Codex workflow.

The script does not add Fake Agent behavior and does not claim that launching the window proves a real Codex Run succeeds.
