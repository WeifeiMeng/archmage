# C 批次：Capability Contract 起点

2026-09-17，`codex/s0-runtime-fencing`。本记录只覆盖 Task 08，不代表 C 批次或 S0 完成。

## 已实现

- Designer、Coder、Tester、Reviewer 四个 v1 Prompt 和严格 JSON Schema。
- Rust 端拒绝 malformed JSON、缺字段、额外字段、错误 enum 和错误 schema version。
- Designer design、criteria、steps 必须非空。
- Tester check ID、evidence ID、单项状态和总状态必须与 Runtime canonical evidence 完全一致。
- Reviewer 不能批准失败检查，issue path 不能为绝对路径或越出 Workspace。
- Coder 的 `blocked` 是合法 Contract 状态；实际 ChangeSet 不信任模型的 `changed_paths`。

## 验证

```bash
cargo test -p archmage-core --test contracts
```

这些是纯 Contract 测试，不调用模型。真实四角色调用、Context/Handoff、Check Runner 和调度器仍未实现。
