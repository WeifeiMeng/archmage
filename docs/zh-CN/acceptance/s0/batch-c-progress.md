简体中文 | [English](../../../en/acceptance/s0/batch-c-progress.md)

# C 批次：Capability Contract 与 Check Runner

2026-09-17，`codex/s0-runtime-fencing`。本记录覆盖 Task 08–10，不代表 C 批次或 S0 完成。

## 已实现

- Designer、Coder、Tester、Reviewer 四个 v1 Prompt 和严格 JSON Schema。
- Rust 端拒绝 malformed JSON、缺字段、额外字段、错误 enum 和错误 schema version。
- Designer design、criteria、steps 必须非空。
- Tester check ID、evidence ID、单项状态和总状态必须与 Runtime canonical evidence 完全一致。
- Reviewer 不能批准失败检查，issue path 不能为绝对路径或越出 Workspace。
- Coder 的 `blocked` 是合法 Contract 状态；实际 ChangeSet 不信任模型的 `changed_paths`。
- ContextRef 只允许 WorkflowInput、CommittedNodeOutput、Artifact、WorkspaceSnapshot。
- 固定角色 bindings，递归 canonical JSON；Prompt 版本与实际发送 bytes 都进入 snapshot hash。
- schema v5 持久化 Context、Handoff 和 attempt-scoped check evidence；consumer attempt 绑定具体 context_id。
- Artifact 在解析时重新验证成功 attempt、长度和 SHA-256；篡改或未提交输入不可消费。
- 隐藏的 `archmage internal check` 只发送 endpoint、attempt ID 和冻结 check ID，不接受任意命令或数据目录。
- Runtime 校验活跃 attempt 与 owner generation，以 `/bin/sh -c` 在真实 worktree 中执行用户授权 check；stdin 关闭、环境变量使用 allowlist，Provider 认证变量不下传。
- Check 由 Codex permission profile 限制为 worktree 可写、其他路径只读、Codex 凭据目录拒绝访问；测试验证 sibling source 写入被拒绝。
- canonical evidence 包含 command/cwd/timeout、exit/signal、时间与时长、stdout/stderr hash 和 Artifact、执行前后 Workspace Snapshot。非零退出和 timeout 是证据，不伪装成传输失败。
- unknown/expired/other-attempt 请求被拒绝；重复完成请求返回同一 evidence，不重复执行。
- Capability Schema 的 `const`/`enum` 节点显式声明类型，满足真实 Codex structured output 校验。

## 验证

```bash
cargo test -p archmage-core --test contracts
cargo test -p archmage-core --test context --test artifacts
cargo test -p archmage-core --test checks
cargo build -p archmage-cli
cargo test -p archmage-core --test checks_live \
  real_tester_requests_success_and_failure_evidence \
  -- --ignored --nocapture
```

前三组测试不调用模型；最后一组调用真实 Codex Tester。2026-09-17 的验收中，真实 Tester 经精确 Unix socket allowlist 请求一个 exit 0 与一个 exit 7 check，并产生 `status=failed` 的合法 TestResult。Context Snapshot 只表示 Archmage 显式提供的输入与 Prompt，不宣称模型输出可确定重放，也不包含 Codex 隐式系统 Prompt。完整四角色主链与 Ready Set 调度器仍未实现。
