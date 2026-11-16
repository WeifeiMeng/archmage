# Host-Managed Branches and Blocked Flow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move branch creation into Archmage orchestration and persist valid blocked Capability results as a first-class terminal Flow state.

**Architecture:** Freeze an optional target branch in each Run definition and let `workspace::provision` create it before any Provider starts. Inject the actual persisted branch into Provider context and forbid branch manipulation. Extend Flow contracts and SQLite states so a blocked Coder/Tester result atomically persists artifacts while terminating the Run as blocked.

**Tech Stack:** Rust 2024, SQLite/rusqlite migrations, serde/serde_json, Git worktrees, React 19, TypeScript, Vitest.

---

## File structure

- Modify `crates/archmage-core/src/runtime/mod.rs`: RunRequest/FrozenRunDefinition target branch and provisioning call.
- Modify `crates/archmage-core/src/workspace/mod.rs` and `workspace/git.rs`: validate and create requested branches without shell interpolation.
- Modify `crates/archmage-core/src/context.rs` and `runtime/flow.rs`: inject host-managed branch constraints.
- Modify `crates/archmage-core/src/contract.rs`: add and validate `routing.decision = blocked`.
- Create `crates/archmage-core/migrations/0018_blocked_flows.sql`: add blocked Run/Node/Delivery states.
- Modify `crates/archmage-core/src/store/mod.rs` and `store/flow.rs`: migrate and atomically persist blocked delivery results.
- Modify `apps/archmage-desktop/src/model.ts`, `App.tsx`, `flowDelivery.ts`, and CSS: target-branch input and blocked labels.
- Modify Rust and desktop tests named below for TDD and regression coverage.

### Task 1: Freeze and provision a host-managed target branch

**Files:**
- Modify: `crates/archmage-core/src/runtime/mod.rs`
- Modify: `crates/archmage-core/src/workspace/mod.rs`
- Test: `crates/archmage-core/tests/runtime.rs`
- Test: `crates/archmage-core/tests/workspace.rs`

- [ ] **Step 1: Write failing tests**

Add a runtime serialization test asserting `RunRequest::new(Path::new("/repo"), "goal").unwrap().with_target_branch("docs/topic")` serializes as `targetBranch`, and a workspace integration test calling the wished-for signature:

```rust
let workspace = provision(&mut store, &run, source.path(), root.path(), Some("docs/topic")).unwrap();
assert_eq!(workspace.branch, "docs/topic");
assert_eq!(git_output(&workspace.path, &["branch", "--show-current"]), "docs/topic\n");
let invalid_run = store.create_run("invalid", &["coder"]).unwrap();
assert!(provision(&mut store, &invalid_run, source.path(), root.path(), Some("bad branch"))
    .unwrap_err().to_string().contains("invalid target branch"));
```

Also verify an existing branch is rejected before worktree creation and `None` still produces `archmage/run-<id>`.

- [ ] **Step 2: Verify RED**

Run: `cargo test -p archmage-core --test runtime target_branch -- --nocapture && cargo test -p archmage-core --test workspace target_branch -- --nocapture`

Expected: compilation fails because `with_target_branch` and the new `provision` parameter do not exist.

- [ ] **Step 3: Implement the minimal branch API**

Add `#[serde(default)] pub target_branch: Option<String>` to `RunRequest` and `FrozenRunDefinition`, normalize blank input to `None`, freeze it with definition version 7, and pass the frozen value to provisioning. In `workspace::provision`, select the explicit branch or the automatic branch, validate explicit values with direct Git argv equivalent to:

```text
git check-ref-format --branch <branch>
git show-ref --verify --quiet refs/heads/<branch>
```

Treat a zero `show-ref` exit as a conflict and a nonzero exit as absent; do not use a shell. Persist the chosen branch in the existing workspace intent before `git worktree add -b`.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test -p archmage-core --test runtime target_branch -- --nocapture && cargo test -p archmage-core --test workspace -- --nocapture`

Expected: all selected tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/archmage-core/src/runtime/mod.rs crates/archmage-core/src/workspace/mod.rs crates/archmage-core/tests/runtime.rs crates/archmage-core/tests/workspace.rs
git commit -m "feat: provision host-managed target branches"
```

### Task 2: Inject the actual branch into Capability context

**Files:**
- Modify: `crates/archmage-core/src/runtime/flow.rs`
- Modify: `crates/archmage-core/src/runtime/dispatch.rs`
- Test: `crates/archmage-core/tests/context.rs`
- Test: `crates/archmage-core/tests/runtime.rs`

- [ ] **Step 1: Write failing prompt tests**

Exercise a persisted workspace with branch `docs/topic` and assert rendered execution context includes all of:

```text
Current Archmage-managed branch: docs/topic
Do not create, switch, rename, or delete Git branches.
Treat branch preparation requested by the user as already completed by Archmage.
```

Assert it does not contain a writable Git common-dir path.

- [ ] **Step 2: Verify RED**

Run: `cargo test -p archmage-core --test context managed_branch -- --nocapture`

Expected: assertion fails because current prompts do not describe branch ownership.

- [ ] **Step 3: Implement one shared instruction builder**

Add a small helper in runtime dispatch code:

```rust
fn managed_branch_instructions(branch: &str) -> String {
    format!("Current Archmage-managed branch: {branch}\nDo not create, switch, rename, or delete Git branches or modify worktree Git metadata. Treat branch preparation requested by the user as already completed by Archmage.")
}
```

Use the persisted `WorkspaceRecord.branch` in both legacy dispatch and Flow role context so every Provider receives the same platform boundary.

- [ ] **Step 4: Verify GREEN and commit**

Run: `cargo test -p archmage-core --test context managed_branch -- --nocapture && cargo test -p archmage-core --test runtime -- --nocapture`

```bash
git add crates/archmage-core/src/runtime/dispatch.rs crates/archmage-core/src/runtime/flow.rs crates/archmage-core/tests/context.rs crates/archmage-core/tests/runtime.rs
git commit -m "feat: tell agents about managed branches"
```

### Task 3: Define a valid blocked Flow contract

**Files:**
- Modify: `crates/archmage-core/src/contract.rs`
- Test: `crates/archmage-core/tests/contracts.rs`

- [ ] **Step 1: Write failing contract tests**

Add table-driven cases proving:

```rust
// accepted
status="blocked", decision="blocked", next=[]
// rejected
status="blocked", decision="continue", next=["reviewer"]
status="implemented", decision="blocked", next=[]
status="blocked", decision="blocked", next=["reviewer"]
```

Cover both Coder and Tester blocked schemas; Designer/Reviewer cannot use blocked routing.

- [ ] **Step 2: Verify RED**

Run: `cargo test -p archmage-core --test contracts blocked_flow -- --nocapture`

Expected: valid blocked envelopes fail because the routing schema and validator only allow continue/complete.

- [ ] **Step 3: Implement contract validation**

Add `blocked` to `flow_output_schema`. Validate status/decision symmetry before graph routing checks. `blocked` requires an empty recipient list and bypasses terminal-capability completion rules; remove the blanket “blocked coder output cannot route a flow” rejection.

- [ ] **Step 4: Verify GREEN and commit**

Run: `cargo test -p archmage-core --test contracts -- --nocapture`

```bash
git add crates/archmage-core/src/contract.rs crates/archmage-core/tests/contracts.rs
git commit -m "feat: define blocked flow routing"
```

### Task 4: Persist blocked Flow terminal state atomically

**Files:**
- Create: `crates/archmage-core/migrations/0018_blocked_flows.sql`
- Modify: `crates/archmage-core/src/store/mod.rs`
- Modify: `crates/archmage-core/src/store/flow.rs`
- Modify: `crates/archmage-core/src/runtime/flow.rs`
- Test: `crates/archmage-core/tests/persistence.rs`
- Test: `crates/archmage-core/tests/flow.rs`
- Test: `crates/archmage-core/tests/runtime.rs`

- [ ] **Step 1: Write failing migration and transition tests**

Add a schema-17 upgrade test that inserts representative Run/Node/Delivery rows, opens Store, asserts version 18 and preserved data, then verifies `blocked` states are accepted. Add a real Flow test whose Coder result is blocked and assert:

```rust
assert_eq!(snapshot.state, "blocked");
assert_eq!(snapshot.outcome.as_deref(), Some("coder_blocked"));
assert_eq!(coder.state, "blocked");
assert_eq!(delivery.state, "blocked");
assert_eq!(attempt.state, "succeeded");
assert!(coder.result.is_some());
assert!(events.iter().any(|e| e.event_type == "workflow.blocked"));
assert!(!events.iter().any(|e| e.event_type == "workflow.failed"));
```

- [ ] **Step 2: Verify RED**

Run: `cargo test -p archmage-core --test persistence blocked -- --nocapture && cargo test -p archmage-core --test flow blocked -- --nocapture`

Expected: migration/state constraints and transition APIs are missing.

- [ ] **Step 3: Add migration 18**

Rebuild `runs`, `node_runs`, and `flow_deliveries` using the exact current columns, foreign keys, and indexes while adding `blocked` to their state CHECK sets. Because multiple tables reference runs and nodes, refactor `Store::open` so the version-17-to-18 migration temporarily disables foreign-key enforcement before opening its immediate migration transaction, copies and swaps the three tables, runs `PRAGMA foreign_key_check`, commits only when it is empty, and re-enables foreign keys before returning. Update the fresh-database path, migration chain, health invariant, and expected version to 18.

- [ ] **Step 4: Add atomic blocked transition**

Implement `Store::finish_blocked_flow_delivery` in one immediate transaction. It must verify run ownership, running delivery/attempt, and attempt revision; persist output, thread, Result/ChangeSet artifact IDs; set Attempt succeeded; set Node/Delivery/Run blocked; set Run outcome to `<target>_blocked`; skip queued deliveries with the existing terminal skip state; and emit `flow.delivery_blocked`, `node.blocked`, and `workflow.blocked`.

In `execute_flow_attempt`, publish artifacts normally and call this transition when `routing.decision == "blocked"`. In `run_flow_to_terminal`, return a blocked snapshot as a normal terminal result.

- [ ] **Step 5: Verify GREEN and commit**

Run: `cargo test -p archmage-core --test persistence blocked -- --nocapture && cargo test -p archmage-core --test flow -- --nocapture && cargo test -p archmage-core --test runtime -- --nocapture`

```bash
git add crates/archmage-core/migrations/0018_blocked_flows.sql crates/archmage-core/src/store/mod.rs crates/archmage-core/src/store/flow.rs crates/archmage-core/src/runtime/flow.rs crates/archmage-core/tests/persistence.rs crates/archmage-core/tests/flow.rs crates/archmage-core/tests/runtime.rs
git commit -m "feat: persist blocked flow results"
```

### Task 5: Add target branch input and blocked UI states

**Files:**
- Modify: `apps/archmage-desktop/src/model.ts`
- Modify: `apps/archmage-desktop/src/model.test.ts`
- Modify: `apps/archmage-desktop/src/App.tsx`
- Modify: `apps/archmage-desktop/src/flowDelivery.ts`
- Modify: `apps/archmage-desktop/src/flowDelivery.test.ts`
- Modify: `apps/archmage-desktop/src/AgentLiveOutput.tsx`
- Modify: `apps/archmage-desktop/src/newRun.css`
- Test: `apps/archmage-desktop/src/RunWorkbench.test.tsx`

- [ ] **Step 1: Write failing desktop tests**

Change the desired request API to:

```ts
expect(toRunRequest(" /repo ", " goal ", "team-1", " docs/topic ")).toEqual({
  repository: "/repo", goal: "goal", teamId: "team-1", targetBranch: "docs/topic",
});
expect(toRunRequest("/repo", "goal", "team-1", "   ")).not.toHaveProperty("targetBranch");
```

Assert `stateLabel`, delivery label/direction, Agent output, and RunWorkbench render “已阻塞”/“Coder 已阻塞” while preserving structured results.

- [ ] **Step 2: Verify RED**

Run: `npm test -- --run src/model.test.ts src/flowDelivery.test.ts src/RunWorkbench.test.tsx`

Expected: request and blocked labels do not exist.

- [ ] **Step 3: Implement UI behavior**

Add optional `targetBranch` to `RunRequest`, new state and input in App, explanatory copy, and request memo dependency. Map `blocked` consistently in Run/Node/Delivery labels and active-state logic. Keep blocked terminal, so polling stops. Reuse existing result rendering for blocked nodes.

- [ ] **Step 4: Verify GREEN and commit**

Run: `npm test -- --run src/model.test.ts src/flowDelivery.test.ts src/RunWorkbench.test.tsx && npm run build`

```bash
git add apps/archmage-desktop/src/model.ts apps/archmage-desktop/src/model.test.ts apps/archmage-desktop/src/App.tsx apps/archmage-desktop/src/flowDelivery.ts apps/archmage-desktop/src/flowDelivery.test.ts apps/archmage-desktop/src/AgentLiveOutput.tsx apps/archmage-desktop/src/newRun.css apps/archmage-desktop/src/RunWorkbench.test.tsx
git commit -m "feat: expose managed branches and blocked runs"
```

### Task 6: End-to-end regression and documentation parity

**Files:**
- Modify if necessary: `docs/en/runbooks/*`, `docs/zh-CN/runbooks/*`
- Test: all touched suites

- [ ] **Step 1: Run full verification**

```bash
cargo fmt --check
cargo test -p archmage-core
cd apps/archmage-desktop && npm test && npm run build
git diff --check
```

Expected: all non-authenticated tests pass; live/paid provider tests remain explicitly ignored.

- [ ] **Step 2: Verify requirements manually from persisted events**

Run the fake-provider blocked integration fixture and assert its event order is `attempt.created` → `node.started` → artifacts → `flow.delivery_blocked` → `node.blocked` → `workflow.blocked`, with no Reviewer invocation.

- [ ] **Step 3: Commit any final test/doc adjustments**

If final verification required changes, stage only the exact changed test or documentation paths shown by `git status --short` and commit them with `git commit -m "test: verify managed branch blocked flow"`. If no files changed, do not create an empty commit.

- [ ] **Step 4: Request independent code review**

Review the complete range against `docs/superpowers/specs/2026-11-16-host-managed-branches-and-blocked-flow-design.md`, fix all Critical/Important findings, rerun full verification, and keep the worktree clean.
