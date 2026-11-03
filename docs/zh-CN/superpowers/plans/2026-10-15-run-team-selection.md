简体中文 | [English](../../../en/superpowers/plans/2026-10-15-run-team-selection.md)

# 运行团队选拔实施计划


> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 让团队选择和组成成为 New Run 的焦点，仅显示相关的启动问题。

**架构：** 从现有客户端模型中导出只读团队摘要和选定的功能绑定问题。保持现有的运行请求和后端预检不变。用紧凑的团队预览和条件诊断替换三个持久状态卡。

**技术堆栈：** React、TypeScript、Vitest、CSS。

---

### 任务 1：团队特定的表单状态

**文件：** `apps/archmage-desktop/src/model.ts`、`apps/archmage-desktop/src/model.test.ts`

- [x] 添加仅选择Coder和Reviewer且未绑定Designer/Tester的测试；预计不会出现任何具有约束力的问题。添加针对不可用的选定绑定的测试。
- [x] 运行 `npm test --prefix apps/archmage-desktop -- src/model.test.ts` 并确认新测试失败。
- [x] 扩展 `runBindingIssues(snapshot, capabilities)`，以便省略的功能保留当前的所有角色行为，并提供的功能限制对所选团队的验证。
- [x] 添加对包含成员、边、入口和终端的流程团队摘要的测试；实现一个直接从 `Team.graph` 派生的类型化帮助器。
- [x] 再次运行重点测试并确认它们通过。

### 任务 2：新建运行表单

**文件：** `apps/archmage-desktop/src/App.tsx`、 `apps/archmage-desktop/src/RunTeamPreview.tsx`、 `apps/archmage-desktop/src/RunTeamPreview.test.tsx`、 `apps/archmage-desktop/src/runTeam.css`、 `apps/archmage-desktop/src/capabilities.css`、 `apps/archmage-desktop/src/agents.css`

- [x] 仅在成功发出新的团队列表请求后才解析所选团队，并仅使用其节点进行启动门控。
- [x] 使用成员名称、流边缘和流 entry/terminal 标签或旧咨询标签渲染一个紧凑的团队预览。
- [x] 删除持久性测试器、绑定成功和运行时就绪卡。仅在可操作时显示无团队、团队加载、绑定和运行时错误。在缺少所需输入或准备状态时保留禁用的“开始”按钮。
- [x] 更新标题和目标占位符以引用所选团队而不是四个固定角色。保留标签、可读对比度和键盘焦点样式。

### 任务 3：验证

**文件：** `docs/superpowers/specs/2026-10-15-run-team-selection-design.md`、`docs/superpowers/plans/2026-10-15-run-team-selection.md`

- [x] 运行 `npm test --prefix apps/archmage-desktop`、`npm run build --prefix apps/archmage-desktop` 和 `git diff --check`。
- [x] 查看最终差异并仅在 `main` 上提交此范围内的 UI 和文档工作。
