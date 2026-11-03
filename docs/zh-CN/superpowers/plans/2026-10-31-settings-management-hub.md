# Settings Management Hub Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move Agent, Capability, and Team management into a full-screen Settings hub while leaving only Run navigation in the main sidebar.

**Architecture:** Keep the existing `AppView` values and dirty-exit guards, but treat every non-Run view as part of the Settings space. A focused `SettingsShell` owns the Settings header and internal category navigation; existing management components remain unchanged and are rendered as its content.

**Tech Stack:** React 19, TypeScript, Vitest, existing CSS theme variables.

---

### Task 1: Add the Settings shell contract

**Files:**
- Create: `apps/archmage-desktop/src/SettingsShell.test.tsx`
- Create: `apps/archmage-desktop/src/SettingsShell.tsx`
- Modify: `apps/archmage-desktop/src/settings.css`

- [x] **Step 1: Write the failing shell test**

Render `SettingsShell` with `activeView="agents"` and assert that it contains “返回主界面”, all four category labels, an active navigation state, and its child content.

- [x] **Step 2: Run the focused test and verify RED**

Run: `npm test -- SettingsShell.test.tsx`

Expected: FAIL because `SettingsShell` does not exist.

- [x] **Step 3: Implement the minimal Settings shell**

Create a component with this public shape:

```ts
type SettingsShellProps = {
  activeView: Exclude<AppView, "runs">;
  onNavigate: (view: Exclude<AppView, "runs">) => void;
  onBack: () => void;
  children: ReactNode;
};
```

Render a top bar with a back button, a categorized internal navigation rail, and a flexible content area. Add responsive CSS that collapses the rail into a horizontal category bar on narrow windows.

- [x] **Step 4: Run the focused test and verify GREEN**

Run: `npm test -- SettingsShell.test.tsx`

Expected: PASS.

### Task 2: Route management pages through Settings

**Files:**
- Modify: `apps/archmage-desktop/src/sidebarNavigation.test.ts`
- Modify: `apps/archmage-desktop/src/App.tsx`

- [x] **Step 1: Write failing navigation assertions**

Update `sidebarNavigation.test.ts` to assert that `App.tsx` no longer contains the three management buttons in the main sidebar and that it uses `SettingsShell` for non-Run views.

- [x] **Step 2: Run the focused test and verify RED**

Run: `npm test -- sidebarNavigation.test.ts`

Expected: FAIL while the old main-sidebar buttons remain.

- [x] **Step 3: Implement Settings-space routing**

In `App.tsx`:

- Render the main sidebar only when `view === "runs"`.
- Wrap `settings`, `agents`, `capabilities`, and `teams` content with `SettingsShell`.
- Route the shell back button through `navigate("runs", null)` so dirty guards remain effective.
- Route internal category changes through `navigate(nextView)`.
- Replace direct `setView("capabilities")` and `setView("teams")` management links with `navigate(...)`.
- Keep the close-window dirty refs and existing confirmation messages unchanged.

Keep the existing Run workspace layout unchanged; the standalone `SettingsShell` supplies its own full-width layout.

- [x] **Step 4: Run navigation and dirty-guard tests**

Run: `npm test -- sidebarNavigation.test.ts model.test.ts teamGraph.test.ts`

Expected: PASS.

### Task 3: Regression verification

**Files:**
- Modify only if verification exposes a defect in files already listed above.

- [x] **Step 1: Run the desktop test suite**

Run: `npm test`

Expected: all tests PASS.

- [x] **Step 2: Build the desktop frontend**

Run: `npm run build`

Expected: TypeScript and Vite build complete successfully.

- [x] **Step 3: Review the final diff**

Run: `git diff --check && git status --short`

Expected: no whitespace errors; only the planned files plus the user's pre-existing untracked files are present.

- [x] **Step 4: Commit the implementation**

Commit message: `feat: move management pages into settings`
