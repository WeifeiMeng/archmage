# 能力指导实施计划

简体中文 | [English](../../../en/superpowers/plans/2026-09-30-capability-guidance.md)

> **对于代理工人：** 所需的子技能：使用超能力：执行计划来逐个任务地实施该计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 使每个能力的角色补充、技能和工具可编辑，并将冻结的指导传递给其真正的代理。

**架构：** 将两个 JSON 列表列添加到现有的修订保护功能行。扩展冻结配置和两个提示构建器；保持 Codex 权限不变。展开现有的 list/detail 编辑器，而不是添加拓扑或单独的目录。

**技术堆栈：** Rust、rusqlite、serde、Tauri、React、TypeScript、Vitest。

---

### 任务 1：坚持经过验证的指导

**文件：** `crates/archmage-core/src/capability.rs`、`crates/archmage-core/src/store/capabilities.rs`、`crates/archmage-core/src/store/mod.rs`、`crates/archmage-core/migrations/0013_capability_guidance.sql`、`crates/archmage-core/tests/agents.rs`。

- [x] 添加一个失败的测试，用一项技能 (`brainstorming`) 和一项工具 (`web_search`) 保存设计人员，重新加载两者，并拒绝重复的名称和过时的修订。运行 `cargo test -p archmage-core --test agents capability_config -- --nocapture` 并确认预期的失败。
- [x] 将 `CapabilityGuidance { name: String, purpose: String }` 和 `skills`/`tools` 数组（serde 默认值）添加到配置和输入。验证非空名称和用途、每个列表最多 16 个条目、名称长度 80、用途长度 500 以及每个列表中不区分大小写的唯一名称。
- [x] 使用 `ALTER TABLE capability_configurations ADD COLUMN skills_json TEXT NOT NULL DEFAULT '[]'` 和等效的 `tools_json` 进行迁移；在 `Store::open` 中注册版本 13。Decode/encode 现有受保护保存事务内的 JSON 数组。
- [x] 重新运行重点测试并检查其是否通过。使用空数组和迁移版本断言将测试中的现有输入文字更新为 13。

### 任务 2：冻结并提供指导

**文件：** `crates/archmage-core/src/context.rs`、`crates/archmage-core/src/runtime/dispatch.rs`、`crates/archmage-core/src/runtime/consultation.rs`、`crates/archmage-core/tests/context.rs`、`crates/archmage-core/tests/consultation_prompt.rs`、`crates/archmage-core/tests/runtime.rs`。

- [x] 添加失败提示测试：主要上下文包括 Skill/Tool 名称、用途和咨询语义；协商包括目标能力的相同冻结指导。添加运行快照测试，在创建运行后更改功能不会更改该运行的数组。
- [x] 将配置传递到主上下文组装和咨询渲染中。保持内置角色和输出模式完整。将指南包含为结构化字段，并带有明确的 advisory/non-enforcement 注释。
- [x] 重新运行重点上下文、咨询和运行时测试；仅解决新的回归。

### 任务 3：编辑能力管理中的指南

**文件：** `apps/archmage-desktop/src/model.ts`、`apps/archmage-desktop/src/model.test.ts`、`apps/archmage-desktop/src/CapabilityManagement.tsx`、`apps/archmage-desktop/src/capabilities.css`、`docs/guide.md`。

- [x] 添加失败的 Vitest 覆盖范围，以序列化已编辑的草稿以及角色文本、技能列表、工具列表或绑定更改时的脏检测。运行 `npm test --prefix apps/archmage-desktop` 并确认预期的失败。
- [x] 将仅绑定草稿助手替换为完整配置草稿。为技能和工具添加可编辑角色文本区域和 name/purpose 行。通过现有的 Tauri 命令保存所有字段并显示 validation/conflict 错误。在列表中显示计数并在详细视图中显示咨询免责声明。
- [x] 在 `docs/guide.md` 中记录工作流程和仅供咨询的语义；运行前端测试并构建。

### 任务4：最终验证

- [x] 从存储库根运行 `cargo fmt --check`、`cargo test --workspace`、`npm test --prefix apps/archmage-desktop`、`npm run build --prefix apps/archmage-desktop` 和 `git diff --check`。
- [x] 根据设计检查差异，并报告任何未经验证的实时 Codex 行为，以区别于确定性测试。
