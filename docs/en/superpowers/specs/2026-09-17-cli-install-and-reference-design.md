# CLI Installation and Reference Design

[简体中文](../../../zh-CN/superpowers/specs/2026-09-17-cli-install-and-reference-design.md) | English

**Status:** Proposed for implementation

## Goal

Make the development build available as a normal `archmage` command and provide one truthful command reference that clearly separates working functionality from planned commands.

## Installation contract

Archmage uses Cargo's standard local package installation:

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
```

This installs the binary into `$HOME/.local/bin`, which is already part of the target development environment's `PATH`. It keeps installation explicit without modifying shell startup files. A successful installation must be verified with:

```bash
command -v archmage
archmage --version
archmage --help
```

`cargo build` remains side-effect free with respect to the user's `PATH`; it only produces `target/debug/archmage`. Archmage will not add a post-build hook, shell alias, or repository-relative symlink. Reinstalling after source changes is explicit.

## Documentation contract

Create `docs/cli-reference.md` as the canonical CLI manual. It contains:

- prerequisites and one-time installation;
- upgrade/reinstall and uninstall commands;
- data-directory behavior and `ARCHMAGE_HOME`;
- exact semantics and examples for `doctor --offline` and `doctor`;
- a concise output-field guide;
- a clearly separated section for `run`, `status`, `logs`, `resume`, and `cancel`, all marked unavailable until the Runtime workflow is wired;
- common errors, exit status meaning, and safe diagnostics handling.

The repository README and current usage guide link to the canonical manual instead of duplicating a competing command reference.

## Verification

Implementation acceptance requires:

1. all workspace tests pass;
2. `cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"` succeeds;
3. the resolved `archmage` binary reports the current package version and help;
4. `archmage doctor --offline` succeeds against a disposable data directory;
5. documentation contains no example claiming that the incomplete workflow commands run successfully;
6. formatting and `git diff --check` pass.

## Non-goals

- Packaging a signed desktop or platform installer.
- Homebrew, npm, or release-binary distribution.
- Making incomplete Workflow commands appear operational.
- Automatically changing shell startup files or global `PATH` configuration.
