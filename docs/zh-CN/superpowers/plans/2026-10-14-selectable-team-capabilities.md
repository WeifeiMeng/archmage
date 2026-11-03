# 可选择的团队能力实施计划

简体中文 | [English](../../../en/superpowers/plans/2026-10-14-selectable-team-capabilities.md)

**状态：** 已实施。确定性测试和构建通过；法典的现场验收仍然是一项单独的手动操作。

> **对于代理工人：** 所需的子技能：使用超能力：执行计划来逐个任务地实施该计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 让用户选择团队成员和关系，`Coder → Reviewer` 和现有的四角色团队均可运行。

**架构：** 在内置角色的子集上验证连接的前向图。仅冻结选定的节点并推动该图表的准备情况、上下文、审核门和最终结果。更新团队编辑器以在边缘之前选择成员。

**技术堆栈：** Rust、rusqlite、Tauri、React、TypeScript、Vitest。

---

### 任务1：图形模型和编辑器

**文件：** `crates/archmage-core/src/team.rs`、`crates/archmage-core/tests/teams.rs`、`apps/archmage-desktop/src/teamGraph.ts`、`apps/archmage-desktop/src/teamGraph.test.ts`、`apps/archmage-desktop/src/TeamManagement.tsx`、`apps/archmage-desktop/src/teams.css`。

- [x] 为 `Coder → Reviewer` 有效、singleton/disconnected/backward 无效以及团队 CRUD 往返编写失败的 Rust 测试；运行 `cargo test -p archmage-core --test teams` 以查看预期的失败。
- [x] 验证子集、前向边缘、单项和可达性。保留内置的四角色图。
- [x] 为 adding/removing 成员编写失败的 Vitest 测试，并仅切换选定端点的边；运行 `npm test --prefix apps/archmage-desktop`。
- [x] 添加成员选择和动态关系控制；创建开始为空，保存显示验证错误。运行前端 tests/build。

### 任务 2：动态运行和调度

**文件：** `crates/archmage-core/src/runtime/mod.rs`、`crates/archmage-core/src/runtime/scheduler.rs`、`crates/archmage-core/src/runtime/dispatch.rs`、`crates/archmage-core/tests/runtime.rs`。

- [x] 编写失败的运行定义测试，断言选定的 `Coder → Reviewer` 团队恰好创建两个冻结节点并忽略未使用的代理绑定。
- [x] 仅创建选定的节点并从冻结图中派生 readiness/completion。保留传统的无团队运行和串行主执行。
- [x] 验证原始的四节点调度程序测试和新的子集测试。

### 任务 3：可选输入合同和反馈

**文件：** `crates/archmage-core/src/context.rs`、 `crates/archmage-core/src/runtime/dispatch.rs`、 `crates/archmage-core/src/contract.rs`、 `crates/archmage-core/src/store/consultations.rs`、 `crates/archmage-core/capabilities/v1/{coder,tester,reviewer}.md`、 `crates/archmage-core/tests/context.rs`、 `crates/archmage-core/tests/contracts.rs`、 `crates/archmage-core/tests/runtime.rs`。

- [x] 没有设计器的测试编码器上下文，没有测试器的审阅者 context/approval，以及仅重新打开选定的下游节点的最终审阅反馈。
- [x] 仅包括可用的上游切换，并带有 goal/repo 后备；保留严格的 JSON 输出模式，并且不要发明测试验收。
- [x] 将最终审核重新开放事务推广到选定节点后缀和绑定修订限制；重新运行重点测试。

### 任务 4：文件和验证

**文件：** `README.md`、`docs/guide.md`、`docs/runbooks/desktop-s0.md`。

- [x] 解释选择成员然后关系流程和两人最少人数。
- [x] 运行 `cargo fmt --all --check`、`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`npm test --prefix apps/archmage-desktop`、`npm run build --prefix apps/archmage-desktop`、`cargo check -p archmage-desktop` 和 `git diff --check`。
- [x] 检查最终差异的历史运行兼容性，并验证没有自动创建额外的团队。
