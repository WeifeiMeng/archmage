# 团队工作空间垂直布局实施方案

简体中文 | [English](../../../en/superpowers/plans/2026-10-29-team-workspace-vertical-layout.md)

> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 在消息流画布上方堆叠团队设置，以便两个面板都使用完整的团队管理内容宽度。

**架构：** 保持现有的 React 组件顺序和行为不变。仅将 `.team-workspace` CSS 网格从两列桌面布局转换为一列，并扩展现有 CSS 回归测试以将该布局锁定到位。

**技术堆栈：** React 19、CSS Grid、TypeScript、Vitest、Vite

---

### 任务 1：堆叠团队工作区面板

**文件：**
- 修改：`apps/archmage-desktop/src/teamsStyle.test.ts`
- 修改：`apps/archmage-desktop/src/teams.css:7`

- [x] **第 1 步：编写失败的布局回归测试**

将此案例添加到 `apps/archmage-desktop/src/teamsStyle.test.ts`：

```ts
it("stacks Team settings above the message-flow canvas", () => {
  expect(teamsCss).toMatch(/\.team-workspace\s*\{[^}]*grid-template-columns:\s*minmax\(0, 1fr\);/s);
  expect(teamsCss).not.toMatch(/\.team-workspace\s*\{[^}]*grid-template-columns:\s*minmax\(350px,/s);
});
```

- [x] **第 2 步：运行重点测试并验证其是否失败**

运行：

```bash
cd apps/archmage-desktop && npm test -- teamsStyle.test.ts
```

预期：新案例失败，因为 `.team-workspace` 仍然声明两个桌面列。

- [x] **步骤 3：实现单列工作区**

将 `apps/archmage-desktop/src/teams.css` 中的 `.team-workspace` 规则替换为：

```css
.team-workspace { display: grid; grid-template-columns: minmax(0, 1fr); gap: 14px; margin-top: 14px; align-items: start; }
```

请勿更改 `TeamManagement.tsx`、画布行为或运行详细信息样式。

- [x] **第 4 步：运行集中且完整的验证**

运行：

```bash
cd apps/archmage-desktop
npm test -- teamsStyle.test.ts
npm test
npm run build
git diff --check
```

预期：重点测试通过，所有前端测试通过，生产构建成功，并且 `git diff --check` 报告没有错误。

- [x] **步骤 5：提交实施**

```bash
git add apps/archmage-desktop/src/teams.css apps/archmage-desktop/src/teamsStyle.test.ts docs/superpowers/plans/2026-10-29-team-workspace-vertical-layout.md
git commit -m "Stack Team workspace panels"
```
