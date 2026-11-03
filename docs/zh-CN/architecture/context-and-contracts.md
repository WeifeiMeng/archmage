简体中文 | [English](../../en/architecture/context-and-contracts.md)

# 上下文与契约

## 上下文绑定

Capability 声明自己需要什么，Workflow 节点声明每项输入来自哪里。

示例：

```text
goal        ← workflow.input.goal
design      ← nodes.design.artifacts.design_json
workspace   ← workflow.workspace.snapshot
test_result ← nodes.test.output
diff        ← workflow.workspace.diff_from_baseline
```

Context Resolver 会解引用这些绑定、校验类型、记录内容 hash，并在调用 Provider 前写入不可变的 Context Snapshot。

## Snapshot 内容

每个 Snapshot 记录：

- Run、节点与 Attempt 标识
- Capability ID 与版本
- Prompt 模板版本
- Provider 绑定与执行设置
- 已解析的标量输入
- Artifact ID、相对路径、媒体类型与内容 hash
- 仓库基线与工作区状态标识
- 创建时间戳与规范序列化版本

大型内容继续保存在 Artifact 中并通过 hash 引用。渲染后的 Provider prompt 及其 hash 也会保留，以便审计实际生效的输入。

## 输出契约

S0 为每个 Capability 使用一个 JSON Schema。每个 Schema 都带版本，并在可行时拒绝未声明字段。Codex Adapter 通过 `codex exec --output-schema` 传入 Schema，Runtime 在接受返回结果前还会再次校验 JSON。

结果包含用于人类阅读的 `summary_markdown` 字段。Runtime 会将其提取为 Markdown Artifact，同时保留已校验 JSON 作为机器契约。

## S0 Handoff

| 生产者 | 必需输出 | 消费者 |
|---|---|---|
| Designer | Design JSON、design Markdown | Coder、Tester、Reviewer |
| Coder | ImplementationResult、workspace diff | Tester、Reviewer |
| Tester | TestResult、命令证据 | Reviewer |
| Reviewer | ReviewResult | 最终 Run 报告 |

Reviewer 结果在 S0 中是终点。S1 会增加 `approved` 或 `changes_requested` 条件以及 Fixer 循环。

内置 Tester 接收测试任务，而不是预先声明的命令列表。其严格结果会记录采用的每条命令、退出码、简要结论与限制。`passed` 结果至少需要一条成功命令，且不能与命令结果矛盾；测试失败或被阻塞时 Reviewer 不能批准。旧的 Archmage CheckRunner 仍作为底层基础设施保留，但内置目标驱动 Workflow 不使用它。

## 契约失败

进程正常退出是必要条件，但并不足够。解析失败、Schema 不匹配、Artifact 缺失、状态矛盾或违反工作区不变量，会将 Attempt 标记为 `contract_invalid` 或 `workspace_invariant_failed`。Runtime 会保存原始最终消息与校验诊断，但不会根据自然语言推断成功。
