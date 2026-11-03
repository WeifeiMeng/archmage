# 消息流团队实施计划

简体中文 | [English](../../../en/superpowers/plans/2026-10-14-message-flow-teams.md)

> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 让用户绘制定向传递路线，包括反馈循环，并运行真正的 Codex 代理，直到配置的功能发出终端信号。

**架构：** 版本团队图表位于现有的 JSON 列中，因此旧的 Teams/Runs 保留协商语义。新流程团队冻结 entry/terminal 选择。持久的 `flow_deliveries` 分类帐对每个传入消息进行一次执行排队；代理返回其正常角色输出以及单独的经过验证的路由信封。新的流调度程序一次运行一个传递，并在使用下一个传递之前自动记录路由。

**技术堆栈：** Rust、rusqlite、Tauri、React、TypeScript、Vitest。

**进展（2026-10-15）：** 实现了版本化图表、持久交付分类账、严格的路由契约、真正的 Codex 流程调度程序、画布编辑器、运行交付显示和指南。现在，审查修复会在尝试开始时绑定尝试、结算 failed/cancelled/interrupted 交付，并使重复边缘绘制幂等。全自动检查和真正的两步编码器 → 审阅者 → 在架构 v15 后通过完全验收。实时反馈周期验收和交互式桌面 pointer/keyboard QA 仍然是单独的后续检查；真正的接受并不能证明这些相互作用。

---

### 任务 1：版本化团队图

**文件：** `crates/archmage-core/src/team.rs`、`crates/archmage-core/tests/teams.rs`、`apps/archmage-desktop/src/model.ts`、`apps/archmage-desktop/src/teamGraph.ts`、`apps/archmage-desktop/src/teamGraph.test.ts`。

- [ ] 添加失败的 Rust 测试，将旧的团队图反序列化为旧版，并接受带有 `entry: "coder"`、`terminal: "reviewer"` 以及 `coder → reviewer` 和 `reviewer → coder` 的流程图。运行 `cargo test -p archmage-core --test teams`；预计流程案例会失败。
- [ ] 将 `mode`（默认旧版）、`entry` 和 `terminal` 添加到 `TeamGraph`。通过现有 forward/single-entry 规则验证旧图。通过选定的端点、无 self/duplicate 边缘、入口的可达性以及从每个节点到终端的路径来验证流图。保持团队存储 API 不变。
- [ ] 添加倒数边的失败 Vitest 测试和清除 entry/terminal 的已删除成员。更新图形助手和 TS 类型；运行 `npm test --prefix apps/archmage-desktop` 直到绿色。
- [ ] 运行 Rust 团队测试并提交此独立可运行的架构更改。

### 任务2：持久交付分类账

**文件：** `crates/archmage-core/migrations/0014_flow_deliveries.sql`、`crates/archmage-core/src/store/mod.rs`、`crates/archmage-core/src/store/flow.rs`、`crates/archmage-core/tests/flow.rs`、`crates/archmage-core/tests/persistence.rs`。

- [ ] 编写失败的存储测试：将条目交付排队，声明一次，提交 attempt/routing 决策，对两个目标交付进行排队，然后验证它们的不同 ID 和稳定顺序。包括重复交付相同的功能。
- [ ] 添加迁移 14 和 `Store::enqueue_flow_entry`、`claim_flow_delivery`、`complete_flow_delivery`、`flow_deliveries` 和 `skip_flow_pending`。保持源尝试、目标、有效负载工件、状态、序列和路由决策的持久性。所有完成写入和扇出插入都发生在具有所有者生成防护的一个 SQLite 事务中。
- [ ] 添加针对 invalid/stale 声明、取消运行、终端跳过和升级 schema-13 数据库的测试。运行 `cargo test -p archmage-core --test flow --test persistence` 并提交。

### 任务 3：路由合约和真实 Codex 输出

**文件：** `crates/archmage-core/src/contract.rs`、`crates/archmage-core/src/runtime/flow.rs`、`crates/archmage-core/src/runtime/dispatch.rs`、`crates/archmage-core/src/context.rs`、`crates/archmage-core/tests/contracts.rs`、`crates/archmage-core/tests/context.rs`。

- [ ] 为 `continue(next=["reviewer"])`、授权 `complete`、不相邻目标、接下来的 duplicate/empty 和审阅者状态不一致编写失败的合约测试。
- [ ] 定义携带 `deliverable`（由选定的内置角色模式验证）和 `routing` (`continue`/`complete`) 的流包络。为真正的 Codex 调用生成严格的 JSON 模式。根据冻结的传出边缘和终端密钥验证路由；永远不要从散文中推断路线。
- [ ] 为来自 Reviewer 的 Coder 交付编写失败的上下文测试：目标、传入的反馈工件、先前的 Coder 工件和存储库快照都无需设计师移交即可呈现。实现特定于流的上下文构建器，同时保持旧上下文哈希不变。
- [ ] 运行重点 contract/context 测试并提交。

### 任务 4：流调度程序和生命周期

**文件：** `crates/archmage-core/src/runtime/mod.rs`、`crates/archmage-core/src/runtime/flow.rs`、`crates/archmage-core/src/runtime/dispatch.rs`、`crates/archmage-core/src/store/flow.rs`、`crates/archmage-core/tests/runtime.rs`、`crates/archmage-core/tests/flow.rs`。

- [ ] 为编码员→审阅者→编码员→审阅者完成编写失败的确定性测试。每次交付测试一次执行，无隐式连接、终端跳过和步骤限制 24。
- [ ] 在冻结团队模式下分支 `run_to_terminal`。流模式播种一笔交付，重复声明一笔交付，调用选定的真实 Codex 绑定，验证并提交 output/routing，并在授权终端信号处停止。传统模式保留其当前的 scheduler/review 门。
- [ ] 在 `RunSnapshot` 中公开交付历史记录；保留取消、中断和业主围栏。运行重点运行时测试，然后 `cargo test --workspace`；犯罪。

### 任务 5：画布编辑和运行显示

**文件：** `apps/archmage-desktop/src/TeamManagement.tsx`、`apps/archmage-desktop/src/teamGraph.ts`、`apps/archmage-desktop/src/teamGraph.test.ts`、`apps/archmage-desktop/src/teams.css`、`apps/archmage-desktop/src/model.ts`，运行工作台组件。

- [ ] 添加失败的 Vitest 测试，用于从输出句柄开始连接、取消空 canvas/Escape、倒数边缘几何、selecting/deleting 线以及键盘源→目标创建。
- [ ] 新团队创建以流程模式开始。将关系复选框替换为 pointer/keyboard 控制柄和边选择。添加 entry/terminal 选择器并清除验证。标记旧 Teams 旧版，提供明确的复制到流程，无需自动 mutating/saving。
- [ ] 在工作台中显示交货和路线历史记录。运行 Vitest/build 并提交。

### 任务 6：文件记录和验收

**文件：** `README.md`、`docs/guide.md`、`docs/runbooks/desktop-s0.md`、`docs/README.md`。

- [ ] 记录用户如何创建流程团队、绘制 Coder ↔ Reviewer、选择 entry/terminal、选择路线以及解释 step-limit/terminal 事件。明确区分旧团队和新流程团队。
- [ ] 运行 `cargo fmt --all --check`、`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`npm test --prefix apps/archmage-desktop`、`npm run build --prefix apps/archmage-desktop`、`cargo check -p archmage-desktop` 和 `git diff --check`。单独记录实时 Codex 验收情况，而不是通过确定性测试声明它。
- [ ] 检查旧 Team/Run 兼容性的最终差异，并确认没有自动创建用户团队。提交文档和最终修复。
