简体中文 | [English](../../../en/superpowers/plans/2026-10-30-agent-provider-grouping.md)

# Agent Provider Grouping Implementation Plan


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 Agent 管理页改为 Codex 与 DeepSeek Harness 分组展示，并把添加入口放入各自 Provider 分组。

**Architecture:** 保持现有 API、数据模型和保存流程不变，只重组 React 展示组件。顶层组件维护当前正在添加的 Provider；纯展示组件按 Provider 过滤实例并渲染分组，以便通过服务端静态渲染验证结构。

**Tech Stack:** React 19、TypeScript、Vitest、现有 CSS 设计变量

---

### Task 1: Provider 分组与定向添加表单

**Files:**
- Modify: `apps/archmage-desktop/src/AgentManagement.test.tsx`
- Modify: `apps/archmage-desktop/src/AgentManagement.tsx`

- [ ] **Step 1: 写失败的结构测试**

在 `AgentManagement.test.tsx` 中构造包含 Codex、DeepSeek Harness 和未支持 Adapter 的 `AgentManagementSnapshot`，静态渲染新的可测试内容组件，并断言：

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

- [ ] **Step 2: 运行测试并确认因组件尚不存在而失败**

Run: `npm test -- --run src/AgentManagement.test.tsx`

Expected: FAIL，提示 `AgentManagementContent` 未导出或断言不满足。

- [ ] **Step 3: 实现 Provider 分组**

在 `AgentManagement.tsx` 中：

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

将 `adding: boolean` 替换为 `addingProvider: SupportedProvider | null`。导出 `AgentManagementContent`，由它渲染无眉题、无 Hero 操作的 `ManagementHero`，并对 `providerSections` 逐项输出：分组标题、次级添加按钮、该 Provider 的实例卡片、空状态或定向的新实例表单。

`NewAgentCard` 改为接收固定 `provider: SupportedProvider`，删除内部 Provider state 与 `<select>`，标题显示 `添加 Codex 实例` 或 `添加 DeepSeek Harness 实例`。创建成功和取消均通过现有回调关闭表单。

- [ ] **Step 4: 运行测试并确认通过**

Run: `npm test -- --run src/AgentManagement.test.tsx`

Expected: PASS。

- [ ] **Step 5: 提交结构改动**

```bash
git add apps/archmage-desktop/src/AgentManagement.tsx apps/archmage-desktop/src/AgentManagement.test.tsx
git commit -m "feat: group Agent instances by provider"
```

### Task 2: 分组样式与完整验收

**Files:**
- Modify: `apps/archmage-desktop/src/agents.css`

- [ ] **Step 1: 写失败的样式约束测试**

在 `AgentManagement.test.tsx` 读取 `agents.css` 并断言新的分组选择器存在，且旧 Hero 强调入口选择器消失：

```tsx
const css = readFileSync(new URL("./agents.css", import.meta.url), "utf8");
expect(css).toContain(".agent-provider-section");
expect(css).toContain(".agent-provider-heading");
expect(css).not.toContain(".agents-view .management-hero-actions");
```

- [ ] **Step 2: 运行测试并确认缺少新样式而失败**

Run: `npm test -- --run src/AgentManagement.test.tsx`

Expected: FAIL，提示缺少 `.agent-provider-section`。

- [ ] **Step 3: 实现低强调 Provider 分组样式**

删除 Agent 页面专用的 Hero 添加按钮强调规则，新增：

```css
.agent-provider-section { display: grid; gap: 10px; }
.agent-provider-heading { display: flex; align-items: end; justify-content: space-between; gap: 18px; }
.agent-provider-heading h2 { margin: 0 0 4px; font-size: 16px; }
.agent-provider-heading p { margin: 0; color: var(--muted); font-size: 11px; }
.agent-provider-empty { padding: 18px; border: 1px dashed var(--border); border-radius: 12px; color: var(--muted); }
```

在窄屏断点中允许标题区换行或纵向排列，添加按钮继续使用现有 `.secondary-action`，不添加强调色覆盖。

- [ ] **Step 4: 运行桌面端完整测试与构建**

Run: `cd apps/archmage-desktop && npm test && npm run build`

Expected: 现有测试全部 PASS，TypeScript 与 Vite 构建成功。

- [ ] **Step 5: 检查差异并提交**

```bash
git diff --check
git add apps/archmage-desktop/src/AgentManagement.test.tsx apps/archmage-desktop/src/agents.css
git commit -m "style: align Agent actions with provider sections"
```

### Task 3: 最终回归

**Files:**
- Verify only

- [ ] **Step 1: 运行完整项目验收**

Run: `cargo fmt --all -- --check && cargo test -p archmage-core && (cd apps/archmage-desktop && npm test && npm run build)`

Expected: Rust 测试、桌面端测试和生产构建全部成功；需要真实认证的 live 测试仍按既有设置忽略。

- [ ] **Step 2: 确认工作区状态**

Run: `git status --short && git log --oneline -5`

Expected: 没有未提交文件，日志包含本计划的两个实现提交。
