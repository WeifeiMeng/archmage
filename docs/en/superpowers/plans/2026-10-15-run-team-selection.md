[简体中文](../../../zh-CN/superpowers/plans/2026-10-15-run-team-selection.md) | English

# Run Team Selection Implementation Plan


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Team selection and composition the focal point of New Run, with only relevant launch problems displayed.

**Architecture:** Derive a read-only Team summary and selected-Capability binding issues from existing client models. Keep the existing Run request and backend preflight unchanged. Replace three persistent status cards with a compact Team preview and conditional diagnostics.

**Tech Stack:** React, TypeScript, Vitest, CSS.

---

### Task 1: Team-specific form state

**Files:** `apps/archmage-desktop/src/model.ts`, `apps/archmage-desktop/src/model.test.ts`

- [x] Add a test where only Coder and Reviewer are selected and Designer/Tester are unbound; expect no binding issues. Add a test for an unusable selected binding.
- [x] Run `npm test --prefix apps/archmage-desktop -- src/model.test.ts` and confirm the new test fails.
- [x] Extend `runBindingIssues(snapshot, capabilities)` so omitted capabilities retain the current all-role behavior and supplied capabilities limit validation to the selected Team.
- [x] Add a test for a flow Team summary containing members, edges, entry, and terminal; implement a typed helper that derives it directly from `Team.graph`.
- [x] Run the focused tests again and confirm they pass.

### Task 2: New Run form

**Files:** `apps/archmage-desktop/src/App.tsx`, `apps/archmage-desktop/src/RunTeamPreview.tsx`, `apps/archmage-desktop/src/RunTeamPreview.test.tsx`, `apps/archmage-desktop/src/runTeam.css`, `apps/archmage-desktop/src/capabilities.css`, `apps/archmage-desktop/src/agents.css`

- [x] Resolve the selected Team only after a successful fresh Team-list request, and use only its nodes for launch gating.
- [x] Render one compact Team preview with member names, flow edges, and flow entry/terminal labels, or a legacy consultation label.
- [x] Remove the persistent Tester, binding-success, and Runtime-ready cards. Show no-Team, Team-load, binding, and Runtime errors only when actionable. Preserve the disabled Start button while required inputs or readiness are missing.
- [x] Update the heading and goal placeholder to refer to the selected Team rather than four fixed roles. Keep labels, readable contrast, and keyboard focus styles.

### Task 3: Verification

**Files:** `docs/superpowers/specs/2026-10-15-run-team-selection-design.md`, `docs/superpowers/plans/2026-10-15-run-team-selection.md`

- [x] Run `npm test --prefix apps/archmage-desktop`, `npm run build --prefix apps/archmage-desktop`, and `git diff --check`.
- [x] Review the final diff and commit only this scoped UI and documentation work on `main`.
