# Archmage

Archmage is a local-first collaboration runtime that connects local agent providers and lets capability-oriented agents work together on one code workspace.

The first executable slice connects a real Codex CLI provider and runs four logical capabilities—Designer, Coder, Tester, and Reviewer—through explicit, auditable handoffs in an isolated Git worktree. It is available through both the CLI and a Tauri desktop workbench.

Install the current development CLI from the repository root:

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
archmage doctor --offline
archmage run --repo /absolute/path/to/repository --goal "your goal"
```

Start the complete desktop development application from the repository root:

```bash
./scripts/dev-desktop.sh
```

This one command checks the local toolchain, installs missing frontend dependencies, and starts both the Vite UI and the Tauri process containing the Rust Runtime. Archmage does not require a separate backend daemon.

In the desktop window, use **Agent 管理** to verify a local Codex connection, then **Capability 管理** to inspect the fixed four-role topology and configure each role's Agent binding and supplemental instructions. Changes affect new Runs only; the editable Team canvas is a later stage.

For troubleshooting, the underlying commands are:

```bash
npm install --prefix apps/archmage-desktop
npm run tauri dev --prefix apps/archmage-desktop
```

The [CLI command reference (中文)](docs/cli-reference.md) and [desktop client guide (中文)](docs/runbooks/desktop-s0.md) are the source of truth for working commands. The [current-version guide (中文)](docs/guide.md) contains deeper contributor verification and architecture notes.

Start with [the documentation index](docs/README.md). The original product brief remains in [target.md](target.md).
