# Selectable Team Capabilities Implementation Plan

[简体中文](../../../zh-CN/superpowers/plans/2026-10-14-selectable-team-capabilities.md) | English

**Status:** Implemented. Deterministic tests and builds pass; live Codex acceptance remains a separate manual exercise.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users select Team members and relationships, with `Coder → Reviewer` and the existing four-role Team both runnable.

**Architecture:** Validate a connected forward graph over a subset of built-in roles. Freeze only selected nodes and drive readiness, context, review gates, and terminal outcome from that graph. Update the Team editor to choose members before edges.

**Tech Stack:** Rust, rusqlite, Tauri, React, TypeScript, Vitest.

---

### Task 1: Graph model and editor

**Files:** `crates/archmage-core/src/team.rs`, `crates/archmage-core/tests/teams.rs`, `apps/archmage-desktop/src/teamGraph.ts`, `apps/archmage-desktop/src/teamGraph.test.ts`, `apps/archmage-desktop/src/TeamManagement.tsx`, `apps/archmage-desktop/src/teams.css`.

- [x] Write failing Rust tests for `Coder → Reviewer` valid, singleton/disconnected/backward invalid, and a team CRUD round trip; run `cargo test -p archmage-core --test teams` to see expected failures.
- [x] Validate subset, forward edges, single entry and reachability. Preserve builtin four-role graph.
- [x] Write failing Vitest tests for adding/removing a member and toggling only edges whose endpoints are selected; run `npm test --prefix apps/archmage-desktop`.
- [x] Add member selection and dynamic relationship controls; create starts empty and save shows validation errors. Run frontend tests/build.

### Task 2: Dynamic Run and scheduling

**Files:** `crates/archmage-core/src/runtime/mod.rs`, `crates/archmage-core/src/runtime/scheduler.rs`, `crates/archmage-core/src/runtime/dispatch.rs`, `crates/archmage-core/tests/runtime.rs`.

- [x] Write a failing Run definition test asserting a selected `Coder → Reviewer` Team creates exactly two frozen nodes and ignores unused Agent bindings.
- [x] Create only selected nodes and derive readiness/completion from frozen graph. Preserve legacy Team-less Runs and serial main execution.
- [x] Verify original four-node scheduler tests and new subset tests.

### Task 3: Optional input contracts and feedback

**Files:** `crates/archmage-core/src/context.rs`, `crates/archmage-core/src/runtime/dispatch.rs`, `crates/archmage-core/src/contract.rs`, `crates/archmage-core/src/store/consultations.rs`, `crates/archmage-core/capabilities/v1/{coder,tester,reviewer}.md`, `crates/archmage-core/tests/context.rs`, `crates/archmage-core/tests/contracts.rs`, `crates/archmage-core/tests/runtime.rs`.

- [x] Test Coder context without Designer, Reviewer context/approval without Tester, and final-review feedback reopening only selected downstream nodes.
- [x] Include only available upstream handoffs, with goal/repo fallback; preserve strict JSON output schemas and do not invent test acceptance.
- [x] Generalize final-review reopen transaction to selected-node suffix and bound revision limit; rerun focused tests.

### Task 4: Docs and verification

**Files:** `README.md`, `docs/guide.md`, `docs/runbooks/desktop-s0.md`.

- [x] Explain choose-members-then-relations flow and the two-member minimum.
- [x] Run `cargo fmt --all --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm test --prefix apps/archmage-desktop`, `npm run build --prefix apps/archmage-desktop`, `cargo check -p archmage-desktop`, and `git diff --check`.
- [x] Inspect the final diff for historical Run compatibility and verify no extra team was created automatically.
