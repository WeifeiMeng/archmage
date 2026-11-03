# Desktop Layout Polish Implementation Plan

[简体中文](../../../zh-CN/superpowers/plans/2026-10-28-desktop-layout-polish.md) | English

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign Team Management, New Run, Agent Management, and Capability Management around the approved command-Hero and adaptive workspace layouts while preserving all existing behavior and leaving Run Detail unchanged.

**Architecture:** Introduce one small presentational `ManagementHero` primitive for shared page identity, page-level actions, and summaries. Keep business state and API calls in their existing owners, derive compact summaries through pure exported functions, and restructure each page's markup plus scoped CSS without changing Runtime contracts.

**Tech Stack:** React 19, TypeScript, CSS, Lucide React, Vitest 5, React DOM server rendering, Vite 8.

---

## File map

- Create `apps/archmage-desktop/src/ManagementHero.tsx`: shared presentational Hero and metric markup.
- Create `apps/archmage-desktop/src/ManagementHero.test.tsx`: SSR assertions for semantic Hero content.
- Create `apps/archmage-desktop/src/managementLayout.ts`: pure display-summary helpers for environment, Agent, Capability, and Team state.
- Create `apps/archmage-desktop/src/managementLayout.test.ts`: test-first coverage for all summary helpers.
- Modify `apps/archmage-desktop/src/App.tsx`: replace only New Run markup with the approved Hero and two-panel workspace.
- Modify `apps/archmage-desktop/src/styles.css`: shared Hero primitives and scoped New Run layout; preserve Run Detail selectors.
- Modify `apps/archmage-desktop/src/AgentManagement.tsx`: command Hero, metrics, compact connection heading, and routing handoff.
- Modify `apps/archmage-desktop/src/agents.css`: approved Agent density and responsive card layout.
- Modify `apps/archmage-desktop/src/CapabilityManagement.tsx`: command Hero, derived totals, and master-detail workspace.
- Modify `apps/archmage-desktop/src/capabilities.css`: vertical Capability rail, editor grid, parallel guidance sections, and breakpoints.
- Modify `apps/archmage-desktop/src/TeamManagement.tsx`: command Hero, integrated legacy state, configuration/canvas workspace, accessible refresh action.
- Modify `apps/archmage-desktop/src/teams.css`: approved Team split workspace, capability cards, canvas panel, and responsive layout.

### Task 1: Shared command Hero and summary helpers

**Files:**
- Create: `apps/archmage-desktop/src/ManagementHero.tsx`
- Create: `apps/archmage-desktop/src/ManagementHero.test.tsx`
- Create: `apps/archmage-desktop/src/managementLayout.ts`
- Create: `apps/archmage-desktop/src/managementLayout.test.ts`
- Modify: `apps/archmage-desktop/src/styles.css`

- [ ] **Step 1: Write failing SSR tests for the shared Hero**

Create `ManagementHero.test.tsx` with a test that renders:

```tsx
renderToStaticMarkup(
  <ManagementHero
    eyebrow="TEAM RUN"
    title="把目标交给团队"
    description="定义任务并确认执行环境。"
    actions={<button>刷新</button>}
    summary={<PageMetric label="本地 Runtime" value="已就绪" tone="ready" />}
  />,
);
```

Assert the output contains `management-hero`, `management-hero-actions`, `page-metric ready`, the heading, description, action, label, and value.

- [ ] **Step 2: Run the Hero test and verify RED**

Run:

```bash
npm test -- ManagementHero.test.tsx
```

Expected: FAIL because `ManagementHero.tsx` does not exist.

- [ ] **Step 3: Implement the minimal shared Hero**

Create typed `ManagementHero` and `PageMetric` components. `ManagementHero` renders a semantic `<header className="management-hero">`, a copy block, optional actions, and optional summary. `PageMetric` renders text labels and accepts `tone?: "default" | "ready" | "warning" | "failed"`.

- [ ] **Step 4: Run the Hero test and verify GREEN**

Run `npm test -- ManagementHero.test.tsx`.

Expected: one test file passes with no warnings.

- [ ] **Step 5: Write failing pure-summary tests**

Create `managementLayout.test.ts` with focused assertions:

```ts
expect(environmentSummary("ready")).toEqual({ label: "本地 Runtime", value: "已就绪", detail: "Agent 与团队配置可用", tone: "ready" });
expect(environmentSummary("preview").value).toBe("浏览器预览");
expect(agentMetrics(snapshot)).toEqual([
  { label: "已验证", value: 1, tone: "ready" },
  { label: "未验证", value: 1, tone: "warning" },
  { label: "连接失败", value: 1, tone: "failed" },
  { label: "总实例", value: 3, tone: "default" },
]);
expect(capabilityMetrics(snapshot)).toEqual({ capabilities: 4, bound: 3, skills: 5, tools: 4 });
expect(teamSummary(teamGraph)).toEqual({ capabilities: 4, routes: 3, modeLabel: "消息流" });
```

Use minimal typed fixtures from `model.ts`.

- [ ] **Step 6: Run the summary tests and verify RED**

Run `npm test -- managementLayout.test.ts`.

Expected: FAIL because `managementLayout.ts` does not exist.

- [ ] **Step 7: Implement the summary helpers**

Create pure functions:

```ts
export type EnvironmentState = "checking" | "ready" | "failed" | "preview";
export function environmentSummary(state: EnvironmentState): PageStatus;
export function agentMetrics(snapshot: AgentManagementSnapshot): PageMetricData[];
export function capabilityMetrics(snapshot: CapabilityManagementSnapshot): CapabilityMetricData;
export function teamSummary(graph: TeamGraph): TeamMetricData;
```

Count only supported Codex instances for Agent totals, valid enabled Codex bindings for Capability bound totals, and current nodes/edges for Team totals.

- [ ] **Step 8: Run shared tests and full frontend tests**

Run:

```bash
npm test -- ManagementHero.test.tsx managementLayout.test.ts
npm test
```

Expected: shared tests pass and the baseline remains 17 existing tests plus the new tests.

- [ ] **Step 9: Add shared Hero CSS**

Add scoped rules for `.management-hero`, `.management-hero-copy`, `.management-hero-actions`, `.management-hero-summary`, `.page-metric`, and tone classes. Use the approved green-tinted gradient, compact title, 14–16px radius, and a `max-width: 780px` wrap breakpoint. Do not alter `.run-header` or other Run Detail selectors.

- [ ] **Step 10: Commit the shared foundation**

```bash
git add apps/archmage-desktop/src/ManagementHero.tsx apps/archmage-desktop/src/ManagementHero.test.tsx apps/archmage-desktop/src/managementLayout.ts apps/archmage-desktop/src/managementLayout.test.ts apps/archmage-desktop/src/styles.css
git commit -m "Add shared management page layout primitives"
```

### Task 2: Redesign New Run without touching Run Detail

**Files:**
- Modify: `apps/archmage-desktop/src/App.tsx`
- Modify: `apps/archmage-desktop/src/styles.css`
- Test: `apps/archmage-desktop/src/managementLayout.test.ts`

- [ ] **Step 1: Extend the environment-summary test with the unimplemented checking and failed states**

Task 1 implements the approved `ready` and `preview` cases plus a neutral fallback. Add exact assertions for `checking` and `failed`, including tone and detail text.

- [ ] **Step 2: Run the focused test and verify RED**

Run `npm test -- managementLayout.test.ts`.

Expected: FAIL because the neutral fallback does not yet return the required `正在检查` and `检查失败` presentation.

- [ ] **Step 3: Complete the minimal environment presentation helper**

Return these values:

```ts
checking: { value: "正在检查", detail: "正在读取本地 Agent 与团队配置", tone: "default" }
ready: { value: "已就绪", detail: "Agent 与团队配置可用", tone: "ready" }
failed: { value: "检查失败", detail: "处理错误后可重新检查", tone: "failed" }
preview: { value: "浏览器预览", detail: "请在桌面窗口启动 Run", tone: "warning" }
```

- [ ] **Step 4: Restructure only the New Run branch in `App.tsx`**

Import `ManagementHero`, `PageMetric`, and `environmentSummary`. Replace the existing New Run header/form-card with:

- `ManagementHero` showing environment state;
- `.new-run-grid`;
- `.new-run-task-panel` containing repository and goal;
- `.new-run-team-panel` containing team selector, `RunTeamPreview`, existing issue blocks, frozen-configuration note, and Start Run.

Keep `canStart`, `chooseRepository`, `startRun`, error recovery callbacks, labels, IDs, and values unchanged.

- [ ] **Step 5: Add scoped New Run CSS**

Replace the old `10vh` presentation spacing with `.new-run-view` top padding around 32px. Add the 58/42 grid, compact panel headings, responsive single-column breakpoint, and local issue spacing. Keep `.run-view`, `.run-header`, `.run-grid`, `.panel`, timeline, and inspector behavior unchanged.

- [ ] **Step 6: Verify New Run and regression build**

Run:

```bash
npm test
npm run build
```

Expected: all tests pass; TypeScript and Vite build exit 0.

- [ ] **Step 7: Commit New Run**

```bash
git add apps/archmage-desktop/src/App.tsx apps/archmage-desktop/src/styles.css apps/archmage-desktop/src/managementLayout.test.ts apps/archmage-desktop/src/managementLayout.ts
git commit -m "Polish new Run layout"
```

### Task 3: Redesign Agent Management

**Files:**
- Modify: `apps/archmage-desktop/src/AgentManagement.tsx`
- Modify: `apps/archmage-desktop/src/agents.css`
- Test: `apps/archmage-desktop/src/managementLayout.test.ts`

- [ ] **Step 1: Add failing Agent metric edge-case tests**

Assert disabled instances do not increase verified/unverified/failed counts, unsupported adapters do not increase total supported instances, and enabled ready/unverified/failed Codex instances are counted correctly.

- [ ] **Step 2: Run the focused test and verify RED**

Run `npm test -- managementLayout.test.ts`.

Expected: FAIL until disabled and unsupported filtering matches the design.

- [ ] **Step 3: Implement minimal Agent metric filtering**

Filter `snapshot.instances` by `providerKind === "codex"`; count state buckets only when `enabled` is true; use the supported filtered length for `总实例`.

- [ ] **Step 4: Restructure Agent Management markup**

Use `ManagementHero` with `添加 Codex 实例` in `actions` and four `PageMetric` items in `summary`. Keep the `adding` toggle and all card callbacks unchanged. Retain a compact `本地连接` heading before `.agent-grid`. Replace the large routing heading with `.agent-routing-handoff` containing the existing navigation callback.

- [ ] **Step 5: Update Agent CSS**

Remove obsolete `.agents-header` and summary-card rules. Reduce top/section spacing, preserve two-column supported cards, keep unsupported/new cards, style the compact routing handoff, and retain the existing one-column breakpoint.

- [ ] **Step 6: Run tests and build**

Run `npm test && npm run build` from `apps/archmage-desktop`.

Expected: all tests pass and build exits 0.

- [ ] **Step 7: Commit Agent Management**

```bash
git add apps/archmage-desktop/src/AgentManagement.tsx apps/archmage-desktop/src/agents.css apps/archmage-desktop/src/managementLayout.ts apps/archmage-desktop/src/managementLayout.test.ts
git commit -m "Polish Agent management layout"
```

### Task 4: Redesign Capability Management as master-detail

**Files:**
- Modify: `apps/archmage-desktop/src/CapabilityManagement.tsx`
- Modify: `apps/archmage-desktop/src/capabilities.css`
- Test: `apps/archmage-desktop/src/managementLayout.test.ts`

- [ ] **Step 1: Add failing Capability metric tests**

Use a snapshot with four configurations, one missing instance, one disabled instance, and repeated Skill/Tool guidance. Assert total Capabilities, only valid enabled Codex bindings, and exact Skill/Tool sums.

- [ ] **Step 2: Run the focused test and verify RED**

Run `npm test -- managementLayout.test.ts`.

Expected: FAIL until valid-binding rules and totals match the approved summary.

- [ ] **Step 3: Implement minimal Capability totals**

For each configuration, resolve its bound instance and count it only when enabled and Codex-backed. Sum `skills.length` and `tools.length` across configurations.

- [ ] **Step 4: Restructure Capability markup**

Use `ManagementHero` with Reload in `actions` and four summary metrics. Wrap `.capability-list` and `.cap-editor` in `.capability-workspace`. Keep list buttons and all selection/dirty confirmation logic unchanged. Wrap Skills and Tools guidance blocks in `.cap-guidance-grid` while preserving every existing row input, limit, label, add, and remove callback.

Apply the same Hero shell to preview and loading/error states, with summary omitted when no snapshot exists.

- [ ] **Step 5: Update Capability CSS**

Make `.capability-workspace` a `240px minmax(0, 1fr)` grid. Make `.capability-list` a vertical rail and remove the old four-column card layout. Remove `.cap-editor` top margin, create two-column `.cap-guidance-grid`, and add medium-width stacked workspace/two-column selector plus narrow one-column selector breakpoints.

- [ ] **Step 6: Run tests and build**

Run `npm test && npm run build`.

Expected: all tests pass and build exits 0.

- [ ] **Step 7: Commit Capability Management**

```bash
git add apps/archmage-desktop/src/CapabilityManagement.tsx apps/archmage-desktop/src/capabilities.css apps/archmage-desktop/src/managementLayout.ts apps/archmage-desktop/src/managementLayout.test.ts
git commit -m "Polish Capability management layout"
```

### Task 5: Redesign Team Management around the approved Command Hero

**Files:**
- Modify: `apps/archmage-desktop/src/TeamManagement.tsx`
- Modify: `apps/archmage-desktop/src/teams.css`
- Test: `apps/archmage-desktop/src/managementLayout.test.ts`

- [ ] **Step 1: Add failing Team summary tests**

Assert flow and legacy graphs produce exact capability/route counts and `消息流`/`历史咨询` mode labels, including an empty new flow graph.

- [ ] **Step 2: Run the focused test and verify RED**

Run `npm test -- managementLayout.test.ts`.

Expected: FAIL until empty and legacy summaries are handled.

- [ ] **Step 3: Implement the Team summary helper**

Return `graph.nodes.length`, `graph.edges.length`, and mode label without changing graph validation.

- [ ] **Step 4: Restructure Team Management markup**

Use `ManagementHero` with Create and icon-only Refresh actions. Give Refresh `aria-label="刷新团队"` and `title="刷新团队"`. Put the current-team selector and Team summary in the Hero summary region. Build `.team-workspace` with:

- `.team-config-panel`: name/copy row, integrated legacy explanation, selected-count badge, capability-card grid, entry/terminal selectors;
- `.team-canvas-panel`: heading/summary, existing `TeamCanvas`, selected-edge state, and guidance.

Keep all create, choose, reload, copy, save, dirty, confirmation, graph-editing, and validation logic unchanged.

- [ ] **Step 5: Update Team CSS**

Add the approved 42/58 workspace, compact capability cards with full-label selected state, integrated legacy notice, canvas panel, footer, and responsive single-column behavior. Preserve the internal 1000×360 canvas dimensions, overflow, node positions, edge hit targets, and focus styles.

- [ ] **Step 6: Run tests and build**

Run `npm test && npm run build`.

Expected: all tests pass and build exits 0.

- [ ] **Step 7: Commit Team Management**

```bash
git add apps/archmage-desktop/src/TeamManagement.tsx apps/archmage-desktop/src/teams.css apps/archmage-desktop/src/managementLayout.ts apps/archmage-desktop/src/managementLayout.test.ts
git commit -m "Polish Team management layout"
```

### Task 6: Cross-page verification and cleanup

**Files:**
- Modify only files needed to correct issues found by verification.

- [ ] **Step 1: Run full automated verification**

From `apps/archmage-desktop` run:

```bash
npm test
npm run build
```

From the repository root run:

```bash
git diff --check HEAD~4..HEAD
git status --short
```

Expected: all tests pass, build exits 0, diff check emits no output, and the working tree is clean after final fixes are committed.

- [ ] **Step 2: Inspect the four redesigned pages**

Run the desktop development build in the available environment and inspect wide, medium, and narrow layouts for:

- New Run ready/preview and issue placement;
- Agent ready/unverified/unsupported/add-instance states;
- Capability normal/dirty/unavailable-binding states;
- Team editable-flow and read-only-legacy states.

Confirm Run Detail markup and layout remain unchanged.

- [ ] **Step 3: Verify accessibility interactions**

Keyboard through each page. Confirm logical focus order, visible focus, checkbox/select semantics, icon-only Refresh name/tooltip, and no hover-only control.

- [ ] **Step 4: Commit any verification fixes**

```bash
git add apps/archmage-desktop/src
git commit -m "Polish responsive management page details"
```

Skip this commit when verification required no changes.
