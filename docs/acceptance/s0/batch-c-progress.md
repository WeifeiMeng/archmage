# C 批次：Capability Contract 起点

2026-09-17，`codex/s0-runtime-fencing`。本记录覆盖 Task 08–09，不代表 C 批次或 S0 完成。

## 已实现

- Designer、Coder、Tester、Reviewer 四个 v1 Prompt 和严格 JSON Schema。
- Rust 端拒绝 malformed JSON、缺字段、额外字段、错误 enum 和错误 schema version。
- Designer design、criteria、steps 必须非空。
- Tester check ID、evidence ID、单项状态和总状态必须与 Runtime canonical evidence 完全一致。
- Reviewer 不能批准失败检查，issue path 不能为绝对路径或越出 Workspace。
- Coder 的 `blocked` 是合法 Contract 状态；实际 ChangeSet 不信任模型的 `changed_paths`。
- ContextRef 只允许 WorkflowInput、CommittedNodeOutput、Artifact、WorkspaceSnapshot。
- 固定角色 bindings，递归 canonical JSON；Prompt 版本与实际发送 bytes 都进入 snapshot hash。
- schema v4 持久化 Context 和 Handoff；consumer attempt 绑定具体 context_id。
- Artifact 在解析时重新验证成功 attempt、长度和 SHA-256；篡改或未提交输入不可消费。

## 验证

```bash
cargo test -p archmage-core --test contracts
cargo test -p archmage-core --test context --test artifacts
```

这些测试不调用模型。Context Snapshot 只表示 Archmage 显式提供的输入与 Prompt，不宣称模型输出可确定重放，也不包含 Codex 隐式系统 Prompt。真实四角色调用、Check Runner 和调度器仍未实现。
