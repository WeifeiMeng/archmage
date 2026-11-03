简体中文 | [English](../../../en/superpowers/specs/2026-09-30-team-management-design.md)

# Team 管理与 Review 门禁拓扑设计

**状态：** 已批准方向，首个可运行 Team 版本的实施范围

## 产品边界

Agent 管理负责本地 Provider 实例与连接。Capability 管理负责 Designer、Coder、Tester、Reviewer 四个逻辑工作者；每个都有内置 persona/prompt 和一个 Agent 绑定。它是列表/详情页，不是拓扑视图。本版本不展示或强制每个 Capability 的 Skill scope。

Team 管理负责由这些 Capability 组成的命名、持久化安排。启动 Run 时选择 Team，并将其冻结进 Run。画布不只是示意：每条已保存边都会触发真实下游咨询，批准结果控制下一执行步骤。

## 首版拓扑约束

- 每种内置 Capability 恰好一个。Designer 是唯一入口；Coder、Tester、Reviewer 保持当前执行契约。
- 必需执行/咨询主干为 `Designer → Coder → Tester → Reviewer`，三条边不可移除，因为当前 Coder 需要 Design，Tester 需要 implementation/changeset，Reviewer 需要 TestResult。
- 可增删三条前向咨询边 `Designer → Tester`、`Designer → Reviewer`、`Coder → Reviewer`。禁止反向边、重复边、自环、循环、任意节点类型或同一 Capability 的第二个副本。节点位置只用于可读性，不具有调度语义。
- 每条边表示下游在其主任务前评审当前上游交付。必需主干也提供执行依赖；可选边增加咨询，但不取消必需顺序。
- Seed 一个包含六条边的默认 Team，与之前 Capability 页展示的拓扑一致。用户可创建/重命名 Team、编辑可选边和节点位置，并为 Run 选择 Team。本版本不支持删除。

## Review 门禁执行

1. Designer 接收 goal，使用现有 persona 和输出契约产出 Design。
2. 每个上游节点完成后，Runtime **并发启动该节点所有出边邻居的咨询**。咨询使用下游 Capability 冻结的 Agent 绑定与 persona，但采用独立严格 Review 契约（`approved` 或 `changes_requested`、summary、非空 suggestions）。它只读，不执行下游主任务，也不写 worktree。
3. 一个生产 Attempt 的所有出边咨询均批准后，该 Handoff 才被接受。即使其他路径也能到达同一下游，可选跨边也不能跳过。
4. 若一个或多个 Reviewer 要求修改，聚合 suggestions、持久化 Review 记录与 Event，并把反馈交回上游 Capability。上游用先前 deliverable 与反馈在新 Attempt 中重跑。旧成功 Attempt 和 Artifact 保持可审计，新成功 Attempt 成为当前 Handoff 来源；其所有出边咨询再对新 Attempt 重跑。
5. 一个生产者在初始输出后最多三次 revision。若第四个输出仍被任何出边咨询拒绝，Run 以具体 `review_round_limit` 原因失败并保留全部建议。Provider/Contract 错误仍是普通执行失败，不是合成拒绝。
6. 批准持久化后，必需主干上的下一 Capability 才能执行。主节点因 Coder 与 Tester 共享可变 Git worktree 而保持串行；只有只读咨询并行 fan-out。
7. 最终 Reviewer 主结果保留现有 approved/changes-requested outcome。`changes_requested` 反馈给 Coder：Coder 与 Tester 重跑，然后 Reviewer 重跑，同样受三次 revision 上限约束。反馈持久化；Reviewer 批准后 Run 完成。

UI 区分下游**咨询**与该 Capability 的主执行；咨询不表示下游节点已经运行。禁止静默自动批准、Fake Agent 或预写 Tester 验收。

## 持久化与兼容性

SQLite 保存带稳定 ID、名称、revision、节点顺序/位置与边集合的 Team。保存时事务校验完整图并比较 revision；过期编辑器收到冲突而不是覆盖。默认 Team 幂等 seed。Run 请求携带 Team ID；旧 CLI 调用者省略时使用默认 Team。冻结 Run 定义保存 Team ID/name/revision 与完整校验拓扑，并与 Agent 绑定和 Capability 配置一同冻结。编辑 Team 或绑定不改变活跃/历史 Run。

Consultation 是独立持久记录，按 Run、源节点、目标节点、源 Attempt 和 Review round 关联，并记录 decision、suggestions、result Artifact/identity 与时间戳。它们产生有序 Event 供 Timeline 检查。Scheduler 从冻结拓扑及持久 Node/Consultation 状态推导进度；重启不会虚构批准或丢弃反馈。强制退出遵循现有 interrupted Run 策略。Schema-9 数据库迁移不改变旧 Run；没有 Team 的历史 Run 定义继续使用冻结的 legacy 四节点行为与 Snapshot 格式。

## 桌面体验

- 侧栏分别提供 Agent、Capability、Team。Capability 管理变为四个独立卡片/列表项，展示绑定 Agent 状态和一个绑定编辑器，不再显示图或边图例。
- Team 管理包含 Team 选择/创建控件和四个 Capability 节点的画布。拖动只改变位置；可通过可见控件切换三条可选前向边，三条必需边样式不同且锁定。非可视边列表提供相同键盘编辑操作。保存/dirty/冲突状态明确。
- New Run 要求选择 Team（默认预选）。Run detail 展示冻结 Team 及每次咨询的状态、Reviewer、suggestions 与 revision round。拓扑只读，来源是冻结 Run 定义，而非当前 Team。
- 纯浏览器 Vite preview 不得虚构持久化 Team、Agent health 或 Run 进度。真实管理与执行需要 Tauri Runtime。

## 失败与校验

- 创建 Run 前拒绝无效图、空白/重复 Team 名、过期 revision、禁用/不支持 Agent 绑定或缺失 Capability 配置。保存失败保留旧 Team。
- Consultation JSON 必须符合严格 Schema。`changes_requested` 至少包含一条可操作的非空建议；`approved` 不能携带阻塞建议。模型输出和反馈与生产 Attempt 保持 provenance 关联。
- 每次 revision 或最终 Review 拒绝均有上限。拒绝后，下游主任务不能使用被拒的生产 Attempt 启动。最终 Reviewer 反馈导致下游任务重跑时，旧结果只保留为历史，不能满足新一轮依赖。
- Cancellation 在现有 Run owner generation 下终止进行中的 consultation process 并记录取消；并发咨询在 owner 丢失后不能发布结果。

## 验证

- Migration 与 Team service 测试覆盖 seed、CRUD update、图校验、revision 冲突和保留 schema-9 数据。
- Scheduler 测试覆盖必需主干、可选 fan-out、全批准 gate、混合拒绝与反馈聚合、修改 Attempt 的重新 Review、三次上限、过期 Review 排除和 legacy Run。
- Runtime 测试通过 test-controlled invocation layer 使用真实 Codex Adapter 边界获得确定性结果；真实认证 Codex 验收仍单独手工执行，不引入 Fake Agent 产品 Adapter。
- 前端测试覆盖边切换、dirty/save/conflict、Team selection 序列化和 consultation Timeline 投影。交付前运行 Rust workspace 测试、前端测试/构建和 `git diff --check`。
