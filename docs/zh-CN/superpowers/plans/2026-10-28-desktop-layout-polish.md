# 桌面布局抛光实施计划

简体中文 | [English](../../../en/superpowers/plans/2026-10-28-desktop-layout-polish.md)

> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 围绕批准的命令英雄和自适应工作区布局重新设计团队管理、新运行、代理管理和能力管理，同时保留所有现有行为并保持运行详细信息不变。

**架构：** 引入一个小型表示 `ManagementHero` 原语，用于共享页面标识、页面级操作和摘要。将业务状态和 API 调用保留在其现有所有者中，通过纯导出函数导出紧凑的摘要，并在不更改运行时契约的情况下重组每个页面的标记和作用域 CSS。

**技术堆栈：** React 19、TypeScript、CSS、Lucide React、Vitest 5、React DOM 服务器渲染、Vite 8。

---

## 文件地图

- 创建 `apps/archmage-desktop/src/ManagementHero.tsx`：共享演示英雄和指标标记。
- 创建 `apps/archmage-desktop/src/ManagementHero.test.tsx`：语义 Hero 内容的 SSR 断言。
- 创建 `apps/archmage-desktop/src/managementLayout.ts`：环境、代理、能力和团队状态的纯显示摘要帮助程序。
- 创建 `apps/archmage-desktop/src/managementLayout.test.ts`：所有摘要助手的测试优先覆盖率。
- 修改 `apps/archmage-desktop/src/App.tsx`：仅将 New Run 标记替换为批准的 Hero 和两面板工作区。
- 修改 `apps/archmage-desktop/src/styles.css`：共享 Hero 原语和作用域 New Run 布局；保留运行详细信息选择器。
- 修改 `apps/archmage-desktop/src/AgentManagement.tsx`：命令 Hero、指标、紧凑连接标题和路由切换。
- 修改 `apps/archmage-desktop/src/agents.css`：批准的代理密度和响应式卡片布局。
- 修改 `apps/archmage-desktop/src/CapabilityManagement.tsx`：命令 Hero、派生总计和主从工作区。
- 修改 `apps/archmage-desktop/src/capabilities.css`：垂直功能轨、编辑器网格、平行引导部分和断点。
- 修改 `apps/archmage-desktop/src/TeamManagement.tsx`：命令 Hero、集成旧状态、configuration/canvas 工作区、可访问的刷新操作。
- 修改 `apps/archmage-desktop/src/teams.css`：批准的团队拆分工作区、功能卡、画布面板和响应式布局。

### 任务 1：共享命令 Hero 和摘要助手

**文件：**
- 创建：`apps/archmage-desktop/src/ManagementHero.tsx`
- 创建：`apps/archmage-desktop/src/ManagementHero.test.tsx`
- 创建：`apps/archmage-desktop/src/managementLayout.ts`
- 创建：`apps/archmage-desktop/src/managementLayout.test.ts`
- 修改：`apps/archmage-desktop/src/styles.css`

- [ ] **第 1 步：为共享 Hero 编写失败的 SSR 测试**

使用呈现的测试创建 `ManagementHero.test.tsx`：

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

断言输出包含 `management-hero`、`management-hero-actions`、`page-metric ready`、标题、描述、操作、标签和值。

- [ ] **第 2 步：运行 Hero 测试并验证 RED**

运行：

```bash
npm test -- ManagementHero.test.tsx
```

预期：失败，因为 `ManagementHero.tsx` 不存在。

- [ ] **第 3 步：实现最小共享 Hero**

创建类型化 `ManagementHero` 和 `PageMetric` 组件。`ManagementHero` 呈现语义 `<header className="management-hero">`、复制块、可选操作和可选摘要。`PageMetric` 呈现文本标签并接受 `tone?: "default" | "ready" | "warning" | "failed"`。

- [ ] **第 4 步：运行 Hero 测试并验证绿色**

运行 `npm test -- ManagementHero.test.tsx`。

预期：一个测试文件通过且没有警告。

- [ ] **步骤 5：编写失败的纯总结测试**

使用集中断言创建 `managementLayout.test.ts`：

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

使用 `model.ts` 中的最小类型固定装置。

- [ ] **第 6 步：运行摘要测试并验证 RED**

运行 `npm test -- managementLayout.test.ts`。

预期：失败，因为 `managementLayout.ts` 不存在。

- [ ] **第 7 步：实现摘要助手**

创建纯函数：

```ts
export type EnvironmentState = "checking" | "ready" | "failed" | "preview";
export function environmentSummary(state: EnvironmentState): PageStatus;
export function agentMetrics(snapshot: AgentManagementSnapshot): PageMetricData[];
export function capabilityMetrics(snapshot: CapabilityManagementSnapshot): CapabilityMetricData;
export function teamSummary(graph: TeamGraph): TeamMetricData;
```

仅计算代理总数的受支持 Codex 实例、功能绑定总数的有效启用 Codex 绑定以及团队总数的当前 nodes/edges。

- [ ] **第 8 步：运行共享测试和完整前端测试**

运行：

```bash
npm test -- ManagementHero.test.tsx managementLayout.test.ts
npm test
```

预期：共享测试通过，基线仍然是 17 个现有测试加上新测试。

- [ ] **第 9 步：添加共享 Hero CSS**

添加 `.management-hero`、`.management-hero-copy`、`.management-hero-actions`、`.management-hero-summary`、`.page-metric` 和音调类别的范围规则。使用批准的绿色渐变、紧凑标题、14–16 像素半径和 `max-width: 780px` 换行断点。请勿更改 `.run-header` 或其他运行详细信息选择器。

- [ ] **第 10 步：提交共享基础**

```bash
git add apps/archmage-desktop/src/ManagementHero.tsx apps/archmage-desktop/src/ManagementHero.test.tsx apps/archmage-desktop/src/managementLayout.ts apps/archmage-desktop/src/managementLayout.test.ts apps/archmage-desktop/src/styles.css
git commit -m "Add shared management page layout primitives"
```

### 任务 2：重新设计新运行而不触及运行详细信息

**文件：**
- 修改：`apps/archmage-desktop/src/App.tsx`
- 修改：`apps/archmage-desktop/src/styles.css`
- 测试： `apps/archmage-desktop/src/managementLayout.test.ts`

- [ ] **步骤 1：使用未实现的检查和失败状态扩展环境摘要测试**

任务 1 实施已批准的 `ready` 和 `preview` 案例以及中性回退。为 `checking` 和 `failed` 添加准确的断言，包括语气和详细文本。

- [ ] **步骤 2：运行重点测试并验证 RED**

运行 `npm test -- managementLayout.test.ts`。

预期：失败，因为中立回退尚未返回所需的 `正在检查` 和 `检查失败` 表示。

- [ ] **第3步：完成最小环境演示助手**

返回这些值：

```ts
checking: { value: "正在检查", detail: "正在读取本地 Agent 与团队配置", tone: "default" }
ready: { value: "已就绪", detail: "Agent 与团队配置可用", tone: "ready" }
failed: { value: "检查失败", detail: "处理错误后可重新检查", tone: "failed" }
preview: { value: "浏览器预览", detail: "请在桌面窗口启动 Run", tone: "warning" }
```

- [ ] **步骤 4：仅重构 `App.tsx` 中的 New Run 分支**

导入 `ManagementHero`、`PageMetric` 和 `environmentSummary`。将现有的 New Run header/form-card 替换为：

- `ManagementHero` 显示环境状态；
- `.new-run-grid`；
- `.new-run-task-panel` 包含存储库和目标；
- `.new-run-team-panel` 包含团队选择器、`RunTeamPreview`、现有问题块、冻结配置注释和开始运行。

保持 `canStart`、`chooseRepository`、`startRun`、错误恢复回调、标签、ID 和值不变。

- [ ] **步骤 5：添加范围内的 New Run CSS**

将旧的 `10vh` 演示间距替换为 `.new-run-view` 顶部填充（32 像素左右）。添加 58/42 网格、紧凑面板标题、响应式单列断点和本地问题间距。保持 `.run-view`、`.run-header`、`.run-grid`、`.panel`、时间线和检查器行为不变。

- [ ] **第 6 步：验证新运行和回归构建**

运行：

```bash
npm test
npm run build
```

预期：所有测试均通过；TypeScript 和 Vite 构建退出 0。

- [ ] **第 7 步：提交新运行**

```bash
git add apps/archmage-desktop/src/App.tsx apps/archmage-desktop/src/styles.css apps/archmage-desktop/src/managementLayout.test.ts apps/archmage-desktop/src/managementLayout.ts
git commit -m "Polish new Run layout"
```

### 任务 3：重新设计代理管理

**文件：**
- 修改：`apps/archmage-desktop/src/AgentManagement.tsx`
- 修改：`apps/archmage-desktop/src/agents.css`
- 测试： `apps/archmage-desktop/src/managementLayout.test.ts`

- [ ] **步骤 1：添加失败的代理指标边缘情况测试**

断言禁用的实例不会增加 verified/unverified/failed 计数，不受支持的适配器不会增加支持的实例总数，并且启用的 ready/unverified/failed Codex 实例计数正确。

- [ ] **步骤 2：运行重点测试并验证 RED**

运行 `npm test -- managementLayout.test.ts`。

预期：失败，直到禁用和不受支持的过滤与设计匹配。

- [ ] **步骤 3：实施最小代理指标过滤**

通过 `providerKind === "codex"` 筛选 `snapshot.instances`；仅当 `enabled` 为 true 时才对状态桶进行计数；使用 `总实例` 支持的过滤长度。

- [ ] **步骤 4：重构代理管理标记**

将 `ManagementHero` 与 `actions` 中的 `添加 Codex 实例` 以及 `summary` 中的四个 `PageMetric` 项一起使用。保持 `adding` 切换和所有卡回调不变。在 `.agent-grid` 之前保留紧凑的 `本地连接` 标题。将大路由标题替换为包含现有导航回调的 `.agent-routing-handoff`。

- [ ] **第 5 步：更新代理 CSS**

删除过时的 `.agents-header` 和摘要卡规则。减少 top/section 间距，保留两列支持的卡，保留 unsupported/new 卡，设计紧凑的路由切换，并保留现有的一列断点。

- [ ] **第 6 步：运行测试并构建**

从 `apps/archmage-desktop` 运行 `npm test && npm run build`。

预期：所有测试均通过并且构建退出 0。

- [ ] **第 7 步：提交代理管理**

```bash
git add apps/archmage-desktop/src/AgentManagement.tsx apps/archmage-desktop/src/agents.css apps/archmage-desktop/src/managementLayout.ts apps/archmage-desktop/src/managementLayout.test.ts
git commit -m "Polish Agent management layout"
```

### 任务 4：将能力管理重新设计为主从结构

**文件：**
- 修改：`apps/archmage-desktop/src/CapabilityManagement.tsx`
- 修改：`apps/archmage-desktop/src/capabilities.css`
- 测试： `apps/archmage-desktop/src/managementLayout.test.ts`

- [ ] **步骤 1：添加失败的能力指标测试**

使用具有四种配置、一种缺失实例、一种禁用实例和重复 Skill/Tool 指导的快照。断言总功能、仅有效启用的 Codex 绑定以及准确的 Skill/Tool 总和。

- [ ] **步骤 2：运行重点测试并验证 RED**

运行 `npm test -- managementLayout.test.ts`。

预期：失败，直到有效的约束规则和总数与批准的摘要匹配。

- [ ] **第 3 步：实施最小能力总计**

对于每个配置，解析其绑定实例并仅在启用且受 Codex 支持时对其进行计数。跨配置求和 `skills.length` 和 `tools.length`。

- [ ] **第4步：重构能力标记**

将 `ManagementHero` 与 `actions` 中的重新加载和四个摘要指标结合使用。将 `.capability-list` 和 `.cap-editor` 包裹在 `.capability-workspace` 中。保持列表按钮和所有 selection/dirty 确认逻辑不变。将技能和工具指导块封装在 `.cap-guidance-grid` 中，同时保留每个现有行输入、限制、标签、添加和删除回调。

将相同的 Hero shell 应用于预览和 loading/error 状态，当不存在快照时，将省略摘要。

- [ ] **第 5 步：更新功能 CSS**

将 `.capability-workspace` 设为 `240px minmax(0, 1fr)` 网格。将 `.capability-list` 设为垂直导轨并删除旧的四列卡片布局。删除 `.cap-editor` 上边距，创建两列 `.cap-guidance-grid`，并添加中等宽度的堆叠 workspace/two-column 选择器和窄的一列选择器断点。

- [ ] **第 6 步：运行测试并构建**

运行 `npm test && npm run build`。

预期：所有测试均通过并且构建退出 0。

- [ ] **第 7 步：提交能力管理**

```bash
git add apps/archmage-desktop/src/CapabilityManagement.tsx apps/archmage-desktop/src/capabilities.css apps/archmage-desktop/src/managementLayout.ts apps/archmage-desktop/src/managementLayout.test.ts
git commit -m "Polish Capability management layout"
```

### 任务 5：围绕批准的指挥英雄重新设计团队管理

**文件：**
- 修改：`apps/archmage-desktop/src/TeamManagement.tsx`
- 修改：`apps/archmage-desktop/src/teams.css`
- 测试： `apps/archmage-desktop/src/managementLayout.test.ts`

- [ ] **第 1 步：添加失败的团队摘要测试**

断言流和旧图生成精确的 capability/route 计数和 `消息流`/`历史咨询` 模式标签，包括空的新流程图。

- [ ] **步骤 2：运行重点测试并验证 RED**

运行 `npm test -- managementLayout.test.ts`。

预期：在处理空和旧摘要之前失败。

- [ ] **步骤 3：实施团队摘要助手**

返回 `graph.nodes.length`、`graph.edges.length` 和模式标签，而不更改图形验证。

- [ ] **步骤 4：重组团队管理标记**

将 `ManagementHero` 与“创建”和“仅图标刷新”操作结合使用。刷新 `aria-label="刷新团队"` 和 `title="刷新团队"`。将当前团队选择器和团队摘要放入英雄摘要区域。使用以下命令构建 `.team-workspace`：

- `.team-config-panel`：name/copy 行、集成旧版解释、选定计数徽章、功能卡网格、entry/terminal 选择器；
- `.team-canvas-panel`： heading/summary、现有 `TeamCanvas`、选定边缘状态和指导。

保持所有创建、选择、重新加载、复制、保存、脏、确认、图形编辑和验证逻辑不变。

- [ ] **第 5 步：更新团队 CSS**

添加经批准的 42/58 工作区、具有全标签选定状态的紧凑功能卡、集成旧通知、画布面板、页脚和响应式单列行为。保留内部 1000×360 画布尺寸、溢出、节点位置、边缘命中目标和焦点样式。

- [ ] **第 6 步：运行测试并构建**

运行 `npm test && npm run build`。

预期：所有测试均通过并且构建退出 0。

- [ ] **第 7 步：进行团队管理**

```bash
git add apps/archmage-desktop/src/TeamManagement.tsx apps/archmage-desktop/src/teams.css apps/archmage-desktop/src/managementLayout.ts apps/archmage-desktop/src/managementLayout.test.ts
git commit -m "Polish Team management layout"
```

### 任务6：跨页面验证和清理

**文件：**
- 仅修改纠正验证发现的问题所需的文件。

- [ ] **第 1 步：运行全自动验证**

从 `apps/archmage-desktop` 运行：

```bash
npm test
npm run build
```

从存储库根目录运行：

```bash
git diff --check HEAD~4..HEAD
git status --short
```

预期：所有测试都通过，构建退出 0，差异检查不发出输出，并且提交最终修复后worktree是干净的。

- [ ] **第 2 步：检查四个重新设计的页面**

在可用环境中运行桌面开发构建并检查宽、中和窄布局：

- 新运行 ready/preview 并发行安置；
- 代理 ready/unverified/unsupported/add-instance 表示；
- 能力 normal/dirty/unavailable-binding 状态；
- 团队可编辑流程和只读旧状态。

确认运行详细信息标记和布局保持不变。

- [ ] **第 3 步：验证可访问性交互**

键盘浏览每一页。确认逻辑焦点顺序、可见焦点、checkbox/select 语义、仅图标刷新 name/tooltip，并且没有仅悬停控件。

- [ ] **第 4 步：提交所有验证修复**

```bash
git add apps/archmage-desktop/src
git commit -m "Polish responsive management page details"
```

当验证不需要更改时跳过此提交。
