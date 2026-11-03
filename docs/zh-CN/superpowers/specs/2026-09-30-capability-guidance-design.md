简体中文 | [English](../../../en/superpowers/specs/2026-09-30-capability-guidance-design.md)

# Capability 指引设计

**状态：** 已批准在 `main` 实施。

## 边界

每个内置 Capability（Designer、Coder、Tester、Reviewer）继续使用固定的 Runtime persona 与 JSON 输出契约。可编辑的 `extra_instructions` 是角色特定补充 prompt。Capability 还拥有手工配置的 Skill 与 Tool 指引条目，每项包含名称和用途。这些条目是给绑定真实 Agent 的指令，不证明 Skill 或 Tool 已安装，也不是权限或执行 allowlist。

## 数据与执行

SQLite 在 `capability_configurations` 上保存 `skills_json` 与 `tools_json`，迁移时默认空数组。保存操作校验名称、用途、列表大小、重复名称及现有 revision。已有配置、数据库与历史 Run 定义会以空数组反序列化。新 Run 定义冻结完整 Capability 配置；后续编辑只影响新 Run。

主节点 Context 包含冻结的角色补充说明和 Skill/Tool 指引。只读咨询会把目标 Capability 的冻结补充说明和指引与内置 persona 一同使用。两种 prompt 都明确说明列表只是建议，不代表可用性，也不限制其他工具。Runtime 不尝试安装 Skill、启用 Tool 或更改 Codex CLI 权限。

## 桌面端

Capability 管理继续采用无拓扑的列表/详情页。所选 Capability 展示内置角色说明、可编辑补充 prompt、Skill 条目、Tool 条目和绑定 Agent。用户可手工增删改条目并原子保存全部变更。列表汇总已配置 Skill 与 Tool 数量。Dirty 导航、刷新确认、乐观 revision 冲突和“仅影响新 Run”提示覆盖扩展后的编辑器。纯浏览器预览仍不具权威性。

## 验证

测试覆盖迁移/默认值、校验与 revision 冲突、新旧 Run 的配置快照、主执行与咨询 prompt 内容，以及编辑器 draft 序列化/dirty 行为。运行聚焦 Rust 与前端测试、完整 Rust workspace 测试、前端构建和格式/diff 检查。
