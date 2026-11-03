# Desktop Runtime Vertical Slice Implementation Plan

[简体中文](../../../zh-CN/superpowers/plans/2026-09-28-desktop-runtime-vertical-slice.md) | English

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the four-capability Runtime main chain driven by real local Codex, and deliver a Tauri desktop client that can start, observe, cancel, and review historical Runs.

**Architecture:** `archmage-core` adds UI-independent Runtime facade, status query and fixed workflow scheduling; both CLI and Tauri only call this set of APIs. The desktop uses Tauri 2, React, TypeScript and Vite, uses typed commands to read durable snapshot/events, and real-time events are only used as refresh signals.

**Tech Stack:** Rust 1.95, Tokio, SQLite/rusqlite, real Codex CLI, Git worktree, Tauri 2, React 19, TypeScript, Vite, Vitest.

---

## implementation agreement

- Implemented in a standalone `codex/desktop-runtime-slice` worktree and does not modify `main` directly.
- No Fake Agent is added; final acceptance must call the logged-in real Codex CLI.
- A single Run always contains four Node Runs: `designer`, `coder`, `tester`, and `reviewer`.
- S0's dependencies are linear, but Ready Set is calculated from the node state and dependency table and is not written as four hard-coded calls.
- Keep tests small: only add one focused test per behavior; front-end only tests form serialization and event reducers.
- The runtime is responsible for persistence, Git, Codex, checking, and cancellation; React does not have direct access to SQLite, Git, or Codex.
- Does not implement daemon, auto-retry, Fix loop, Workflow editor, Provider settings, or auto-commit/merge.

## File layout

```text
crates/archmage-core/
  migrations/0006_runtime.sql          # Run definition、结果 artifact 与取消字段
  src/provider/codex.rs                # 通用 schema-bound Codex invocation
  src/runtime/mod.rs                   # Runtime facade 与 DTO
  src/runtime/scheduler.rs             # Ready Set 与终态归并
  src/runtime/dispatch.rs              # 单 Node 上下文、Codex、artifact、handoff
  src/runtime/recovery.rs              # 强退后的 interrupted 对账
  src/store/mod.rs                     # snapshot/list/query DTO 查询
  src/store/transitions.rs             # Runtime 原子状态迁移
  tests/runtime.rs                     # 小型 persisted lifecycle/Ready Set 测试
  tests/runtime_live.rs                # ignored 真实 Codex 四能力验收
crates/archmage-cli/
  src/main.rs                          # run/status/logs/cancel 接线
  src/output.rs                        # 人类可读及 JSON 输出
  tests/cli.rs                         # 非网络 CLI 查询测试
apps/archmage-desktop/
  package.json                         # 前端与 Tauri scripts
  src-tauri/Cargo.toml                 # Tauri shell，依赖 archmage-core
  src-tauri/src/lib.rs                 # AppState、commands、event forwarding
  src-tauri/tauri.conf.json            # Vite/Tauri 配置
  src-tauri/capabilities/default.json  # 最小 dialog/opener 权限
  src/api.ts                           # typed invoke/listen boundary
  src/model.ts                         # Runtime DTO 与 reducer
  src/App.tsx                          # workbench composition
  src/components/                      # RunForm/Sidebar/Progress/Timeline/Inspector
  src/styles.css                       # option A visual system
  src/model.test.ts                    # reducer/form tests
docs/cli-reference.md                  # 可复制的 run/status/logs/cancel 命令
docs/runbooks/desktop-s0.md            # 桌面启动与真实验收步骤
```

### Task 1: Runtime data contract and persistent query

**Files:**
- Create: `crates/archmage-core/migrations/0006_runtime.sql`
- Create: `crates/archmage-core/tests/runtime.rs`
- Modify: `crates/archmage-core/src/store/mod.rs`
- Modify: `crates/archmage-core/src/store/transitions.rs`
- Modify: `crates/archmage-core/src/runtime/mod.rs`

- [ ] **Step 1: Write failing tests**

Build a temporary store in `tests/runtime.rs`, use `RunRequest` to create Run and then assert definition to save repo/goal/checks. The order of the four nodes is fixed. `list_runs()` and `snapshot()` return the same Run.

```rust
#[test]
fn start_definition_is_queryable_as_one_four_node_run() {
    let fixture = RuntimeFixture::new();
    let request = RunRequest::new(fixture.repo(), "add health endpoint", vec![
        CheckSpec::new("check-1", "cargo test"),
    ]).unwrap();
    let run_id = fixture.runtime.create_run_definition(&request).unwrap();
    let snapshot = fixture.runtime.snapshot(&run_id).unwrap();
    assert_eq!(snapshot.goal, "add health endpoint");
    assert_eq!(snapshot.nodes.iter().map(|n| n.key.as_str()).collect::<Vec<_>>(),
               ["designer", "coder", "tester", "reviewer"]);
    assert_eq!(snapshot.checks[0].command, "cargo test");
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test -p archmage-core --test runtime start_definition_is_queryable_as_one_four_node_run`

Expected: FAIL because `RunRequest`, runtime query, and v6 migration do not exist yet.

- [ ] **Step 3: Implement minimal contract**

Add serializable `RunRequest`, `CheckSpec`, `RunSummary`, `RunSnapshot`, `NodeSnapshot`; v6 adds the required fields for repository/definition for `runs` and for attempts/artifactsAdd the required fields for the result reference query.`Store::list_runs` Press `created_at DESC`, `Store::run_snapshot` assembles Run, Node, workspace and latest event sequence at once.

- [ ] **Step 4: Run GREEN and submit**

Run: `cargo test -p archmage-core --test runtime`

Expected: PASS。

Commit: `feat: add runtime run snapshots`

### Task 2: Universal real Codex Invocation

**Files:**
- Modify: `crates/archmage-core/src/provider/codex.rs`
- Modify: `crates/archmage-core/src/provider/mod.rs`
- Modify: `crates/archmage-core/src/process.rs`
- Test: `crates/archmage-core/tests/codex_protocol.rs`

- [ ] **Step 1: Write failing tests**

Test `CodexInvocation::argv()` generates explicit `--cd`, `--sandbox`, `--json`, `--output-schema`, `--output-last-message` for designer/read-only and coder/workspace-write, with prompt only passed in from stdin.

```rust
#[test]
fn invocation_builds_schema_bound_argv_without_prompt_text() {
    let invocation = CodexInvocation::new(repo(), schema(), result(), Sandbox::WorkspaceWrite);
    let argv = invocation.argv();
    assert!(argv.windows(2).any(|v| v == ["--sandbox", "workspace-write"]));
    assert!(!argv.iter().any(|part| part.contains("secret goal")));
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test -p archmage-core --test codex_protocol invocation_builds_schema_bound_argv_without_prompt_text`

Expected: FAIL because the generic invocation type does not exist.

- [ ] **Step 3: Implement invocation**

Extract `CodexInvocation`, `CodexOutcome` and `invoke()`; keep the complete JSONL/stderr/result file, limit the size and deadline, and require exit code 0, `turn.completed`, thread ID and legal result files to be established at the same time. The original text of stderr must not be spelled into user error.

- [ ] **Step 4: Run GREEN and submit**

Run: `cargo test -p archmage-core --test codex_protocol`

Expected: PASS。

Commit: `feat: generalize real Codex invocation`

### Task 3: Ready Set and Run final state

**Files:**
- Create: `crates/archmage-core/src/runtime/scheduler.rs`
- Modify: `crates/archmage-core/src/runtime/mod.rs`
- Modify: `crates/archmage-core/src/store/transitions.rs`
- Test: `crates/archmage-core/tests/runtime.rs`

- [ ] **Step 1: Write failing tests**

Assert the status of the four nodes respectively: initially only designer ready; after designer succeeded, only coder ready; failed/cancelled will no longer dispatch successors; after reviewer succeeded, Run completed.

```rust
#[test]
fn ready_set_advances_from_persisted_dependencies() {
    assert_eq!(ready_keys(&states("pending,pending,pending,pending")), ["designer"]);
    assert_eq!(ready_keys(&states("succeeded,pending,pending,pending")), ["coder"]);
    assert!(ready_keys(&states("failed,pending,pending,pending")).is_empty());
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test -p archmage-core --test runtime ready_set_advances_from_persisted_dependencies`

Expected: FAIL because scheduler does not exist.

- [ ] **Step 3: Implement Ready Set**

Define a fixed dependency table and calculate the Ready Set from the snapshot; add transaction methods `mark_ready`, `complete_run`, `fail_run`, `cancel_run`. Each status change is submitted with the same transaction as the event.

- [ ] **Step 4: Run GREEN and submit**

Run: `cargo test -p archmage-core --test runtime`

Expected: PASS。

Commit: `feat: schedule persisted workflow nodes`

### Task 4: Four abilities real Dispatch and Handoff

**Files:**
- Create: `crates/archmage-core/src/runtime/dispatch.rs`
- Modify: `crates/archmage-core/src/runtime/mod.rs`
- Modify: `crates/archmage-core/src/context.rs`
- Modify: `crates/archmage-core/src/artifact.rs`
- Modify: `crates/archmage-core/src/check_runner.rs`
- Test: `crates/archmage-core/tests/runtime.rs`

- [ ] **Step 1: Write failing tests**

Use the submitted artifact and real SQLite, do not start the Provider, construct the inputs of the four nodes respectively, and assert that the required context keys, handoff producer, Tester check policy and Reviewer changeset/test result are complete.

```rust
#[test]
fn reviewer_context_uses_canonical_changeset_and_test_evidence() {
    let context = fixture.build_context(CapabilityKind::Reviewer).unwrap();
    assert_eq!(context.keys(), ["goal", "design", "implementation", "changeset", "test_result"]);
    assert!(context.handoffs.iter().all(|handoff| !handoff.content_hash.is_empty()));
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test -p archmage-core --test runtime reviewer_context_uses_canonical_changeset_and_test_evidence`

Expected: FAIL because dispatch/context builder does not exist yet.

- [ ] **Step 3: Implement single node execution**

`dispatch_node` completes begin attempt, context persist/bind, schema write to attempt directory, real Codex invocation, contract verification, result artifact release and attempt completion. Designer/Reviewer uses read-only; Coder/Tester uses worktree-write. Tester starts the attempt-specific check socket, and only `archmage internal check` can request a frozen check; Reviewer inputs ChangeSet and canonical check evidence published using Runtime.

- [ ] **Step 4: Access `run_to_terminal`**

Cyclically read snapshot → calculate Ready Set → dispatch; when the node fails, Run will be marked as failed and will not be automatically retried. Reviewer `approved` corresponds to completed/approved, `changes_requested` corresponds to completed/changes_requested.

- [ ] **Step 5: Run GREEN and submit**

Run: `cargo test -p archmage-core --test runtime --test context --test contracts --test checks`

Expected: PASS。

Commit: `feat: execute four capability workflow`

### Task 5: Cancel, restore and CLI main chain

**Files:**
- Create: `crates/archmage-core/src/runtime/recovery.rs`
- Modify: `crates/archmage-core/src/runtime/control.rs`
- Modify: `crates/archmage-core/src/runtime/mod.rs`
- Modify: `crates/archmage-core/src/store/recovery.rs`
- Modify: `crates/archmage-cli/src/main.rs`
- Modify: `crates/archmage-cli/src/output.rs`
- Modify: `crates/archmage-cli/tests/cli.rs`
- Test: `crates/archmage-core/tests/runtime.rs`

- [ ] **Step 1: Write failing tests**

The test cancel request is a durable event; when an active attempt is found at startup but the owner lock has no holder, it is marked attempt/node/run interrupted; status/logs is read-only and will not change the state without authorization.

```rust
#[test]
fn reconciliation_marks_abandoned_run_interrupted() {
    let run = fixture.running_attempt();
    fixture.runtime.reconcile_interrupted().unwrap();
    assert_eq!(fixture.runtime.snapshot(&run).unwrap().state, "interrupted");
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test -p archmage-core --test runtime reconciliation_marks_abandoned_run_interrupted`

Expected: FAIL because runtime recovery does not exist.

- [ ] **Step 3: Implement control/recovery and CLI**

`Runtime::cancel` persists the request and terminates the owned process group; `reconcile_interrupted` only writes interrupted when the Run owner lock can be obtained and the process identity is no longer alive. Wiring `archmage run/status/logs/cancel`, `run` waits for the terminal, `status --json` outputs snapshot, `logs` outputs events.

- [ ] **Step 4: Run GREEN and submit**

Run: `cargo test -p archmage-core --test runtime && cargo test -p archmage-cli --test cli`

Expected: PASS。

Commit: `feat: expose runtime through CLI`

### Task 6: Tauri 2 + React desktop skeleton

**Files:**
- Create: `apps/archmage-desktop/package.json`
- Create: `apps/archmage-desktop/package-lock.json`
- Create: `apps/archmage-desktop/index.html`
- Create: `apps/archmage-desktop/tsconfig.json`
- Create: `apps/archmage-desktop/vite.config.ts`
- Create: `apps/archmage-desktop/src-tauri/Cargo.toml`
- Create: `apps/archmage-desktop/src-tauri/src/{main,lib}.rs`
- Create: `apps/archmage-desktop/src-tauri/{build.rs,tauri.conf.json}`
- Create: `apps/archmage-desktop/src-tauri/capabilities/default.json`
- Modify: `Cargo.toml`

- [ ] **Step 1: Generate and tighten the official Tauri 2 React/TS/Vite structure**

Using the current official template structure, the workspace added `apps/archmage-desktop/src-tauri`. Only core commands, dialog folder picker and opener are enabled; shell plugin is not enabled to prevent the front end from directly executing arbitrary commands.

- [ ] **Step 2: Implement typed Tauri commands**

`AppState` Save Runtime and `run_id -> JoinHandle`. Command implementation `doctor`, `start_run`, `list_runs`, `get_run_snapshot`, `list_events`, `cancel_run`, `reveal_workspace`; `start_run` spawn after creating durable Run`run_to_terminal`, send only run_id/event_seq via `run-event`.

- [ ] **Step 3: Compile and submit**

Run: `npm install --prefix apps/archmage-desktop && npm run build --prefix apps/archmage-desktop && cargo check -p archmage-desktop`

Expected: TypeScript/Vite build and Tauri Rust check both succeeded.

Commit: `feat: scaffold Tauri runtime client`

### Task 7: Workbench-first client

**Files:**
- Create: `apps/archmage-desktop/src/{main.tsx,api.ts,model.ts,App.tsx,styles.css}`
- Create: `apps/archmage-desktop/src/components/{RunForm,RunSidebar,RunHeader,NodeProgress,Timeline,NodeInspector}.tsx`
- Create: `apps/archmage-desktop/src/model.test.ts`

- [ ] **Step 1: Write two failing tests**

The form test serializes repository/goal/check rows into `RunRequest` and filters empty checks; the reducer test deduplicates event_seq, detects sequence gaps, and requires snapshot refresh.

```ts
it("serializes nonblank checks", () => {
  expect(toRunRequest("/repo", "goal", ["npm test", " "]).checks)
    .toEqual([{ checkId: "check-1", command: "npm test" }]);
});

it("requests refresh on an event gap", () => {
  expect(reduceEvent(stateAt(4), eventAt(6)).needsRefresh).toBe(true);
});
```

- [ ] **Step 2: Run RED**

Run: `npm test --prefix apps/archmage-desktop`

Expected: FAIL because model/reducer does not exist yet.

- [ ] **Step 3: Implement data layer and layout A**

Create typed invoke wrappers; `list_runs` at startup, load snapshot+events when Run is selected, and incrementally refresh after listening to `run-event`. Implement the left Run list, new Run form, four-node progress, timeline, inspector, and Cancel/Reveal Workspace. The status color and copy are expressed at the same time, not just relying on color.

- [ ] **Step 4: Implement window closing protection**

Prevent close when active Run exists, and use native confirmation box to provide "return to application" or "cancel and exit"; do not declare background continuation after close.

- [ ] **Step 5: Run GREEN and submit**

Run: `npm test --prefix apps/archmage-desktop && npm run build --prefix apps/archmage-desktop`

Expected: PASS。

Commit: `feat: build desktop run workbench`

### Task 8: Real Codex acceptance and usage documents

**Files:**
- Create: `crates/archmage-core/tests/runtime_live.rs`
- Create: `docs/runbooks/desktop-s0.md`
- Modify: `docs/cli-reference.md`
- Modify: `README.md`

- [ ] **Step 1: Add ignored true experiment collection**

The fixture repo only contains a small task that can be modified quickly and a check command. The test calls the public Runtime API and asserts that the four nodes all generate different real Codex thread IDs, Run terminal, events are continuous, the worktree is checkable, and the source repo HEAD/status/content remains unchanged.

- [ ] **Step 2: Run the real Core/CLI for acceptance**

Run: `cargo test -p archmage-core --test runtime_live -- --ignored --nocapture`

Expected: The real Codex completes four capability invocations; if Reviewer changes_requested, the process is still a legal terminal, but it must not be disguised as approved.

- [ ] **Step 3: Start manual smoke test on desktop**

Run: `npm run tauri dev --prefix apps/archmage-desktop`

Verification: Select repo, enter target/check, start real run, observe four nodes/timeline, view historical run, Reveal Workspace. Only do this round of desktop manual verification.

- [ ] **Step 4: Update the document and make the final lightweight access control**

The document gives reproducible commands for `archmage run/status/logs/cancel` and `npm run tauri dev --prefix apps/archmage-desktop`, and makes it clear that `resume` does not yet provide automatic repair semantics.

Run: `cargo fmt --all --check && cargo test --workspace && npm test --prefix apps/archmage-desktop && npm run build --prefix apps/archmage-desktop`

Expected: All non-ignored checks pass.

Commit: `docs: add runtime and desktop guide`
