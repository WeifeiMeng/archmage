# Agent Provider Grouping Implementation Plan

[简体中文](../../../zh-CN/superpowers/plans/2026-10-30-agent-provider-grouping.md) | English

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Change the Agent management page to display the Codex and DeepSeek Harness groups, and put the added entries into the respective Provider groups.

**Architecture:** Keep the existing API, data model and save process unchanged, and only restructure the React presentation components. The top-level component maintains the Provider currently being added; the pure display component filters instances by Provider and renders groups to verify the structure through server-side static rendering.

**Tech Stack:** React 19, TypeScript, Vitest, existing CSS design variables

---

### Task 1: Provider grouping and directed addition form

**Files:**
- Modify: `apps/archmage-desktop/src/AgentManagement.test.tsx`
- Modify: `apps/archmage-desktop/src/AgentManagement.tsx`

- [ ] **Step 1: Write failing structural tests**

Construct `AgentManagementSnapshot` with Codex, DeepSeek Harness, and unsupported Adapter in `AgentManagement.test.tsx`, statically render the new testable content component, and assert:

```tsx
const html = renderToStaticMarkup(
  <AgentManagementContent
    snapshot={snapshot}
    busy={null}
    addingProvider="deepseek_harness"
    onToggleAdding={() => undefined}
    onRun={async () => true}
    onOpenCapabilities={() => undefined}
  />,
);

expect(html).not.toContain("LOCAL AGENT CONNECTIONS");
expect(html).toContain("添加 Codex");
expect(html).toContain("添加 DeepSeek Harness");
expect(html).toContain("添加 DeepSeek Harness 实例");
expect(html).not.toContain("<select");
```

- [ ] **Step 2: Run the test and confirm that it fails because the component does not exist yet**

Run: `npm test -- --run src/AgentManagement.test.tsx`

Expected: FAIL, prompting `AgentManagementContent` not exported or assertion not satisfied.

- [ ] **Step 3: Implement Provider Grouping**

In `AgentManagement.tsx`:

```tsx
type SupportedProvider = Extract<ProviderKind, "codex" | "deepseek_harness">;

const providerSections: Array<{
  kind: SupportedProvider;
  description: string;
  addLabel: string;
}> = [
  { kind: "codex", description: "Codex CLI 连接", addLabel: "添加 Codex" },
  { kind: "deepseek_harness", description: "官方 Python SDK 连接", addLabel: "添加 DeepSeek Harness" },
];
```

Replace `adding: boolean` with `addingProvider: SupportedProvider | null`. Export `AgentManagementContent`, which renders `ManagementHero` with no title and no Hero operation, and outputs item by item for `providerSections`: group title, secondary add button, instance card of this Provider, empty state, or directed new instance form.

`NewAgentCard` receives fixed `provider: SupportedProvider` instead, removes the internal Provider state with `<select>`, and the header displays `添加 Codex 实例` or `添加 DeepSeek Harness 实例`. Both successful creation and cancellation close the form through existing callbacks.

- [ ] **Step 4: Run the test and confirm it passes**

Run: `npm test -- --run src/AgentManagement.test.tsx`

Expected: PASS。

- [ ] **Step 5: Commit structural changes**

```bash
git add apps/archmage-desktop/src/AgentManagement.tsx apps/archmage-desktop/src/AgentManagement.test.tsx
git commit -m "feat: group Agent instances by provider"
```

### Task 2: Grouping style and complete acceptance

**Files:**
- Modify: `apps/archmage-desktop/src/agents.css`

- [ ] **Step 1: Write a failing style constraint test**

At `AgentManagement.test.tsx` reads `agents.css` and asserts that the new group selector exists and the old Hero emphasis entry selector disappears:

```tsx
const css = readFileSync(new URL("./agents.css", import.meta.url), "utf8");
expect(css).toContain(".agent-provider-section");
expect(css).toContain(".agent-provider-heading");
expect(css).not.toContain(".agents-view .management-hero-actions");
```

- [ ] **Step 2: Run the test and confirm it fails due to missing new styles**

Run: `npm test -- --run src/AgentManagement.test.tsx`

Expected: FAIL, prompting that `.agent-provider-section` is missing.

- [ ] **Step 3: Implement low-emphasis Provider grouping style**

Delete the Hero add button dedicated to the Agent page to emphasize the rules, and add:

```css
.agent-provider-section { display: grid; gap: 10px; }
.agent-provider-heading { display: flex; align-items: end; justify-content: space-between; gap: 18px; }
.agent-provider-heading h2 { margin: 0 0 4px; font-size: 16px; }
.agent-provider-heading p { margin: 0; color: var(--muted); font-size: 11px; }
.agent-provider-empty { padding: 18px; border: 1px dashed var(--border); border-radius: 12px; color: var(--muted); }
```

Allow title area to wrap or align vertically in narrow screen breakpoints, add buttons to continue using existing `.secondary-action`, and do not add accent color overlays.

- [ ] **Step 4: Run full desktop test and build**

Run: `cd apps/archmage-desktop && npm test && npm run build`

Expected: All existing tests are PASS, TypeScript and Vite built successfully.

- [ ] **Step 5: Check the differences and submit**

```bash
git diff --check
git add apps/archmage-desktop/src/AgentManagement.test.tsx apps/archmage-desktop/src/agents.css
git commit -m "style: align Agent actions with provider sections"
```

### Task 3: Final return

**Files:**
- Verify only

- [ ] **Step 1: Run complete project acceptance**

Run: `cargo fmt --all -- --check && cargo test -p archmage-core && (cd apps/archmage-desktop && npm test && npm run build)`

Expected: Rust tests, desktop tests, and production builds all succeed; live tests that require real authentication are still ignored according to the existing settings.

- [ ] **Step 2: Confirm workspace status**

Run: `git status --short && git log --oneline -5`

Expected: There are no uncommitted files and the log contains two implementation commits for this plan.
