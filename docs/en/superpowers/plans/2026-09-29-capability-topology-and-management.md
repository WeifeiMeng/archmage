# Capability Topology and Management Implementation Plan

[简体中文](../../../zh-CN/superpowers/plans/2026-09-29-capability-topology-and-management.md) | English

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show the four built-in Capability roles as a truthful fixed topology and let users bind each role to an Agent without changing existing Runs. Follow-up scope decision: prompt and metadata editing are deferred.

**Architecture:** Core owns built-in topology and capability configuration. SQLite persists configuration, Runtime freezes it into each Run, and Tauri exposes typed commands. React draws the fixed graph and edits one selected capability at a time.

**Tech Stack:** Rust, rusqlite, Tauri 2, React, TypeScript, CSS, Vitest.

---

### Task 1: Persist Capability configuration

**Files:** `crates/archmage-core/migrations/0009_capabilities.sql`, `crates/archmage-core/src/store/mod.rs`, `crates/archmage-core/src/capability.rs`, `crates/archmage-core/tests/agents.rs`.

- [ ] Add a failing Store test: opening a schema-8 database upgrades to 9, preserves `capability_bindings`, and yields four default configurations; invalid key and stale revision saves fail.
- [ ] Run `cargo test -p archmage-core capability_config -- --nocapture` and confirm failure before implementation.
- [ ] Add the version-9 migration and Store read/save methods. Save metadata and binding in one immediate transaction, compare the supplied revision, validate key and field limits (80/500/8000), and bump revision exactly once.
- [ ] Expose immutable key/default metadata and topology edges in `capability.rs`. Execution edges are `designer→coder→tester→reviewer`; additional context edges are `designer→tester`, `designer→reviewer`, `coder→reviewer`.
- [ ] Run the focused test, then commit this unit.

### Task 2: Freeze configuration and render prompts

**Files:** `crates/archmage-core/src/runtime/mod.rs`, `crates/archmage-core/src/runtime/dispatch.rs`, `crates/archmage-core/src/context.rs`, `crates/archmage-core/tests/runtime.rs`.

- [ ] Add failing tests: a Run freezes all four configuration records; editing one later leaves the older Run unchanged; an old definition without configuration deserializes; the rendered context contains the frozen supplemental instructions.
- [ ] Run `cargo test -p archmage-core capability_config -- --nocapture` and confirm the new tests fail.
- [ ] Extend the frozen Run definition with a per-capability configuration map using serde defaults for legacy Runs. Resolve it at creation, return it in `RunSnapshot`, and pass frozen instructions to `resolve_context` as a separate manifest field. Preserve the fixed base prompt and schema.
- [ ] Run focused tests and commit this unit.

### Task 3: Tauri/API boundary

**Files:** `apps/archmage-desktop/src-tauri/src/lib.rs`, `apps/archmage-desktop/src/api.ts`, `apps/archmage-desktop/src/model.ts`.

- [ ] Define typed `CapabilityConfiguration`, `CapabilityManagementSnapshot`, and save input in Rust/TypeScript with matching camelCase fields.
- [ ] Add `get_capability_management` and `save_capability_configuration` commands calling the Core service. The latter returns the refreshed snapshot, including revision and available Agent instances.
- [ ] Keep the old binding command only for backward compatibility while moving UI routing controls to the new page.
- [ ] Run `cargo check -p archmage-desktop` and `npm run build` from `apps/archmage-desktop`; commit this unit.

### Task 4: Fixed graph and editor

**Files:** `apps/archmage-desktop/src/CapabilityManagement.tsx`, `apps/archmage-desktop/src/capabilities.css`, `apps/archmage-desktop/src/AgentManagement.tsx`, `apps/archmage-desktop/src/App.tsx`, `apps/archmage-desktop/src/model.test.ts`.

- [ ] Add a failing frontend test for topology metadata: four stable nodes, three execution edges, three additional context edges, and no edge-edit action.
- [ ] Run `npm test` from `apps/archmage-desktop` and confirm the new test fails.
- [ ] Add a `Capabilities` sidebar destination. Draw a fixed left-to-right SVG/HTML topology with solid execution arrows and dashed extra handoffs, a legend, accessible node buttons, and horizontal overflow at narrow widths.
- [ ] Show read-only role metadata and a single editable Agent binding. Save only on explicit action; preserve the selection on failure and show revision conflicts. Show real connection status only from Runtime.
- [ ] Remove the binding table from `AgentManagement` and link to `Capabilities`. Link Run binding warnings to the new page.
- [ ] Run frontend tests/build and commit this unit.

### Task 5: Integrated verification and docs

**Files:** `README.md`, `docs/guide.md`, the files changed above.

- [ ] Document where Capability settings live, the meaning of solid versus dashed arrows, future-Run-only behavior, and that Team canvas editing is not yet supported.
- [ ] Run `cargo test --workspace`, `npm test`, and `npm run build` in the desktop app; check `git diff --check`.
- [ ] Inspect the final diff against the approved spec and report any remaining gaps. Commit docs and final fixes without merging or pushing unless requested.
