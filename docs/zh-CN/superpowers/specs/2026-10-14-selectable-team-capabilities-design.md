简体中文 | [English](../../../en/superpowers/specs/2026-10-14-selectable-team-capabilities-design.md)

# 可选择 Team Capability 设计

**状态：** 已批准在 `main` 实施的方向。

## 范围

创建 Team 时，用户从现有四个 Capability 中至少选择两个，再选择成员之间的有向关系。用户自行创建 Team；系统不再额外 seed Team。已有四成员 Team 与历史 Run 继续有效。

首版保留固定语义顺序 Designer → Coder → Tester → Reviewer。允许所选成员之间的任意前向边。已保存图必须是单个连通有向无环图：每个成员都参与一条边，并且所有成员都能从唯一入口到达。禁止重复边、自环、反向边。关系保持当前含义：下游在执行自己的主任务前评审上游交付；兄弟节点可并发咨询。主任务因共享同一 worktree，仍按语义顺序串行。

## Runtime

新 Run 只冻结所选 Capability、其绑定、配置和图。Scheduler 根据图的入边而不是旧四节点主干推导 ready。拥有出边的源节点必须先得到批准，下游主任务才能开始。最后一个所选节点决定完成；若包含 Reviewer，其状态就是 Run outcome，否则成功完成为 `approved`，任何 blocked/failed 节点都会使 Run 失败。

Prompt 输入取决于存在哪些上游角色。Designer 使用 goal 与 repository view。Coder 有 Design 时使用 Design，否则使用 goal 与 repository view。Tester 有 Coder 时使用 implementation 与 ChangeSet，否则使用 goal 与 repository view；存在 Design 时也包含 Design。Reviewer 接收可用的 Design、implementation/ChangeSet 与 Test result。没有 Tester 时，Reviewer 可依据现有证据批准，但不得声称测试已发生。内置角色 prompt 解释可选输入，输出 JSON Schema 不变。

出边咨询的 Review 反馈会让生产 Capability 重跑，最多三次 revision。最终 Reviewer `changes_requested` 会重跑最近的上游非 Reviewer Capability 及语义顺序中之后的所有成员，同样最多三次。对 `Coder → Reviewer` 即重跑 Coder、Reviewer；对原四成员 Team 仍为 Coder、Tester、Reviewer。没有 Team 的旧 Run 定义保持 legacy 行为。

## 桌面端与校验

Team 创建从空图开始。用户选择 Capability、切换所选成员间的前向关系，并在画布上排列节点。移除成员也移除其边。页面解释有效图规则、禁用控件、保存冲突，以及咨询与主任务的区别。已有 Team 也按同一规则编辑。浏览器预览绝不虚构已持久化 Team。

后端校验是权威。无效图在持久化与创建 Run 前被拒绝；未选择 Capability 的不支持或禁用 Agent 绑定不会阻塞 Run。测试覆盖图选择/拒绝、持久化往返、动态 Run 节点、Scheduler、Coder→Reviewer Context/outcome 与四节点兼容性。真实 Codex 验收与确定性测试分开。
