# 审查门控团队管理实施计划

简体中文 | [English](../../../en/superpowers/plans/2026-09-30-team-management.md)

> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 让运行选择一个已保存的团队，其固定的四能力图会导致真正的并发下游协商和有限的反馈修订。

**架构：** SQLite 和 `archmage-core` 拥有团队定义和审核记录。每次运行都会冻结经过验证的团队和 Capability/Agent 设置。运行时保留所需的四角色执行主干，在已保存的传出边缘上扇出只读协商，并使用持久建议重新运行被拒绝的生产者。Tauri 公开类型化的 Team 和 Run 数据；React 只拥有编辑和可视化功能。

**技术堆栈：** Rust、rusqlite、Tokio、真正的 Codex CLI 适配器、Tauri 2、React、TypeScript、Vitest。

---

### 任务 1：团队模型和图验证

**文件：** 创建 `crates/archmage-core/src/team.rs`；修改 `crates/archmage-core/src/lib.rs`；测试 `crates/archmage-core/tests/teams.rs`。

- [ ] 编写接受所需脊柱和 `(designer,tester)`、`(designer,reviewer)`、`(coder,reviewer)` 的任何子集的测试，但拒绝缺少所需边缘、duplicate/reverse 边缘、非内置键、重复节点或有界画布坐标之外的位置。
- [ ] 运行 `cargo test -p archmage-core --test teams graph_ -- --nocapture`；确认测试失败，因为 `TeamGraph` 不存在。
- [ ] 使用明确要求和允许的边集定义 `TeamGraph { nodes: Vec<TeamNode>, edges: Vec<TeamEdge> }`、`TeamNode { capability: String, x: i32, y: i32 }`、`TeamEdge { from: String, to: String }` 和 `validate()`。定义默认的六边图。
- [ ] 运行重点测试并提交 `feat: validate first-release team graphs`。

### 任务 2：保留团队定义并公开管理

**文件：** 创建 `crates/archmage-core/migrations/0010_teams.sql`、`crates/archmage-core/src/store/teams.rs`；修改 `crates/archmage-core/src/store/mod.rs`、`crates/archmage-core/src/team.rs`、`apps/archmage-desktop/src-tauri/src/lib.rs`、`apps/archmage-desktop/src/model.ts`、`apps/archmage-desktop/src/api.ts`；测试 `crates/archmage-core/tests/teams.rs`。

- [ ] 添加迁移测试，升级 schema-9 数据库并保留 bindings/Run 行；断言一个幂等种子默认团队。
- [ ] 添加服务测试：创建一个命名团队，使用匹配的修订版更新 name/graph，拒绝 blank/duplicate 名称和过时的修订版，并在验证失败时保留前一行不变。运行红色测试。
- [ ] 以事务方式保留 `teams(id,name,revision,graph_json,created_at,updated_at)`。使用现有的存储迁移路径和 `TeamService` 包装器。没有删除命令。
- [ ] 添加键入的 `list_teams`、`create_team`、`save_team` Tauri 命令和匹配的前端 API/model 类型。运行 Rust 测试和前端构建；提交 `feat: persist and manage teams`。

### 任务 3：将选定的团队冻结为运行状态

**文件：**修改`crates/archmage-core/src/runtime/mod.rs`、`apps/archmage-desktop/src/model.ts`、`apps/archmage-desktop/src/api.ts`、`apps/archmage-desktop/src/App.tsx`；测试 `crates/archmage-core/tests/runtime.rs`、`apps/archmage-desktop/src/model.test.ts`。

- [ ] 测试 `RunRequest { team_id: Some(id), ... }` 冻结团队 ID/name/revision/graph；稍后的团队编辑不会更改该运行。测试省略的团队 ID 选择 CLI 兼容性的默认值和没有团队静态快照的旧定义。
- [ ] 运行红色的重点 Rust 测试，然后在 definition/snapshot 中添加具有 serde 默认值的 `team_id: Option<String>` 和 `FrozenTeam`。在创建运行或配置worktree之前解析并验证团队。
- [ ] 添加期望 `teamId` 的前端序列化测试，然后更新请求帮助程序。运行 tests/build；提交 `feat: freeze selected team into runs`。

### 任务 4：保留协商决定和修订过渡

**文件：** 创建 `crates/archmage-core/migrations/0011_consultations.sql`、`crates/archmage-core/src/store/consultations.rs`；修改 `crates/archmage-core/src/store/transitions.rs`、`crates/archmage-core/src/store/mod.rs`；测试 `crates/archmage-core/tests/consultations.rs`。

- [ ] 测试审查记录主要针对源尝试和边缘；approval/rejection 活动包括评审和轮次；duplicate/stale 批准无法满足新生产者的尝试。
- [ ] 测试成功的上游节点的原子重新打开以进行另一次尝试和有限的反馈存储；如果下游主节点已经启动，则拒绝重新打开。运行红色测试。
- [ ] 通过 revision/owner-generation 检查和单独的审核记录、建议 JSON、thread/result 工件身份和状态添加严格的存储转换。不要覆盖旧的 Attempts/artifacts。运行测试；提交 `feat: persist review consultations and revisions`。

### 任务 5：真正的法典咨询和门控调度程序

**文件：** 创建 `crates/archmage-core/capabilities/v1/consultation.schema.json`、`crates/archmage-core/src/runtime/consultation.rs`；修改 `crates/archmage-core/src/runtime/dispatch.rs`、`crates/archmage-core/src/runtime/scheduler.rs`、`crates/archmage-core/src/runtime/mod.rs`、`crates/archmage-core/src/contract.rs`、`crates/archmage-core/src/context.rs`；测试 `crates/archmage-core/tests/runtime.rs` 和新的重点调度程序测试。

- [ ] 测试具有三个传出边缘的生产者需要三个批准，混合决策汇总建议并仅重新运行该生产者，第四个被拒绝的输出失败，并且旧的批准无法解锁新的尝试。包括最终审阅者 `changes_requested` 通过编码器和测试器重新路由。跑红。
- [ ] 使用只读沙箱和咨询模式调用真正的 Codex 适配器。根据目标能力角色、冻结绑定、目标、当前生产者工件和源尝试 ID 构建提示。验证拒绝的非空白建议；保留精确的工件和事件。
- [ ] 与单独的商店处理同时进行传出咨询；在调度下游主节点之前连接所有结果。将持续的反馈反馈到生产者的下一个上下文中，并使调度程序读取冻结的团队图。保持主要worktree写入执行的序列化。运行集中测试；提交 `feat: gate team runs on real consultations`。

### 任务 6：桌面团队画布和独立的功能页面

**文件：** 创建 `apps/archmage-desktop/src/TeamManagement.tsx`、`apps/archmage-desktop/src/teams.css`；修改 `apps/archmage-desktop/src/CapabilityManagement.tsx`、`apps/archmage-desktop/src/capabilities.css`、`apps/archmage-desktop/src/App.tsx`、`apps/archmage-desktop/src/model.ts`、`apps/archmage-desktop/src/model.test.ts`。

- [ ] 测试三个强制边仍然为 selected/locked，可选边切换而不生成无效图形，并且导航会警告未保存的团队编辑。运行红色前端测试。
- [ ] 用四项 list/detail 编辑器替换功能拓扑；保留代理绑定和只读角色摘要。
- [ ] 添加 Team selection/create、可拖动的四节点画布、键盘可操作的可选边缘列表、可见的必需边缘锁定、Save/dirty/conflict 反馈和真正的 Tauri-only 状态。将团队选择添加到“新运行”，并将冻结 Team/review 事件显示添加到“运行”详细信息。
- [ ] 运行 `npm test` 和 `npm run build`；提交 `feat: add team canvas and review timeline`。

### 任务 7：记录和验证

**文件：**修改`README.md`、`docs/guide.md`、`docs/runbooks/desktop-s0.md`；查看所有更改的文件。

- [ ] 文档代理 vs 能力 vs 团队所有权、图表约束、团队选择、consultation/feedback 轮次以及未来的技能范围边界。
- [ ] 运行 `cargo test --workspace --quiet`、`npm test`、`npm run build` 和 `git diff --check`。单独报告已验证的法典现场验收是否已运行；切勿将确定性适配器测试与真实帐户探测等同起来。
- [ ] 检查与 `docs/superpowers/specs/2026-09-30-team-management-design.md` 的最终差异，修复任何差距，并提交最终 docs/fixes。未经用户请求，请勿合并或推送。
