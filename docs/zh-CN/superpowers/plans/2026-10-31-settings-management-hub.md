# 设置管理中心实施计划

简体中文 | [English](../../../en/superpowers/plans/2026-10-31-settings-management-hub.md)

> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 将代理、功能和团队管理移至全屏设置中心，同时仅在主侧边栏中保留运行导航。

**架构：** 保留现有的 `AppView` 值和脏出口防护，但将每个非运行视图视为设置空间的一部分。重点突出的 `SettingsShell` 拥有“设置”标题和内部类别导航；现有的管理组件保持不变并呈现为其内容。

**技术堆栈：** React 19、TypeScript、Vitest、现有 CSS 主题变量。

---

### 任务 1：添加 Settings shell 合约

**文件：**
- 创建：`apps/archmage-desktop/src/SettingsShell.test.tsx`
- 创建：`apps/archmage-desktop/src/SettingsShell.tsx`
- 修改：`apps/archmage-desktop/src/settings.css`

- [x] **第 1 步：编写失败的 shell 测试**

使用 `activeView="agents"` 渲染 `SettingsShell` 并断言它包含“返回主界面”、所有四个类别标签、活动导航状态及其子内容。

- [x] **步骤 2：运行重点测试并验证 RED**

运行： `npm test -- SettingsShell.test.tsx`

预期：失败，因为 `SettingsShell` 不存在。

- [x] **第 3 步：实现最小的 Settings shell**

创建具有此公共形状的组件：

```ts
type SettingsShellProps = {
  activeView: Exclude<AppView, "runs">;
  onNavigate: (view: Exclude<AppView, "runs">) => void;
  onBack: () => void;
  children: ReactNode;
};
```

渲染一个带有后退按钮、分类的内部导航栏和灵活的内容区域的顶部栏。添加响应式 CSS，将栏杆折叠成窄窗口上的水平类别栏。

- [x] **第 4 步：运行重点测试并验证绿色**

运行： `npm test -- SettingsShell.test.tsx`

预期：通过。

### 任务 2：通过“设置”路由管理页面

**文件：**
- 修改：`apps/archmage-desktop/src/sidebarNavigation.test.ts`
- 修改：`apps/archmage-desktop/src/App.tsx`

- [x] **第 1 步：编写失败的导航断言**

更新 `sidebarNavigation.test.ts` 以断言 `App.tsx` 不再包含主侧栏中的三个管理按钮，并且它对非运行视图使用 `SettingsShell`。

- [x] **步骤 2：运行重点测试并验证 RED**

运行： `npm test -- sidebarNavigation.test.ts`

预期：失败，而旧的主侧边栏按钮仍然存在。

- [x] **第三步：实现设置-空间路由**

在 `App.tsx` 中：

- 仅当 `view === "runs"` 时渲染主侧边栏。
- 使用 `SettingsShell` 包装 `settings`、`agents`、`capabilities` 和 `teams` 内容。
- 将外壳后退按钮穿过 `navigate("runs", null)`，以便脏防护装置保持有效。
- 通过 `navigate(nextView)` 路由内部类别更改。
- 将直接 `setView("capabilities")` 和 `setView("teams")` 管理链接替换为 `navigate(...)`。
- 保持关闭窗口脏引用和现有确认消息不变。

保持现有的Run工作区布局不变；独立的 `SettingsShell` 提供自己的全宽布局。

- [x] **第 4 步：运行导航和脏防护测试**

运行： `npm test -- sidebarNavigation.test.ts model.test.ts teamGraph.test.ts`

预期：通过。

### 任务3：回归验证

**文件：**
- 仅当验证暴露上面已列出的文件中的缺陷时才进行修改。

- [x] **第 1 步：运行桌面测试套件**

运行： `npm test`

预期：所有测试均通过。

- [x] **第 2 步：构建桌面前端**

运行： `npm run build`

预期：TypeScript 和 Vite 构建成功完成。

- [x] **第 3 步：查看最终差异**

运行： `git diff --check && git status --short`

预期：没有空格错误；仅存在计划文件以及用户预先存在的未跟踪文件。

- [x] **第 4 步：提交实施**

提交消息： `feat: move management pages into settings`
