[简体中文](../../zh-CN/runbooks/desktop-s0.md) | English

# Archmage Desktop Client Guide

The desktop client is the local workbench for the current Runtime. It calls `archmage-core` directly; it does not parse CLI stdout and has no separate background daemon.

## Current capabilities

- Select a local Git repository.
- Configure local Codex or DeepSeek Harness Agent instances and perform real connectivity verification.
- Select an Agent instance independently for each of the four roles in Capability Management.
- Select at least two capabilities in Team Management, designate entry and terminal capabilities, and draw message routes on the canvas.
- Create a Run from a Team, repository, and goal.
- Start real Provider Runs for the selected capabilities; the default Team remains Designer → Coder → Tester → Reviewer.
- Inspect recent Runs, selected capability node state, and the persisted event timeline.
- Inspect node-related events.
- Open the isolated workspace.
- Cancel an active Run.
- On client restart, terminate Provider processes that lost their owner and converge abandoned Runs.

The current version does not include graphical resume, arbitrary node types, or background resident execution. The Team canvas is limited to four built-in capabilities, but message-flow edges may form bidirectional cycles.

## Start the development client

Prerequisite checks:

```bash
git --version
cargo --version
node --version
npm --version
codex --version
codex login status
python3 -m pip show deepseek-harness-sdk deepseek-harness-runtime-bin
python3 -c 'import deepseek_harness; print("DeepSeek Harness SDK ready")'
```

Start from the Archmage repository root:

```bash
./scripts/dev-desktop.sh
```

The script checks Git, Cargo, Node.js, npm, and Codex CLI; installs frontend dependencies only when `node_modules` is absent; and starts Tauri. Tauri starts both the Vite UI and the process embedding the Rust `archmage-core` Runtime, with no separate backend daemon.

The first start installs or compiles dependencies and is slower. The desktop window opens after the Vite local URL and `Running .../archmage-desktop` appear.

To isolate data, set a dedicated directory before starting:

```bash
export ARCHMAGE_HOME="$HOME/Library/Application Support/archmage-desktop-dev"
./scripts/dev-desktop.sh
```

The script passes through `ARCHMAGE_HOME`, the current Codex configuration, and `DSH_HOME`. Archmage does not copy DeepSeek Harness credentials or endpoints; each Agent instance explicitly configures its Provider route and model.

For npm or Tauri troubleshooting, run the underlying commands separately:

```bash
npm install --prefix apps/archmage-desktop
npm run tauri dev --prefix apps/archmage-desktop
```

The desktop and CLI use the same data format. They see the same Runs when `ARCHMAGE_HOME` matches.

## Configure and verify local Agents

1. Open **Agent Management** from the sidebar. You can add Codex or DeepSeek Harness instances.
2. The default `codex-default` resolves `codex` from the current `PATH`. To use a specific installation, enter and save its absolute path.
3. Leave the model override empty to inherit Codex configuration, or set one for that Agent instance only.
4. Click **Inspect CLI** to check only the executable, version, required flags, and Git. “CLI available · not verified” does not prove account connectivity.
5. Click **Verify Connection** to make one real read-only Codex request with that instance's executable and model override, then display version, availability, and last verification time.
6. Open **Capability Management**, select Designer, Coder, Tester, and Reviewer in turn, and confirm each Execution Agent. Add role-specific focus under Supplemental Role Prompt. Under Skills and Tools, add names and purposes manually—for example Designer's `brainstorming` or `web_search`—then save the Capability. Runtime-owned roles and output contracts remain fixed.

A DeepSeek Harness instance resolves `python3` from `PATH` by default or accepts an absolute Python path containing `deepseek-harness-sdk`. Matching versions of `deepseek-harness-sdk` and `deepseek-harness-runtime-bin` are required, together with a Provider route and model. The instance inherits credentials, endpoint, and `DSH_HOME` from the Archmage process. **Inspect SDK** checks installation and versions only; **Verify Connection** starts a temporary read-only sidecar and makes one real request, which may consume quota. Archmage integrates through the official Python SDK and its own Cordis configuration and does not require modifications to the DeepSeek Harness repository.

**Add Agent Instance** supports multiple Codex or DeepSeek Harness configurations. Capability bindings are global, not per-Run overrides. Run creation freezes the current bindings and Capability configuration; later edits affect new Runs only.

Capability Management does not show topology or define Team relationships. Skill/Tool lists are prompt guidance for the bound real Agent; they are not proof of installation or availability and are not permission allowlists. Both execution and read-only consultation use the Capability configuration frozen at Run creation.

When Capability edits are unsaved, switching capabilities, leaving the page, or closing the window asks whether to discard them. Concurrent updates produce a conflict; reload before editing again.

An unbound instance may be disabled or deleted. Rebind every Capability before disabling or deleting a bound instance. New Run reports missing or unavailable bindings and warns about instances that have not passed real verification.

Within one Run, each DSH-bound Capability owns a separate long-lived sidecar and DSH conversation. Repeated feedback or message-flow deliveries to that Capability continue its conversation, but other capabilities do not share it. Designer, Reviewer, and consultations use read-only permissions; Coder and Tester use workspace-write permissions; interactive approvals are denied automatically. Legacy consultations use independent temporary conversations and do not enter the primary Capability conversation. Sidecars are reclaimed when the Run completes, fails, or is cancelled. Conversations are not restored after an Archmage restart; abandoned Runs converge to `interrupted`.

Claude, OpenClaw, and Hermes are displayed as unsupported and cannot run fake probes or bind to capabilities.

## Configure Team topology

1. Open **Team Management** and click **Create Team**. A new Team starts with an empty message-flow canvas; no Coder–Reviewer pair is created automatically.
2. Select at least two capabilities. For example, select Coder and Reviewer, set Coder as Entry Capability and Reviewer as Terminal-Signal Capability.
3. Drag from Coder's output handle to Reviewer's input handle to create Coder → Reviewer, then draw Reviewer → Coder as the feedback route. Keyboard users can press Enter on an output handle and then Enter on the target input handle. Drop on empty space or press Escape to cancel.
4. Select an edge to delete it. Drag node bodies or use arrow keys to move them. Name and save the Team. Every capability must be reachable from the entry and must have a path to the terminal capability.
5. Return to New Run and select the Team. Leaving with unsaved changes prompts; version conflicts require reload before editing.

Arrows represent deliverables and feedback, not consultation permission. Every delivery executes the real Agent bound to the target Capability once; two messages cause two executions, processed serially in queue order. Structured output selects one or more connected downstream capabilities, or the designated terminal capability emits completion. In a Coder ↔ Reviewer cycle, Coder implements and routes to Reviewer; Reviewer returns revision feedback to Coder; Coder revises and routes back; Reviewer completes after approval. A Run fails after 24 executions while retaining history, and termination marks remaining deliveries skipped. A Team without Tester must not claim Tester acceptance.

The default and previously saved Teams retain legacy consultation semantics and are labeled **Legacy Consultation**. **Copy as Message-Flow Team** creates an unsaved copy that requires new entry and terminal choices and does not modify the original. Legacy edges still mean downstream read-only consultation approval with revision feedback to upstream; do not interpret them as message flow.

## Create the first Run

1. Click **New Run**.
2. Click **Select** and choose a local Git repository with at least one commit.
3. Enter a goal that clearly states the intended implementation and verification outcome.
4. Select the Team and click **Start Real Run**. The Run freezes only its selected capabilities, topology, and Agent bindings. If Tester is present, it chooses verification from the actual goal, design, implementation, ChangeSet, and workspace evidence available.

The workbench then shows:

- Run state, repository, frozen Team, and Run ID at the top;
- the current execution stage for each capability;
- the Agent instance frozen for each capability node;
- the persisted SQLite event timeline;
- node results and related events; message-flow Teams show each delivery and route while legacy Teams show consultation progress; frozen Team edges remain visible;
- **Workspace**, which reveals the isolated worktree in Finder;
- **Cancel**, which terminates active execution and persists `cancelled` state.

## Close and recovery

Closing the window while Runs started by this client are active asks whether to cancel them. Continuing the close sends cancellation before exit.

Archmage does not continue scheduling after the client exits. After a crash or forced exit, the next start terminates ownerless main-task or consultation processes and marks affected Runs `interrupted`. Graphical resume is not available.

## Use with the CLI

Inspect a desktop-created Run:

```bash
archmage status <run-id> --json
archmage logs <run-id>
```

Cancel from another terminal:

```bash
archmage cancel <run-id>
```

If the CLI cannot find a desktop Run, the two processes usually use different `ARCHMAGE_HOME` values.

## Quick verification

Frontend tests and production build:

```bash
npm test --prefix apps/archmage-desktop
npm run build --prefix apps/archmage-desktop
```

Rust/Tauri compilation check:

```bash
cargo check -p archmage-desktop
```

See the [CLI command reference](../cli-reference.md) for complete command documentation.
