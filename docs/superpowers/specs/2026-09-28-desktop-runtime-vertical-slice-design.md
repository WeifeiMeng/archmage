# Desktop Runtime Vertical Slice Design

**Status:** Approved for implementation planning

## Goal

Deliver the first genuinely runnable Archmage product slice: a Tauri desktop workbench that selects a local Git repository, accepts a goal and required checks, runs the fixed Designer → Coder → Tester → Reviewer workflow through the real local Codex CLI, and presents durable progress and results without requiring users to operate the CLI.

## Scope

The slice includes:

- one real Codex provider shared by four logical Capability roles;
- one Run containing all four Node Runs, not one Run per node;
- a fixed linear workflow for S0;
- real per-Run Git worktree isolation;
- explicit Context Snapshots, Handoffs, contracts, artifacts, check evidence, and events;
- a Tauri 2 desktop application using React, TypeScript, and Vite;
- repository selection, goal/check input, environment preflight, Run start, live timeline, node inspection, cancellation, historical Run inspection, and workspace reveal;
- interruption reconciliation when the application is restarted.

The slice does not include a background daemon, system tray, Workflow editor, Provider settings UI, Fix loop, automatic retry, automatic commit/merge/push, signed packaging, or multiple provider types.

## Runtime architecture

`archmage-core` remains independent of Tauri and the CLI. It exposes a small application-facing Runtime facade:

```text
RunRequest
  repo: absolute path
  goal: nonblank string
  checks: one or more frozen shell commands

Runtime
  preflight(request)
  start_run(request) -> run_id
  run_to_terminal(run_id)
  snapshot(run_id) -> RunSnapshot
  events(run_id, after_seq) -> Event[]
  cancel(run_id)
  reconcile_interrupted()
```

`start_run` validates input, persists the Run definition, creates the Run worktree, and returns only after the durable Run identity exists. `run_to_terminal` owns execution and is safe to place on an application-managed Tokio task.

The scheduler derives the Ready Set from persisted node state and fixed dependencies. The initial graph is linear, but scheduling is state-based rather than a hard-coded function chain so later conditional and parallel nodes do not require replacing the Runtime boundary. Workspace access leases and Provider capacity remain enforced by Core.

Each node execution performs this sequence:

1. claim an Attempt under the current Run owner generation;
2. resolve and persist the immutable Context Snapshot;
3. render the role prompt and invoke a fresh real `codex exec` turn;
4. persist streamed provider diagnostics and thread identity;
5. validate the role JSON against its schema and business invariants;
6. publish the human-readable output and structured result as artifacts;
7. record Handoffs and atomically complete the Attempt and Node Run;
8. emit committed events used by both CLI and desktop clients.

The Tester is the only role that can request frozen validation checks through the attempt-specific Unix socket. The Runtime launches checks under the existing restricted Codex sandbox and owns canonical evidence. The Reviewer receives ChangeSet and TestResult artifacts; failed required checks cannot become an approved result.

## Desktop architecture

Add an `apps/archmage-desktop` application:

```text
React Workbench
    │ invoke/listen
    ▼
Tauri command layer
    │ typed DTO conversion only
    ▼
archmage-core Runtime
    ├── Codex CLI
    ├── Git/worktrees
    └── SQLite/artifacts
```

Tauri owns an application `AppState` containing the Runtime handle, active Run task handles, and event forwarding subscriptions. React never opens SQLite, invokes Git/Codex directly, or parses CLI stdout.

The initial Tauri command surface is:

- `doctor(online: bool)`
- `start_run(request)`
- `list_runs()`
- `get_run_snapshot(run_id)`
- `list_events(run_id, after_seq)`
- `cancel_run(run_id)`
- `reveal_workspace(run_id)`

Committed Core events are forwarded as `run-event` Tauri events containing `run_id` and `event_seq`. React treats the event as an invalidation/append signal. On initial load, refresh, sequence gap, or listener reconnect, it calls `get_run_snapshot` and `list_events`; correctness never depends on receiving every in-memory event.

## User experience

The first layout uses the approved workbench-first option A:

- left sidebar: New Run and recent Runs;
- main header: repository, goal summary, Run state, Run ID, worktree, Cancel and Reveal Workspace actions;
- new Run card: repository picker, goal textarea, repeatable check command fields, and environment status;
- four-node progress strip: Designer, Coder, Tester, Reviewer;
- timeline: ordered durable Runtime events and concise provider progress;
- node inspector: Context, result, logs, artifacts, ChangeSet, and check evidence.

The initial visual system is intentionally replaceable. Components are divided by product responsibility rather than pixel layout so a later user-supplied prototype can change navigation and appearance without changing Tauri commands or Runtime DTOs.

The client presents `doctor` as “环境检查”. Offline checks run automatically before enabling Start. The real online Codex probe is explicit because it consumes model usage.

## Lifecycle and error semantics

Before a Run exists, repository, goal, checks, Git, Codex executable, storage, and schemas must pass preflight. Failure remains on the form and does not create a partial Run.

After a Run is durably created, failures become persisted events and terminal node/Run states. The client does not infer success from process exit or optimistic UI state. It renders the Runtime snapshot.

Cancellation is a durable control request. Runtime validates owned process identity, terminates the relevant Codex/check process group, and records cancellation before the UI reports it as terminal.

The first desktop slice does not continue after the window closes. Closing with an active Run prompts the user to return to the app or cancel and exit. If the process is forcibly terminated, the next launch reconciles the old Run as interrupted; it does not claim background execution. Historical terminal Runs remain inspectable after restart.

## Testing strategy

Testing stays deliberately small:

- reuse existing Store, worktree, ownership, contract, context, and check tests;
- add focused Core tests for Ready Set progression, one complete persisted lifecycle, cancellation, and interrupted reconciliation;
- add small TypeScript tests only for Run form serialization and event-state reduction;
- use `npm run build` and Rust `cargo check` as the primary desktop compile gates;
- keep one ignored, explicit real-Codex end-to-end acceptance against a temporary fixture repository;
- manually launch the Tauri development app once and verify repository selection, Run progress, final state, restart history, and workspace reveal.

No Fake Agent is shipped or accepted as product evidence. Deterministic unit tests may exercise pure scheduling/state functions without pretending that a simulated Provider proves the real integration. The final vertical acceptance always uses the authenticated local Codex CLI.

## Delivery order

1. Expose the minimal Runtime facade and fixed workflow scheduling.
2. Complete real Codex capability invocation and node artifact/Handoff publication.
3. Add cancellation, interruption reconciliation, snapshots, and event queries.
4. Scaffold Tauri/React and typed commands.
5. Build the workbench-first UI and reconnect behavior.
6. Run the real Codex vertical acceptance and update the user guide.

The first usable checkpoint is not a visual shell: it must start a real Run from the desktop and make durable progress through the same Core Runtime used by the CLI.
