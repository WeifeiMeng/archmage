[简体中文](../../../zh-CN/superpowers/plans/2026-10-29-team-workspace-vertical-layout.md) | English

# Team Workspace Vertical Layout Implementation Plan


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stack Team settings above the message-flow canvas so both panels use the full Team Management content width.

**Architecture:** Keep the existing React component order and behavior unchanged. Convert only the `.team-workspace` CSS grid from a two-column desktop layout to one column, and extend the existing CSS regression test to lock that layout in place.

**Tech Stack:** React 19, CSS Grid, TypeScript, Vitest, Vite

---

### Task 1: Stack the Team workspace panels

**Files:**
- Modify: `apps/archmage-desktop/src/teamsStyle.test.ts`
- Modify: `apps/archmage-desktop/src/teams.css:7`

- [x] **Step 1: Write the failing layout regression test**

Add this case to `apps/archmage-desktop/src/teamsStyle.test.ts`:

```ts
it("stacks Team settings above the message-flow canvas", () => {
  expect(teamsCss).toMatch(/\.team-workspace\s*\{[^}]*grid-template-columns:\s*minmax\(0, 1fr\);/s);
  expect(teamsCss).not.toMatch(/\.team-workspace\s*\{[^}]*grid-template-columns:\s*minmax\(350px,/s);
});
```

- [x] **Step 2: Run the focused test and verify it fails**

Run:

```bash
cd apps/archmage-desktop && npm test -- teamsStyle.test.ts
```

Expected: the new case fails because `.team-workspace` still declares two desktop columns.

- [x] **Step 3: Implement the single-column workspace**

Replace the `.team-workspace` rule in `apps/archmage-desktop/src/teams.css` with:

```css
.team-workspace { display: grid; grid-template-columns: minmax(0, 1fr); gap: 14px; margin-top: 14px; align-items: start; }
```

Do not change `TeamManagement.tsx`, canvas behavior, or Run detail styles.

- [x] **Step 4: Run focused and complete verification**

Run:

```bash
cd apps/archmage-desktop
npm test -- teamsStyle.test.ts
npm test
npm run build
git diff --check
```

Expected: the focused test passes, all frontend tests pass, the production build succeeds, and `git diff --check` reports no errors.

- [x] **Step 5: Commit the implementation**

```bash
git add apps/archmage-desktop/src/teams.css apps/archmage-desktop/src/teamsStyle.test.ts docs/superpowers/plans/2026-10-29-team-workspace-vertical-layout.md
git commit -m "Stack Team workspace panels"
```
