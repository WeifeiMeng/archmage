简体中文 | [English](../../../en/superpowers/specs/2026-10-14-message-flow-teams-design.md)

# 消息流 Team 设计

跨 Provider 的实际运行语义见[跨 Provider 的 Capability 交付](../../architecture/cross-provider-handoff.md)。

**状态：** 已实现。[实施计划](../plans/2026-10-14-message-flow-teams.md)记录验证历史。仅对新消息流 Team，本设计取代[可选择 Team Capability](2026-10-14-selectable-team-capabilities-design.md)中的关系与调度语义。

## 目的与范围

Team 是所选 Capability 的有向图。箭头承载交付与反馈，而非咨询权限或前置条件。每次执行后，Capability 的真实 Agent 选择一个或多个已连接接收方。每条交付消息都会触发接收方的一次全新执行；消息不会隐式合并。允许 Coder → Reviewer → Coder 等循环。Team 显式指定一个入口 Capability 和一个有权发出终止信号的 Capability。用户自行创建 Team；系统不会 seed Coder–Reviewer Team。

首版保留四种内置 Capability 类型和每个 Capability 配置一个 Agent 绑定。不会增加任意节点类型、可编辑边条件、通用 Workflow 表达式语言，也不允许对同一 Git worktree 并发写入。

## Team 配置与画布

新消息流 Team 至少选择两个不同 Capability，指定其中的入口和终止信号发出者，并连接有向边。自环和重复有向边无效；相反方向的边彼此独立，可同时存在。每个 Capability 必须从入口可达，且必须有通向终止发出者的路径。这些是拓扑校验，不是执行依赖。终止发出者也可拥有出边，仅在其选择继续时使用。入口收到 Run goal 作为第一条消息。

在画布上从输出 handle 拖到另一节点输入 handle 创建边；临时箭头跟随指针，释放到空白或按 Escape 取消。相反方向的边显示为独立曲线和箭头。选择边后可删除。键盘用户可先聚焦源 handle，再聚焦目标 handle 创建同样的边。节点移动与 handle 拖动分离。保留成员选择器，移除关系 checkbox 列表。入口和终止选项从成员中选择并清晰显示。无效图保存时给出具体原因。移除成员时同时移除关联边，必要时清空入口/终止选择。

## 持久消息流 Runtime

消息流 Run 冻结 Team 图、入口/终止选择、Agent 绑定、Capability 指引、仓库、目标和步数上限，并为入口创建初始 delivery。每条 delivery 有 ID、源 Attempt（初始消息除外）、目标 Capability、payload Artifact 引用和 sequence。队列与状态持久化，因此路由和交付可审计，崩溃恢复也能识别未完成工作。每条开始执行的 delivery 恰好触发一次目标执行，即使同一目标已有其他排队消息。共享 Git worktree 上的执行串行；选择多个接收方会创建独立 delivery，不会合并。

真实 Agent 在同一次执行中返回角色特定 deliverable 与结构化 routing。继续时，`next` 必须是该节点出边邻居中非空、无重复的子集。只有配置的终止发出者可返回 `complete`；此时不能有接收方，并在提交 deliverable 后完成 Run。blocked 或无效 deliverable 依照现有契约使 Attempt 失败。非法路由（未知/非邻接接收方、重复接收方、无权 `complete` 或空 continue）会显式使 Run 失败，绝不静默修正。角色 deliverable 仍严格校验；routing 单独校验，并与已提交 Attempt 原子持久化后才让新 delivery 可运行。

每次执行看到触发它的 delivery、goal/repository Context 及该 delivery 引用的已提交 Artifact。因此 revision 反馈作为显式 incoming message 到达，而不是倒转上游节点状态。角色指引允许 Coder 从 goal 或反馈实现，也让 Reviewer 在不虚构缺失测试证据的情况下返回 revision 请求或完成信号。当 Reviewer 是终止发出者时，`approved` 必须与 `complete` 一致，`changes_requested` 必须沿至少一条配置边继续。其他角色作为终止发出者时使用相同显式 routing signal，并继续受其角色契约约束。

每个已提交决策后，Runtime 消费下一条 queued delivery。成功终止不会丢弃已提交历史；其他 queued delivery 标记为 `skipped_due_terminal`，不再启动，Run 完成。冻结的默认 24 步上限约束循环；达到上限时以 `flow_step_limit_reached` 失败并保留历史。取消、中断与进程 ownership 保持现有安全行为。

在 Coder → Reviewer → Coder 中，Coder 向 Reviewer 提交 ChangeSet，Reviewer 返回意见并路由给 Coder。该 delivery 以意见和先前 Artifact 触发新的 Coder 执行。Coder 修订后再次交给 Reviewer；Reviewer 批准后发出 `complete`，而不是回传，Run 结束。

## 兼容与迁移

已有持久化 Team 图和冻结 Run 保留 legacy 咨询/Scheduler 语义。带版本的消息流 Team 表示区分新模式，不重新解释旧 JSON 或历史 Event。现有默认 Team 保持 legacy。Team 页面标记 legacy Team，并提供显式复制/转换操作，要求重新选择入口与终止；操作不修改原 Team，也不自动 seed 新 Team。新建 Team 使用消息流模式。新消息流 Run 只使用 delivery queue，不再同时运行 legacy Review gate 或最终 Reviewer 回退逻辑。

## 验证

聚焦测试覆盖画布拖动/取消/删除、双向边渲染、包含循环/可达性/终止路径的拓扑校验、持久化与迁移、每次触发一条 delivery、多接收方、反馈循环、终止授权与 Reviewer 状态一致性、步数上限，以及 legacy Team/Run 兼容性。确定性测试不能替代真实 Codex 验收。真实验收由用户创建 Coder ↔ Reviewer Team，使用真实 Codex，验证至少一轮反馈和终止批准，并检查持久化 delivery 与 Artifact。
