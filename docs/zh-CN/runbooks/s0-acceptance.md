简体中文 | [English](../../en/runbooks/s0-acceptance.md)

# S0 真实 Codex 验收手册

## 目的

本流程是 S0 的发布门禁。必须使用已安装且已认证的 Codex CLI，不得以模拟 Provider 替代。

## 前置条件

- `archmage doctor` 通过。
- 存在至少包含一个 commit 的一次性本地 Git 仓库。
- 仓库具有确定性的校验命令。
- 请求的变更小、可观察且安全，例如添加带测试的 health endpoint。
- 源仓库不包含该变更需要的凭证或生产专用数据。

## 场景 A：完整协作流程

1. 记录源仓库的 branch、`HEAD`、worktree status 与已配置 remote。
2. 运行：

   ```bash
   archmage run \
     --repo <acceptance-repository> \
     --goal "<small acceptance goal>" \
     --check "<deterministic validation command>"
   ```

3. 确认 Designer、Coder、Tester 和 Reviewer 各有不同的 Codex thread ID。
4. 确认每个节点都有 Context Snapshot、渲染 prompt、JSONL Event 流、已校验 JSON 结果与 Markdown 摘要。
5. 确认 Coder 修改的是 Run worktree，且记录的 diff 相对于已持久化基线。
6. 确认 Tester 通过 Archmage 管理的 Check Runner 调用每个必需检查，且其 Attempt 范围证据与 TestResult 匹配。
7. 确认 Reviewer 通过 Context 绑定收到设计、diff 与 TestResult。
8. 确认最终报告链接四个节点结果与 worktree 位置，并将 Reviewer 的 `approved` 或 `changes_requested` 保留为完成结果。
9. 再次检查源 worktree 与 remote；二者必须保持不变。

## 场景 B：中断与恢复

1. 使用能让 Tester 保持活动足够久的目标启动相同 Workflow。
2. 在 Tester 运行时终止 Archmage Runtime 进程。
3. 重启 CLI 并检查 Run。
4. 确认旧 Tester Attempt 被对账为 `interrupted`，其不完整输出不能用于 Context。
5. 运行 `archmage resume <run-id>`。
6. 确认创建新的 Tester Attempt 与新的真实 Codex thread。
7. 确认 Workflow 完成，且不会重新运行已成功的上游节点。

## 场景 C：离线契约回归（不属于真实 Provider 验收）

1. 使用格式错误的最终 JSON fixture 运行契约单元测试；另行将 Schema 编译失败作为 preflight 失败测试。
2. 确认即使退出状态成功，结果接受函数仍会拒绝无效输出。
3. 使用真实临时数据库测试失败状态转换与依赖就绪规则。这些是组件测试，不是合成 Agent Workflow，不能满足场景 A 或 B。

[实施计划](../superpowers/plans/2026-09-16-s0-real-codex-vertical-slice.md)还要求在 S0 完成前提供真实 Provider 的取消、dirty source、失败检查与并发 reader 证据。

## 验收记录

发布记录包括：

- Archmage commit 与 build profile
- 操作系统、Git 版本与 Codex CLI 版本
- 所有场景的 Run ID
- CLI 退出码
- 自动化测试命令与结果
- 完整流程及恢复证据的 Artifact 路径
- 源仓库前后 Git 证据
- 所有偏差；在解决偏差或显式修改已批准 S0 规格前，门禁均视为失败
