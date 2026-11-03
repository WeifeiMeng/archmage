[简体中文](../zh-CN/cli-reference.md) | English

# Archmage CLI Command Reference

Updated: 2026-09-28. This reference describes the actual behavior of the current code.

## Start in one minute

Prerequisites: Git, Rust/Cargo, and an authenticated Codex CLI are installed locally.

```bash
git --version
cargo --version
codex --version
codex login status
```

Install the command from the Archmage repository root:

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
export PATH="$HOME/.local/bin:$PATH"
archmage doctor --offline
```

Run an online check before entering a real-model workflow:

```bash
archmage doctor
```

Then run the complete four-capability flow:

```bash
archmage run \
  --repo /absolute/path/to/your-repository \
  --goal "Implement a small, verifiable feature"
```

`run` invokes the real local Codex and executes Designer, Coder, Tester, and Reviewer in order. It waits until the Run finishes, then prints the complete JSON snapshot. Record the output `id` for later queries.

## Available and unavailable commands

Currently available:

- `archmage doctor [--offline]`
- `archmage run --repo ... --goal ...`
- `archmage status <run-id> [--json]`
- `archmage logs <run-id>`
- `archmage cancel <run-id>`

Not yet available:

- `archmage resume <run-id>`
- `archmage logs <run-id> --follow`

Unavailable commands fail explicitly and never fabricate execution results.

## Install, update, and uninstall

Install or replace an older build after source changes:

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
```

Verify:

```bash
command -v archmage
archmage --version
archmage --help
```

If `command -v archmage` prints nothing, add the installation directory to `PATH`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

If you only run `cargo build -p archmage-cli`, the command is at `target/debug/archmage` in the repository and is not installed into `PATH`:

```bash
cargo build -p archmage-cli
./target/debug/archmage --help
```

Uninstall:

```bash
cargo uninstall --root "$HOME/.local" archmage-cli
```

Uninstalling the CLI does not delete the Archmage database, workspaces, or diagnostic logs.

## Data directory

Archmage uses the current user's operating-system application data directory by default. `config.data_root` in `archmage doctor --offline` output is the exact location.

To isolate development data, set the same `ARCHMAGE_HOME` for every related command:

```bash
export ARCHMAGE_HOME="$HOME/Library/Application Support/archmage-dev"
archmage doctor --offline
```

`run`, `status`, `logs`, and `cancel` must read the same data directory or they cannot find the Run.

The directory contains:

- `archmage.db`: Run, Node, Attempt, Event, and control requests;
- `workspaces/`: one isolated Git worktree per Run;
- `diagnostics/` or `probe-*`: Codex invocation diagnostics.

These files may contain local paths, target source code, and model events. Do not publish or commit them without inspection.

## `doctor`: preflight environment checks

Check without invoking a model:

```bash
archmage doctor --offline
```

It checks the Git/Codex executable, CLI flag support, data directory, and SQLite migrations. `authentication` reports `not_verified_offline`.

Check with one real Codex invocation:

```bash
archmage doctor
```

Online mode requests schema-valid `{"ok":true}` in a separate temporary repository with read-only permission. It verifies the current Codex login, model connection, thread ID, and structured output. It may consume Codex quota and has a 120-second deadline.

Doctor only validates the environment. It does not modify the target repository or execute the four-capability Workflow.

## `run`: execute the four-capability chain

Minimal usage:

```bash
archmage run \
  --repo /absolute/path/to/repository \
  --goal "Add an empty-input error to the parser and verify it"
```

`run` does not accept a prewritten acceptance command. The Agent behind Tester chooses tests and inspection methods from the goal, repository, and upstream artifacts.

Execution semantics:

1. Create an isolated Git worktree for the Run.
2. Designer produces a design and explicit Handoff.
3. Coder implements in that workspace and produces a ChangeSet.
4. Tester selects test commands and records their commands, exit codes, results, and limitations in structured output.
5. Reviewer returns `approved` or `changes_requested` from the implementation, Tester result, and Handoff.
6. Persist all state and events to SQLite.

The four Agents are logical capabilities; they do not require four different providers or resident processes. The desktop can bind them to one or more real Codex instances. The CLI uses persisted global bindings and freezes their snapshot when creating the Run.

## `status`: inspect a Run

Concise output:

```bash
archmage status <run-id>
```

JSON output:

```bash
archmage status <run-id> --json
```

The output contains Run state/outcome, the four Node states, workspace path, and last event sequence.

## `logs`: read persisted events

```bash
archmage logs <run-id>
```

Each line is one JSON event in sequence order and can be filtered with `jq`:

```bash
archmage logs <run-id> | jq -c '{sequence,event_type,node_id,payload}'
```

`--follow` is not implemented. Use the desktop client for live observation.

## `cancel`: cancel a Run

```bash
archmage cancel <run-id>
```

Archmage persists the cancellation request. If the Run has an active Codex child process, it terminates that process before converging the Run and unfinished nodes to `cancelled`.

## Recovery after an unexpected process exit

There is no background daemon. After the CLI or desktop exits, no new node is scheduled in the background. On its next start, the desktop inspects abandoned Attempts: executions that cannot be confirmed alive become `interrupted` rather than being reported as successful.

`resume` is not implemented, so an interrupted Run currently must be recreated.

## Troubleshooting

### `archmage: command not found`

```bash
ls "$HOME/.local/bin/archmage"
printf '%s\n' "$PATH"
export PATH="$HOME/.local/bin:$PATH"
```

### `codex executable not found on PATH`

```bash
command -v codex
codex --version
codex login status
```

Archmage uses the current `PATH` and Codex configuration and does not read or print authentication credentials.

### Offline Doctor succeeds but online Doctor fails

Offline success does not prove login, network, quota, or model-service availability. Inspect `metadata.json`, `events.jsonl`, and `stderr.log` under the diagnostics path in the error. Sanitize them before sharing.

### Finding a Run ID

`archmage run` prints the Run ID to stderr when it starts, and the final JSON includes `id`. It is also visible under “Recent Runs” in the desktop client.

`archmage internal check` is a reserved low-level CheckRunner protocol, not a user entry point for the current built-in chain. Do not call it manually or depend on its interface stability.
