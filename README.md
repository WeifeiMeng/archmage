# Archmage

Archmage is a local-first collaboration runtime that connects local agent providers and lets capability-oriented agents work together on one code workspace.

The first executable slice is being built to connect a real Codex CLI provider and run four logical agents—Designer, Coder, Tester, and Reviewer—through explicit, auditable handoffs in an isolated Git worktree. The full workflow is not implemented yet.

Install the current development CLI from the repository root:

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
archmage doctor --offline
```

The [CLI command reference (中文)](docs/cli-reference.md) is the source of truth for working commands and explicitly marks the Workflow commands that are not implemented yet. The [current-version guide (中文)](docs/guide.md) contains deeper contributor verification and architecture notes.

Start with [the documentation index](docs/README.md). The original product brief remains in [target.md](target.md).
