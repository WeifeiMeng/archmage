简体中文 | [English](../../../en/superpowers/plans/2026-09-29-capability-topology-and-management.md)

# 能力拓扑及管理实施方案


> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 将四个内置功能角色显示为真实的固定拓扑，并让用户将每个角色绑定到代理，而无需更改现有的运行。后续范围决策：提示和元数据编辑被推迟。

**架构：** Core拥有内置的拓扑和能力配置。SQLite 保留配置，运行时将其冻结到每次运行中，Tauri 公开键入的命令。React 绘制固定图表并一次编辑一项选定的功能。

**技术堆栈：** Rust、rusqlite、Tauri 2、React、TypeScript、CSS、Vitest。

---

### 任务 1：持久功能配置

**文件：** `crates/archmage-core/migrations/0009_capabilities.sql`、`crates/archmage-core/src/store/mod.rs`、`crates/archmage-core/src/capability.rs`、`crates/archmage-core/tests/agents.rs`。

- [ ] 添加失败的 Store 测试：打开 schema-8 数据库升级到 9，保留 `capability_bindings`，并产生四个默认配置；无效的密钥和过时的修订版本保存失败。
- [ ] 执行前运行 `cargo test -p archmage-core capability_config -- --nocapture` 并确认失败。
- [ ] 添加版本 9 迁移和 Store read/save 方法。在一个即时事务中保存元数据和绑定，比较提供的修订版，验证键和字段限制 (80/500/8000)，并一次性修改修订版。
- [ ] 公开 `capability.rs` 中不可变的 key/default 元数据和拓扑边缘。执行边沿为 `designer→coder→tester→reviewer`；其他上下文边缘为 `designer→tester`、`designer→reviewer`、`coder→reviewer`。
- [ ] 运行重点测试，然后提交该单元。

### 任务 2：冻结配置并呈现提示

**文件：** `crates/archmage-core/src/runtime/mod.rs`、`crates/archmage-core/src/runtime/dispatch.rs`、`crates/archmage-core/src/context.rs`、`crates/archmage-core/tests/runtime.rs`。

- [ ] 添加失败的测试：运行冻结所有四个配置记录；稍后编辑一个运行会使旧的运行保持不变；没有配置的旧定义反序列化；渲染的上下文包含冻结的补充指令。
- [ ] 运行 `cargo test -p archmage-core capability_config -- --nocapture` 并确认新测试失败。
- [ ] 使用传统运行的 serde 默认值，通过每个功能的配置映射来扩展冻结的运行定义。在创建时解析它，在 `RunSnapshot` 中返回它，并将冻结指令作为单独的清单字段传递给 `resolve_context`。保留固定的基本提示和架构。
- [ ] 运行重点测试并提交该单元。

### 任务3：Tauri/API边界

**文件：** `apps/archmage-desktop/src-tauri/src/lib.rs`、`apps/archmage-desktop/src/api.ts`、`apps/archmage-desktop/src/model.ts`。

- [ ] 定义类型化 `CapabilityConfiguration`、`CapabilityManagementSnapshot`，并将输入保存在 Rust/TypeScript 中，并使用匹配的驼峰式字段。
- [ ] 添加调用 Core 服务的 `get_capability_management` 和 `save_capability_configuration` 命令。后者返回刷新的快照，包括修订版和可用的代理实例。
- [ ] 保留旧的绑定命令只是为了向后兼容，同时将 UI 路由控件移动到新页面。
- [ ] 从 `apps/archmage-desktop` 运行 `cargo check -p archmage-desktop` 和 `npm run build`；提交本单元。

### 任务 4：固定图表和编辑器

**文件：** `apps/archmage-desktop/src/CapabilityManagement.tsx`、`apps/archmage-desktop/src/capabilities.css`、`apps/archmage-desktop/src/AgentManagement.tsx`、`apps/archmage-desktop/src/App.tsx`、`apps/archmage-desktop/src/model.test.ts`。

- [ ] 为拓扑元数据添加失败的前端测试：四个稳定节点、三个执行边缘、三个附加上下文边缘，并且无边缘编辑操作。
- [ ] 从 `apps/archmage-desktop` 运行 `npm test` 并确认新测试失败。
- [ ] 添加 `Capabilities` 侧边栏目标。绘制一个固定的从左到右 SVG/HTML 拓扑，其中包含实线执行箭头和虚线额外切换、图例、可访问的节点按钮以及窄宽度的水平溢出。
- [ ] 显示只读角色元数据和单个可编辑代理绑定。只节省明确的行动；保留失败时的选择并显示修订冲突。仅从运行时显示真实连接状态。
- [ ] 从 `AgentManagement` 中删除绑定表并链接到 `Capabilities`。Link Run 将警告绑定到新页面。
- [ ] 运行前端 tests/build 并提交该单元。

### 任务 5：集成验证和文档

**文件：** `README.md`、`docs/guide.md`，上面更改的文件。

- [ ] 记录功能设置的位置、实线箭头与虚线箭头的含义、仅未来运行的行为以及尚不支持团队画布编辑。
- [ ] 在桌面应用程序中运行 `cargo test --workspace`、`npm test` 和 `npm run build`；检查 `git diff --check`。
- [ ] 根据批准的规范检查最终差异并报告任何剩余的差距。除非有要求，否则无需合并或推送即可提交文档和最终修复。
