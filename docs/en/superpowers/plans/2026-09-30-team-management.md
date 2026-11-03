[简体中文](../../../zh-CN/superpowers/plans/2026-09-30-team-management.md) | English

# Review-Gated Team Management Implementation Plan


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a Run select a saved Team whose fixed four-Capability graph causes real, concurrent downstream consultations and bounded feedback revisions.

**Architecture:** SQLite and `archmage-core` own Team definitions and review records. Each Run freezes a validated Team and Capability/Agent settings. The Runtime preserves the required four-role execution spine, fans out read-only consultations on saved outgoing edges, and reruns rejected producers with persisted suggestions. Tauri exposes typed Team and Run data; React owns only editing and visualization.

**Tech Stack:** Rust, rusqlite, Tokio, real Codex CLI adapter, Tauri 2, React, TypeScript, Vitest.

---

### Task 1: Team model and graph validation

**Files:** Create `crates/archmage-core/src/team.rs`; modify `crates/archmage-core/src/lib.rs`; test `crates/archmage-core/tests/teams.rs`.

- [ ] Write tests that accept the required spine and any subset of `(designer,tester)`, `(designer,reviewer)`, `(coder,reviewer)`, but reject a missing required edge, duplicate/reverse edge, nonbuilt-in key, duplicate node, or position outside bounded canvas coordinates.
- [ ] Run `cargo test -p archmage-core --test teams graph_ -- --nocapture`; confirm tests fail because `TeamGraph` is absent.
- [ ] Define `TeamGraph { nodes: Vec<TeamNode>, edges: Vec<TeamEdge> }`, `TeamNode { capability: String, x: i32, y: i32 }`, `TeamEdge { from: String, to: String }`, and `validate()` with explicit required and allowed edge sets. Define a default six-edge graph.
- [ ] Run the focused tests and commit `feat: validate first-release team graphs`.

### Task 2: Persist Team definitions and expose management

**Files:** Create `crates/archmage-core/migrations/0010_teams.sql`, `crates/archmage-core/src/store/teams.rs`; modify `crates/archmage-core/src/store/mod.rs`, `crates/archmage-core/src/team.rs`, `apps/archmage-desktop/src-tauri/src/lib.rs`, `apps/archmage-desktop/src/model.ts`, `apps/archmage-desktop/src/api.ts`; test `crates/archmage-core/tests/teams.rs`.

- [ ] Add a migration test upgrading a schema-9 database and preserving bindings/Run rows; assert one idempotently seeded default Team.
- [ ] Add service tests: create a named Team, update name/graph with matching revision, reject blank/duplicate name and stale revision, and leave the prior row unchanged on validation failure. Run tests red.
- [ ] Persist `teams(id,name,revision,graph_json,created_at,updated_at)` transactionally. Use the existing Store migration path and `TeamService` wrapper. No delete command.
- [ ] Add typed `list_teams`, `create_team`, `save_team` Tauri commands and matching frontend API/model types. Run Rust tests and frontend build; commit `feat: persist and manage teams`.

### Task 3: Freeze selected Team into Run

**Files:** Modify `crates/archmage-core/src/runtime/mod.rs`, `apps/archmage-desktop/src/model.ts`, `apps/archmage-desktop/src/api.ts`, `apps/archmage-desktop/src/App.tsx`; test `crates/archmage-core/tests/runtime.rs`, `apps/archmage-desktop/src/model.test.ts`.

- [ ] Test `RunRequest { team_id: Some(id), ... }` freezes the Team ID/name/revision/graph; later Team edits do not change that Run. Test an omitted Team ID selects the default for CLI compatibility and a legacy definition without a Team still snapshots.
- [ ] Run focused Rust tests red, then add `team_id: Option<String>` with a serde default and `FrozenTeam` in the definition/snapshot. Resolve and validate the Team before creating a Run or provisioning worktree.
- [ ] Add a frontend serialization test expecting `teamId`, then update the request helper. Run tests/build; commit `feat: freeze selected team into runs`.

### Task 4: Persist consultation decisions and revision transitions

**Files:** Create `crates/archmage-core/migrations/0011_consultations.sql`, `crates/archmage-core/src/store/consultations.rs`; modify `crates/archmage-core/src/store/transitions.rs`, `crates/archmage-core/src/store/mod.rs`; test `crates/archmage-core/tests/consultations.rs`.

- [ ] Test review records keyed to the source Attempt and edge; approval/rejection events include reviewer and round; duplicate/stale approvals cannot satisfy a new producer Attempt.
- [ ] Test atomic reopen of a succeeded upstream node for another Attempt and bounded feedback storage; reject reopen if a downstream main node has already started. Run tests red.
- [ ] Add strict storage transitions with revision/owner-generation checks and separate review records, suggestions JSON, thread/result artifact identity, and status. Do not overwrite old Attempts/artifacts. Run tests; commit `feat: persist review consultations and revisions`.

### Task 5: Real Codex consultations and gated scheduler

**Files:** Create `crates/archmage-core/capabilities/v1/consultation.schema.json`, `crates/archmage-core/src/runtime/consultation.rs`; modify `crates/archmage-core/src/runtime/dispatch.rs`, `crates/archmage-core/src/runtime/scheduler.rs`, `crates/archmage-core/src/runtime/mod.rs`, `crates/archmage-core/src/contract.rs`, `crates/archmage-core/src/context.rs`; test `crates/archmage-core/tests/runtime.rs` and new focused scheduler tests.

- [ ] Test a producer with three outgoing edges requires three approvals, that mixed decisions aggregate suggestions and rerun only that producer, that the fourth rejected output fails, and that old approvals cannot unblock a new Attempt. Include final Reviewer `changes_requested` rerouting through Coder and Tester. Run red.
- [ ] Invoke the real Codex adapter with a read-only sandbox and consultation schema. Build the prompt from the target Capability persona, frozen binding, goal, current producer artifact, and source-attempt ID. Validate nonblank suggestions for rejection; persist exact artifact and event.
- [ ] Run outgoing consultations concurrently with separate Store handles; join all results before scheduling a downstream main node. Feed persisted feedback into the producer's next context and make the scheduler read the frozen Team graph. Keep main worktree-writing executions serialized. Run focused tests; commit `feat: gate team runs on real consultations`.

### Task 6: Desktop Team canvas and separated Capability page

**Files:** Create `apps/archmage-desktop/src/TeamManagement.tsx`, `apps/archmage-desktop/src/teams.css`; modify `apps/archmage-desktop/src/CapabilityManagement.tsx`, `apps/archmage-desktop/src/capabilities.css`, `apps/archmage-desktop/src/App.tsx`, `apps/archmage-desktop/src/model.ts`, `apps/archmage-desktop/src/model.test.ts`.

- [ ] Test the three mandatory edges remain selected/locked, optional edges toggle without making an invalid graph, and navigation warns about unsaved Team edits. Run frontend tests red.
- [ ] Replace Capability topology with a four-item list/detail editor; keep Agent binding and read-only persona summary.
- [ ] Add Team selection/create, draggable four-node canvas, keyboard-operable optional-edge list, visible required-edge lock, Save/dirty/conflict feedback, and genuine Tauri-only state. Add Team selection to New Run and frozen Team/review event display to Run detail.
- [ ] Run `npm test` and `npm run build`; commit `feat: add team canvas and review timeline`.

### Task 7: Documentation and verification

**Files:** Modify `README.md`, `docs/guide.md`, `docs/runbooks/desktop-s0.md`; review all changed files.

- [ ] Document Agent vs Capability vs Team ownership, graph constraints, Team selection, consultation/feedback rounds, and the future skill-scope boundary.
- [ ] Run `cargo test --workspace --quiet`, `npm test`, `npm run build`, and `git diff --check`. Separately report that authenticated Codex live acceptance was or was not run; never equate deterministic adapter tests with a real account probe.
- [ ] Inspect the final diff against `docs/superpowers/specs/2026-09-30-team-management-design.md`, fix any gap, and commit final docs/fixes. Do not merge or push without user request.
