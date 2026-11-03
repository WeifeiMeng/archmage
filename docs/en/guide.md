[简体中文](../zh-CN/guide.md) | English

# Current Version Guide

Updated: 2026-10-14. This guide describes the real entry points in the current source. Use the [CLI command reference](cli-reference.md) for daily CLI arguments and the [desktop client guide](runbooks/desktop-s0.md) for graphical operation.

## Current capabilities

Archmage has a runnable vertical chain: it connects to a real local Codex CLI, schedules the logical Designer, Coder, Tester, and Reviewer capabilities in sequence inside an isolated Git worktree for each Run, and persists Run, Node, Attempt, Artifact, Handoff, ChangeSet, Agent-binding snapshots, and events.

Available entry points:

- CLI: `doctor`, `run`, `status`, `logs`, and `cancel`;
- Tauri desktop client: manage local Agents, configure each Capability, draw message-flow Teams on a canvas, choose a Team for a new Run, inspect delivery messages or legacy consultation progress, open the workspace, and cancel Runs;
- process recovery: on startup, terminate abandoned Codex main-task/consultation processes that have lost their Run owner and converge orphaned Runs to `interrupted`.

Not yet available: `resume`, `logs --follow`, a background daemon, or arbitrary workflow orchestration.

## 1. Prepare the environment

```bash
cd /Users/mengweifei/Desktop/code/github/archmage
rustc --version
cargo --version
git --version
node --version
npm --version
codex --version
codex login status
```

Archmage uses Git/Codex from the current `PATH` and the current Codex login and model configuration. It does not silently switch accounts or models.

To isolate development data:

```bash
export ARCHMAGE_HOME="$HOME/Library/Application Support/archmage-dev"
```

The CLI and desktop must use the same `ARCHMAGE_HOME` to see the same Runs.

## 2. Install and run the CLI

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
export PATH="$HOME/.local/bin:$PATH"
archmage doctor --offline
archmage doctor
```

Run the real four-capability flow:

```bash
archmage run \
  --repo /absolute/path/to/repository \
  --goal "Implement a small, verifiable feature"
```

Inspect and cancel:

```bash
archmage status <run-id> --json
archmage logs <run-id>
archmage cancel <run-id>
```

`run` waits for a terminal state. `logs` reads persisted events but does not support `--follow`; `resume` is not implemented. The current CLI `run` has no `--team` option and uses the default Team. Select a custom Team from the desktop New Run page.

## 3. Start the desktop client

```bash
./scripts/dev-desktop.sh
```

On first use, open Agent Management, inspect the local Codex CLI, and click Verify Connection to make a real read-only request. In Capability Management, confirm the Agent bound to each of the four capabilities; optionally add role-specific instructions and manual Skill/Tool names and purposes. In Team Management, create a message-flow Team, select capabilities, and connect them on the canvas. Finally, return to New Run, select the Team and Git repository, and enter the goal. Skill/Tool configuration is advisory prompt guidance: it neither verifies installation nor restricts callable scope. See the [desktop client guide](runbooks/desktop-s0.md) for detailed operation and shutdown behavior.

## 4. Key Runtime semantics

- A Run contains the Nodes selected by its Team; four built-in capability types are currently available.
- Multiple capabilities do not require multiple long-lived Agent instances. The system supports several Codex instances and initially creates `codex-default`.
- Agents are bound globally by capability in Capability Management; there is no per-Run override. Runtime-owned roles and output contracts remain fixed, while supplemental prompts and Skill/Tool guidance are configurable.
- New Teams use message-flow mode: select at least two capabilities, explicitly choose entry and terminal-signal capabilities, and connect directed edges on the canvas. Feedback cycles such as Coder → Reviewer → Coder are allowed. Every delivery triggers one execution of the target capability's bound real Agent. Structured output selects one or more connected recipients, while the terminal capability may choose completion. Execution is serialized, cycles are limited to 24 steps, and pending deliveries become skipped at termination.
- Cross-Provider edges hand off validated JSON artifacts; they do not share Provider conversation history. See [Cross-provider Capability handoff](architecture/cross-provider-handoff.md) for the Codex/DSH boundary.
- The default and existing Teams retain legacy consultation mode: edges trigger read-only consultation, downstream approval gates, and up to three revision rounds. An old Team can be copied explicitly into a message-flow Team without changing the original.
- Run creation freezes the selected Team, Capability configuration, and concrete Agent bindings. Later global changes do not alter historical Runs.
- Each Node has an independent provider result plus explicit Context/Handoff.
- Tester receives the testing task, workspace, and upstream artifacts directly; its bound real Agent chooses test methods and commands.
- Every state transition and event is written to SQLite.
- Every Run uses an isolated Git worktree; the user's repository checkout is not the execution workspace.
- There is no daemon: after the client or CLI exits, no new node is scheduled.

## 5. Development verification

Routine checks that do not call a model:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm test --prefix apps/archmage-desktop
npm run build --prefix apps/archmage-desktop
cargo check -p archmage-desktop
git diff --check
```

Tests that require real Codex are ignored by default, so normal `cargo test` does not consume model quota. Online environment probe:

```bash
cargo test -p archmage-core --test codex_live real_readonly_probe \
  -- --ignored --nocapture
```

Final acceptance of the main Runtime chain should use a temporary Git fixture and real Codex, verify that all four nodes reach a successful terminal state, that Tester records commands it actually selected, and that Reviewer produces the outcome.

## 6. Data and diagnostics

`config.data_root` in `archmage doctor --offline` is the actual data directory. Its main contents are:

- `archmage.db`: the schema-v15 execution ledger, Agent instances, Capability configurations, Teams, delivery messages, consultations, and events;
- Run worktrees and Artifacts;
- Codex JSONL, stderr, and metadata diagnostics.

Diagnostics may contain local paths, target code, or model context. Do not upload or commit them without sanitization. Do not edit SQLite manually; Runtime relies on revisions, transactions, foreign keys, and event order to preserve state consistency.

## Troubleshooting

### The CLI cannot find a Run created by the desktop

Confirm both processes use the same `ARCHMAGE_HOME`. When it is unset, both use the operating-system application data directory.

### Offline Doctor succeeds but online Doctor fails

Offline checks do not verify login, network, quota, or model service. Inspect the diagnostics path in the error and sanitize files before sharing.

### Does a Run continue after the client closes?

No. On normal close, the client asks to cancel active Runs started in that session. An orphaned Run from an abnormal exit becomes `interrupted` on the next startup, after active Codex main-task or consultation processes are terminated. Recreate the Run until `resume` is implemented.
