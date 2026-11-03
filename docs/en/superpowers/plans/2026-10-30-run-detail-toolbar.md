[简体中文](../../../zh-CN/superpowers/plans/2026-10-30-run-detail-toolbar.md) | English

# Run Detail Toolbar Cleanup Implementation Plan


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Run detail header actions with the approved grouped toolbar and remove the redundant frozen-team-topology panel.

**Architecture:** Extract the three header controls into a focused presentational React component with its own CSS and rendering tests. Keep all existing callbacks and Run state decisions in `App.tsx`; the page only delegates display to the new component and stops rendering the duplicate topology panel.

**Tech Stack:** React 19, TypeScript, Lucide React, CSS, Vitest, React DOM server rendering.

---

### Task 1: Add the grouped Run header toolbar

**Files:**
- Create: `apps/archmage-desktop/src/RunHeaderActions.tsx`
- Create: `apps/archmage-desktop/src/RunHeaderActions.css`
- Create: `apps/archmage-desktop/src/RunHeaderActions.test.tsx`
- Modify: `apps/archmage-desktop/src/App.tsx:1-45,334-342`

- [ ] **Step 1: Write the failing component tests**

Create `RunHeaderActions.test.tsx` with static-render tests that verify the grouped toolbar label, all three active-Run actions, disabled workspace behavior, and omission of the cancel action for an inactive Run:

```tsx
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { RunHeaderActions } from "./RunHeaderActions";

const callbacks = { onRefresh: vi.fn(), onReveal: vi.fn(), onCancel: vi.fn() };

describe("Run header actions", () => {
  it("groups the active Run actions and disables an unavailable workspace", () => {
    const html = renderToStaticMarkup(
      <RunHeaderActions {...callbacks} workspaceAvailable={false} cancellable />,
    );
    expect(html).toContain('aria-label="Run 操作"');
    expect(html).toContain("刷新");
    expect(html).toContain("工作区");
    expect(html).toContain("取消");
    expect(html).toContain("disabled");
  });

  it("omits cancel after the Run becomes inactive", () => {
    const html = renderToStaticMarkup(
      <RunHeaderActions {...callbacks} workspaceAvailable cancellable={false} />,
    );
    expect(html).not.toContain("取消");
  });
});
```

- [ ] **Step 2: Run the test and verify RED**

Run: `cd apps/archmage-desktop && npm test -- RunHeaderActions.test.tsx`

Expected: FAIL because `./RunHeaderActions` does not exist.

- [ ] **Step 3: Implement the toolbar component**

Create `RunHeaderActions.tsx`:

```tsx
import { FolderOpen, RefreshCw, Square } from "lucide-react";
import "./RunHeaderActions.css";

export function RunHeaderActions({ workspaceAvailable, cancellable, onRefresh, onReveal, onCancel }: {
  workspaceAvailable: boolean;
  cancellable: boolean;
  onRefresh: () => void;
  onReveal: () => void;
  onCancel: () => void;
}) {
  return <div className="header-actions" role="group" aria-label="Run 操作">
    <button onClick={onRefresh}><RefreshCw size={16} />刷新</button>
    <button onClick={onReveal} disabled={!workspaceAvailable}><FolderOpen size={16} />工作区</button>
    {cancellable && <button className="danger" onClick={onCancel}><Square size={14} fill="currentColor" />取消</button>}
  </div>;
}
```

Create `RunHeaderActions.css` with higher-specificity Run-header selectors so it safely overrides the legacy base styles regardless of CSS import order:

```css
.run-header .header-actions {
  align-self: flex-start;
  display: inline-flex;
  gap: 0;
  margin-top: 38px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface);
  box-shadow: var(--shadow);
}
.run-header .header-actions button {
  min-width: 0;
  min-height: 38px;
  padding: 0 12px;
  border: 0;
  border-radius: 0;
  background: transparent;
  color: var(--text);
  white-space: nowrap;
  transition: background-color 160ms ease, color 160ms ease;
}
.run-header .header-actions button + button { border-left: 1px solid var(--border); }
.run-header .header-actions button:hover:not(:disabled) { background: var(--surface-2); }
.run-header .header-actions button:focus-visible { position: relative; z-index: 1; outline: 2px solid var(--accent); outline-offset: -2px; }
.run-header .header-actions button:active:not(:disabled) { background: var(--accent-soft); }
.run-header .header-actions button:disabled { cursor: not-allowed; opacity: .4; }
.run-header .header-actions .danger { background: var(--danger-bg); color: var(--danger); }
.run-header .header-actions .danger:hover:not(:disabled) { background: color-mix(in srgb, var(--danger-bg) 72%, var(--danger) 28%); }
```

Update `App.tsx` to import `RunHeaderActions`, remove toolbar-only icon imports, and replace the inline `.header-actions` JSX with:

```tsx
<RunHeaderActions
  workspaceAvailable={Boolean(snapshot.workspace)}
  cancellable={isActive(snapshot.state)}
  onRefresh={onRefresh}
  onReveal={onReveal}
  onCancel={onCancel}
/>
```

- [ ] **Step 4: Run the component tests and verify GREEN**

Run: `cd apps/archmage-desktop && npm test -- RunHeaderActions.test.tsx`

Expected: 2 tests pass.

- [ ] **Step 5: Commit the toolbar implementation**

```bash
git add apps/archmage-desktop/src/App.tsx apps/archmage-desktop/src/RunHeaderActions.tsx apps/archmage-desktop/src/RunHeaderActions.css apps/archmage-desktop/src/RunHeaderActions.test.tsx
git commit -m "style: group Run header actions"
```

### Task 2: Remove the duplicate frozen topology panel

**Files:**
- Modify: `apps/archmage-desktop/src/App.tsx:340-343`
- Create: `apps/archmage-desktop/src/RunDetailLayout.test.ts`

- [ ] **Step 1: Write the failing layout regression test**

Create `RunDetailLayout.test.ts` using the repository's existing source-layout test pattern:

```ts
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const appSource = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");

describe("Run detail layout", () => {
  it("does not repeat the frozen Team topology below capability progress", () => {
    expect(appSource).not.toContain("冻结的团队拓扑");
    expect(appSource).not.toContain("snapshot.team.graph.edges.map");
    expect(appSource).toContain("<FlowProgress snapshot={snapshot} />");
    expect(appSource).toContain("<ConsultationProgress snapshot={snapshot} />");
  });
});
```

- [ ] **Step 2: Run the test and verify RED**

Run: `cd apps/archmage-desktop && npm test -- RunDetailLayout.test.ts`

Expected: FAIL because `App.tsx` still contains the frozen topology heading and direct edge rendering.

- [ ] **Step 3: Remove only the duplicate panel**

Delete this JSX from `RunWorkbench`:

```tsx
{snapshot.team && <section className="panel team-run-panel"><div className="panel-title"><div><span>冻结的团队拓扑</span><small>{snapshot.team.name} · v{snapshot.team.revision}</small></div></div><div className="team-run-edges">{snapshot.team.graph.edges.map((edge) => <span key={`${edge.from}-${edge.to}`}>{edge.from} → {edge.to}</span>)}</div></section>}
```

Keep the following `FlowProgress` / `ConsultationProgress` branch unchanged.

- [ ] **Step 4: Run the layout test and verify GREEN**

Run: `cd apps/archmage-desktop && npm test -- RunDetailLayout.test.ts`

Expected: 1 test passes.

- [ ] **Step 5: Commit the topology cleanup**

```bash
git add apps/archmage-desktop/src/App.tsx apps/archmage-desktop/src/RunDetailLayout.test.ts
git commit -m "style: remove duplicate Run topology panel"
```

### Task 3: Verify the complete desktop change

**Files:**
- Verify only; no planned source changes.

- [ ] **Step 1: Run all desktop tests**

Run: `cd apps/archmage-desktop && npm test`

Expected: all Vitest files pass with zero failures.

- [ ] **Step 2: Build the desktop frontend**

Run: `cd apps/archmage-desktop && npm run build`

Expected: TypeScript and Vite complete with exit code 0.

- [ ] **Step 3: Inspect the live Run detail page**

Open the active Run detail page in the Tauri development app and verify:

- “刷新 / 工作区 / 取消” stay on one line in one grouped surface.
- “取消” remains visibly destructive but subordinate to the Run title.
- The frozen topology panel is absent.
- Team name/version, capability progress, progress messages, timeline, and inspector remain visible.

- [ ] **Step 4: Record final repository state**

Run: `git status --short && git log -3 --oneline`

Expected: only the user's pre-existing untracked files remain; the two implementation commits are at `HEAD`.
