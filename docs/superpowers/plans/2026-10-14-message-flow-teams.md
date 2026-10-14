# Message-Flow Teams Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users draw directed delivery routes, including feedback loops, and run real Codex Agents until a configured Capability emits a terminal signal.

**Architecture:** Version Team graphs in their existing JSON column so old Teams/Runs keep consultation semantics. New flow Teams freeze entry/terminal choices. A durable `flow_deliveries` ledger queues one execution per incoming message; the Agent returns its normal role output plus a separate validated routing envelope. A new flow dispatcher runs one delivery at a time and records routing atomically before consuming the next.

**Tech Stack:** Rust, rusqlite, Tauri, React, TypeScript, Vitest.

**Progress (2026-10-14):** Versioned graphs, durable delivery ledger, strict routing contract, real Codex flow dispatcher, canvas editor, Run delivery display, and guides are implemented. Deterministic feedback-cycle tests and a real two-step Coder → Reviewer → complete acceptance passed. A live feedback-cycle acceptance and interactive desktop pointer/keyboard QA remain separate follow-up checks; the real acceptance does not prove those interactions.

---

### Task 1: Versioned Team graph

**Files:** `crates/archmage-core/src/team.rs`, `crates/archmage-core/tests/teams.rs`, `apps/archmage-desktop/src/model.ts`, `apps/archmage-desktop/src/teamGraph.ts`, `apps/archmage-desktop/src/teamGraph.test.ts`.

- [ ] Add a failing Rust test that deserializes an old Team graph as legacy and accepts a flow graph with `entry: "coder"`, `terminal: "reviewer"`, and both `coder → reviewer` and `reviewer → coder`. Run `cargo test -p archmage-core --test teams`; expect the flow case to fail.
- [ ] Add `mode` (default legacy), `entry`, and `terminal` to `TeamGraph`. Validate legacy graphs by the existing forward/single-entry rule. Validate flow graphs by selected endpoints, no self/duplicate edge, reachability from entry, and path from every node to terminal. Keep Team storage API unchanged.
- [ ] Add a failing Vitest test for reciprocal edges and a deleted member clearing entry/terminal. Update graph helpers and TS types; run `npm test --prefix apps/archmage-desktop` until green.
- [ ] Run Rust Team tests and commit this independently runnable schema change.

### Task 2: Durable delivery ledger

**Files:** `crates/archmage-core/migrations/0014_flow_deliveries.sql`, `crates/archmage-core/src/store/mod.rs`, `crates/archmage-core/src/store/flow.rs`, `crates/archmage-core/tests/flow.rs`, `crates/archmage-core/tests/persistence.rs`.

- [ ] Write a failing store test: enqueue an entry delivery, claim it once, commit an attempt/routing decision that queues two target deliveries, then verify their distinct IDs and stable order. Include a repeated delivery to the same Capability.
- [ ] Add migration 14 and `Store::enqueue_flow_entry`, `claim_flow_delivery`, `complete_flow_delivery`, `flow_deliveries`, and `skip_flow_pending`. Keep source attempt, target, payload artifacts, state, sequence, and routing decision durable. All completion writes and fan-out inserts happen in one SQLite transaction with owner-generation fencing.
- [ ] Add tests for invalid/stale claims, cancelled runs, terminal skip, and upgrading a schema-13 database. Run `cargo test -p archmage-core --test flow --test persistence` and commit.

### Task 3: Routing contract and real Codex output

**Files:** `crates/archmage-core/src/contract.rs`, `crates/archmage-core/src/runtime/flow.rs`, `crates/archmage-core/src/runtime/dispatch.rs`, `crates/archmage-core/src/context.rs`, `crates/archmage-core/tests/contracts.rs`, `crates/archmage-core/tests/context.rs`.

- [ ] Write failing contract tests for `continue(next=["reviewer"])`, authorized `complete`, nonadjacent targets, duplicate/empty next, and Reviewer status disagreement.
- [ ] Define a flow envelope carrying `deliverable` (validated by the selected built-in role schema) and `routing` (`continue`/`complete`). Generate a strict JSON schema for the real Codex invocation. Validate the routing against frozen outgoing edges and terminal key; never infer routes from prose.
- [ ] Write a failing context test for a Coder delivery from Reviewer: goal, incoming feedback artifact, previous Coder artifact, and repository snapshot are rendered without a Designer handoff. Implement a flow-specific context builder while leaving legacy context hashing unchanged.
- [ ] Run focused contract/context tests and commit.

### Task 4: Flow scheduler and lifecycle

**Files:** `crates/archmage-core/src/runtime/mod.rs`, `crates/archmage-core/src/runtime/flow.rs`, `crates/archmage-core/src/runtime/dispatch.rs`, `crates/archmage-core/src/store/flow.rs`, `crates/archmage-core/tests/runtime.rs`, `crates/archmage-core/tests/flow.rs`.

- [ ] Write a failing deterministic test for Coder → Reviewer → Coder → Reviewer completion. Test one execution per delivery, no implicit join, terminal skip, and step limit 24.
- [ ] Branch `run_to_terminal` on frozen Team mode. Flow mode seeds one entry delivery, repeatedly claims one delivery, invokes the selected real Codex binding, validates and commits output/routing, and stops on authorized terminal signal. Legacy mode keeps its current scheduler/review gates.
- [ ] Expose delivery history in `RunSnapshot`; preserve cancellation, interruption, and owner fencing. Run focused runtime tests, then `cargo test --workspace`; commit.

### Task 5: Canvas editing and Run display

**Files:** `apps/archmage-desktop/src/TeamManagement.tsx`, `apps/archmage-desktop/src/teamGraph.ts`, `apps/archmage-desktop/src/teamGraph.test.ts`, `apps/archmage-desktop/src/teams.css`, `apps/archmage-desktop/src/model.ts`, Run workbench components.

- [ ] Add failing Vitest tests for beginning a connection from an output handle, cancelling on empty canvas/Escape, reciprocal edge geometry, selecting/deleting a line, and keyboard source→target creation.
- [ ] New Team creation starts in flow mode. Replace relationship checkboxes with pointer/keyboard handles and edge selection. Add entry/terminal selectors and clear validation. Mark old Teams legacy, offer explicit copy-to-flow without mutating/saving automatically.
- [ ] Show delivery and routing history in the workbench. Run Vitest/build and commit.

### Task 6: Documentation and acceptance

**Files:** `README.md`, `docs/guide.md`, `docs/runbooks/desktop-s0.md`, `docs/README.md`.

- [ ] Document how a user creates a flow Team, draws Coder ↔ Reviewer, selects entry/terminal, chooses routes, and interprets step-limit/terminal events. Explicitly distinguish legacy Teams and new flow Teams.
- [ ] Run `cargo fmt --all --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm test --prefix apps/archmage-desktop`, `npm run build --prefix apps/archmage-desktop`, `cargo check -p archmage-desktop`, and `git diff --check`. Record live Codex acceptance separately, without claiming it from deterministic tests.
- [ ] Inspect the final diff for old Team/Run compatibility and confirm no user Team was automatically created. Commit docs and final fixes.
