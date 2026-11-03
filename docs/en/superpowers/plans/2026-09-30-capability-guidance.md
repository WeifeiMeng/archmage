# Capability Guidance Implementation Plan

[简体中文](../../../zh-CN/superpowers/plans/2026-09-30-capability-guidance.md) | English

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make each Capability's role supplement, Skills, and Tools editable and pass the frozen guidance to its real Agent.

**Architecture:** Add two JSON list columns to the existing revision-guarded Capability row. Extend the frozen configuration and both prompt builders; keep Codex permissions unchanged. Expand the existing list/detail editor rather than adding a topology or a separate catalog.

**Tech Stack:** Rust, rusqlite, serde, Tauri, React, TypeScript, Vitest.

---

### Task 1: Persist validated guidance

**Files:** `crates/archmage-core/src/capability.rs`, `crates/archmage-core/src/store/capabilities.rs`, `crates/archmage-core/src/store/mod.rs`, `crates/archmage-core/migrations/0013_capability_guidance.sql`, `crates/archmage-core/tests/agents.rs`.

- [x] Add a failing test that saves Designer with one Skill (`brainstorming`) and one Tool (`web_search`), reloads both, and rejects a duplicate name and stale revision. Run `cargo test -p archmage-core --test agents capability_config -- --nocapture` and confirm the expected failure.
- [x] Add `CapabilityGuidance { name: String, purpose: String }` and `skills`/`tools` arrays (serde defaults) to configuration and input. Validate nonblank names and purposes, maximum 16 entries per list, name length 80, purpose length 500, and case-insensitive unique names within each list.
- [x] Migrate with `ALTER TABLE capability_configurations ADD COLUMN skills_json TEXT NOT NULL DEFAULT '[]'` and the equivalent `tools_json`; register version 13 in `Store::open`. Decode/encode the JSON arrays inside the existing guarded save transaction.
- [x] Re-run the focused test and check it passes. Update existing input literals in tests with empty arrays and migration version assertions to 13.

### Task 2: Freeze and deliver guidance

**Files:** `crates/archmage-core/src/context.rs`, `crates/archmage-core/src/runtime/dispatch.rs`, `crates/archmage-core/src/runtime/consultation.rs`, `crates/archmage-core/tests/context.rs`, `crates/archmage-core/tests/consultation_prompt.rs`, `crates/archmage-core/tests/runtime.rs`.

- [x] Add failing prompt tests: primary context includes Skill/Tool names, purposes, and advisory semantics; consultation includes the target Capability's same frozen guidance. Add a Run snapshot test that changing a Capability after Run creation does not change that Run's arrays.
- [x] Pass configuration into primary context assembly and consultation rendering. Keep built-in persona and output schema intact. Include the guidance as structured fields, with an explicit advisory/non-enforcement note.
- [x] Re-run focused context, consultation, and runtime tests; resolve only new regressions.

### Task 3: Edit guidance in Capability management

**Files:** `apps/archmage-desktop/src/model.ts`, `apps/archmage-desktop/src/model.test.ts`, `apps/archmage-desktop/src/CapabilityManagement.tsx`, `apps/archmage-desktop/src/capabilities.css`, `docs/guide.md`.

- [x] Add failing Vitest coverage for serializing an edited draft and for dirty detection when role text, Skill list, Tool list, or binding changes. Run `npm test --prefix apps/archmage-desktop` and confirm the expected failure.
- [x] Replace the binding-only draft helper with a full-configuration draft. Add editable role textarea and name/purpose rows for Skills and Tools. Save all fields through the existing Tauri command and display validation/conflict errors. Show counts in the list and the advisory disclaimer in the detail view.
- [x] Document the workflow and the advisory-only semantics in `docs/guide.md`; run frontend tests and build.

### Task 4: Final verification

- [x] Run `cargo fmt --check`, `cargo test --workspace`, `npm test --prefix apps/archmage-desktop`, `npm run build --prefix apps/archmage-desktop`, and `git diff --check` from repository root.
- [x] Check the diff against the design and report any unverified live-Codex behavior distinctly from deterministic tests.
