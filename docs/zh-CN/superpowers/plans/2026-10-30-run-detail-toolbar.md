简体中文 | [English](../../../en/superpowers/plans/2026-10-30-run-detail-toolbar.md)

# 运行详细工具栏清理实施计划


> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 使用批准的分组工具栏替换运行详细信息标题操作，并删除多余的冻结团队拓扑面板。

**架构：** 将三个标头控件提取到一个具有自己的 CSS 和渲染测试的集中展示的 React 组件中。将所有现有回调和运行状态决策保留在 `App.tsx` 中；该页面仅将显示委托给新组件，并停止渲染重复的拓扑面板。

**技术堆栈：** React 19、TypeScript、Lucide React、CSS、Vitest、React DOM 服务器渲染。

---

### 任务 1：添加分组的运行标题工具栏

**文件：**
- 创建：`apps/archmage-desktop/src/RunHeaderActions.tsx`
- 创建：`apps/archmage-desktop/src/RunHeaderActions.css`
- 创建：`apps/archmage-desktop/src/RunHeaderActions.test.tsx`
- 修改：`apps/archmage-desktop/src/App.tsx:1-45,334-342`

- [ ] **第 1 步：编写失败的组件测试**

使用静态渲染测试创建 `RunHeaderActions.test.tsx`，以验证分组的工具栏标签、所有三个活动运行操作、禁用的工作区行为以及省略非活动运行的取消操作：

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

- [ ] **步骤 2：运行测试并验证 RED**

运行： `cd apps/archmage-desktop && npm test -- RunHeaderActions.test.tsx`

预期：失败，因为 `./RunHeaderActions` 不存在。

- [ ] **第3步：实现工具栏组件**

创建 `RunHeaderActions.tsx`：

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

使用更高特异性的运行标头选择器创建 `RunHeaderActions.css`，以便无论 CSS 导入顺序如何，它都可以安全地覆盖旧的基本样式：

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

更新 `App.tsx` 以导入 `RunHeaderActions`，删除仅工具栏图标导入，并将内联 `.header-actions` JSX 替换为：

```tsx
<RunHeaderActions
  workspaceAvailable={Boolean(snapshot.workspace)}
  cancellable={isActive(snapshot.state)}
  onRefresh={onRefresh}
  onReveal={onReveal}
  onCancel={onCancel}
/>
```

- [ ] **步骤 4：运行组件测试并验证绿色**

运行： `cd apps/archmage-desktop && npm test -- RunHeaderActions.test.tsx`

预期：2 项测试通过。

- [ ] **第 5 步：提交工具栏实施**

```bash
git add apps/archmage-desktop/src/App.tsx apps/archmage-desktop/src/RunHeaderActions.tsx apps/archmage-desktop/src/RunHeaderActions.css apps/archmage-desktop/src/RunHeaderActions.test.tsx
git commit -m "style: group Run header actions"
```

### 任务 2：删除重复的冻结拓扑面板

**文件：**
- 修改：`apps/archmage-desktop/src/App.tsx:340-343`
- 创建：`apps/archmage-desktop/src/RunDetailLayout.test.ts`

- [ ] **第 1 步：编写失败的布局回归测试**

使用存储库现有的源布局测试模式创建 `RunDetailLayout.test.ts`：

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

- [ ] **步骤 2：运行测试并验证 RED**

运行： `cd apps/archmage-desktop && npm test -- RunDetailLayout.test.ts`

预期：失败，因为 `App.tsx` 仍包含冻结的拓扑标题和直接边缘渲染。

- [ ] **第 3 步：仅删除重复的面板**

从 `RunWorkbench` 中删除此 JSX：

```tsx
{snapshot.team && <section className="panel team-run-panel"><div className="panel-title"><div><span>冻结的团队拓扑</span><small>{snapshot.team.name} · v{snapshot.team.revision}</small></div></div><div className="team-run-edges">{snapshot.team.graph.edges.map((edge) => <span key={`${edge.from}-${edge.to}`}>{edge.from} → {edge.to}</span>)}</div></section>}
```

保持以下 `FlowProgress` / `ConsultationProgress` 分支不变。

- [ ] **步骤 4：运行布局测试并验证绿色**

运行： `cd apps/archmage-desktop && npm test -- RunDetailLayout.test.ts`

预期：1 次测试通过。

- [ ] **步骤 5：提交拓扑清理**

```bash
git add apps/archmage-desktop/src/App.tsx apps/archmage-desktop/src/RunDetailLayout.test.ts
git commit -m "style: remove duplicate Run topology panel"
```

### 任务 3：验证完整的桌面更改

**文件：**
- 仅验证；没有计划的源更改。

- [ ] **第 1 步：运行所有桌面测试**

运行： `cd apps/archmage-desktop && npm test`

预期：所有 Vitest 文件均通过且零失败。

- [ ] **第 2 步：构建桌面前端**

运行： `cd apps/archmage-desktop && npm run build`

预期：TypeScript 和 Vite 完成，退出代码为 0。

- [ ] **第 3 步：检查实时运行详细信息页面**

打开 Tauri 开发应用程序中活动的运行详细信息页面并验证：

- “刷新/工作区/取消”在一个分组表面中停留在一行上。
- “取消”仍然具有明显的破坏性，但从属于“运行”标题。
- 冻结拓扑面板不存在。
- 团队 name/version、功能进度、进度消息、时间表和检查员仍然可见。

- [ ] **步骤 4：记录最终存储库状态**

运行： `git status --short && git log -3 --oneline`

预期：仅保留用户预先存在的未跟踪文件；两个实现提交位于 `HEAD`。
