# 跨提供商切换指南设计

简体中文 | [English](../../../en/superpowers/specs/2026-11-03-cross-provider-handoff-guide-design.md)

**状态：** 已批准用于文档实施。

## 目的

添加维护的、有代码支持的英语和简体中文指南，解释当两种功能使用不同的代理提供商（尤其是 Codex 和 DeepSeek Harness (DSH)）时，Archmage 的双手如何从一种功能工作到另一种功能。两个指南都必须回答三个问题，而不要求读者检查运行时：

1. 能力A实际上提供了什么？
2. 能力B实际接收到什么上下文？
3. 协议的哪些部分与提供商无关，哪些部分仅属于 Codex 或 DSH 适配器？

这些指南记录了当前的行为。他们没有提出新的运行时协议、更改提供程序执行或引入 MCP、ACP、共享提供程序历史记录或任意功能模式。

## 位置和文档更新

创建两个一流的技术指南：

- `docs/architecture/cross-provider-handoff.md` — 英语；
- `docs/architecture/cross-provider-handoff.zh-CN.md` — 简体中文。

架构是合适的层，因为文档定义了当前的边界和不变量，而不是操作过程或未来的功能。每个指南都将以互惠语言链接开头。两个版本将使用相同的标题结构、JSON示例、源链接和规范含义；翻译不得引入不同的协议要求。

链接指南：

- `docs/README.md`，按照推荐的阅读顺序，在同一个条目上有两种语言的链接；
- `docs/superpowers/specs/2026-10-14-message-flow-teams-design.md`，除了持久的流程描述之外；
- `docs/architecture/context-and-contracts.md`，作为详细的跨提供商解释。

还要进行两个小的一致性修正：

- 将 Message-Flow Teams 设计状态从“实施中”更改为已实施状态，引导读者查看实施计划以验证历史记录；
- 将桌面指南中每次交付都执行“真实 Codex”的声明替换为描述与目标功能绑定的代理的提供商中立措辞，并将该中文指南链接到简体中文协议文档。

## 导轨结构

两种语言版本都将包含以下部分：

1. **协议边界**——Archmage，而不是任何一个提供者，是协调者。能力代理不建立直接的 Codex 到 DSH 会话。
2. **通用语言** — 受版本化 JSON Schema 约束的 UTF-8 JSON 是机器合约。仅在声明的人类可读字段内允许使用 Markdown 和自然语言。
3. **交付信封** — 文档 `deliverable` 和 `routing`，包括 `continue`、`complete` 和 `next` 约束。
4. **持久交付记录** — 解释 FlowDelivery 身份、源尝试、目标能力、序列、有效负载工件、状态、执行尝试和路由决策。
5. **接收器上下文** — 显示 `goal`、`repo_view`、`incoming`、可选 `changeset` 和可选 `previous_result`。解释包含 `artifact_id`、`sha256` 和 `content` 的 Artifact 包装器。
6. **端到端示例** — 使用代表性 JSON 跟踪 Coder/Codex → Reviewer/DSH 和 Reviewer/DSH → Coder/Codex 反馈周期。
7. **提供商适配器** — 将通用业务合约与 Codex CLI stdin/schema/result 处理和 DSH 换行符分隔的 JSON sidecar 协议分开。
8. **对话和工作区边界** - 声明下游功能不接收上游隐藏的对话历史记录。解释独立的 Codex 执行、per-Run/per-Capability 持久 DSH 对话、共享的隔离 Git worktree和规范的 Coder ChangeSet。
9. **验证和失败语义** — 涵盖格式错误的 JSON、架构违规、非法收件人、重复或空路由、未经授权的完成、role/routing 矛盾、阻塞输出和流程步骤限制。
10. **兼容性和非目标** — 将当前消息流团队与遗留咨询团队区分开来，并声明该协议不是 MCP、ACP 或 OpenAI SDK 切换原语。
11. **事实来源** — 链接到角色架构、合同验证、上下文渲染、流程运行时、交付存储、Codex 适配器和 DSH 适配器。

## 规范示例

这些指南将使用编码器切换作为主要示例，因为它同时使用结构化结果工件和单独的规范变更集：

```json
{
  "deliverable": {
    "schema_version": "1",
    "summary_markdown": "Implemented the requested change.",
    "status": "implemented",
    "changed_paths": ["src/example.rs"],
    "notes": []
  },
  "routing": {
    "decision": "continue",
    "next": ["reviewer"]
  }
}
```

接收器示例将显示完整的经过验证的信封被放置在 `incoming` 工件内容中，而不是减少到散文或 `summary_markdown` 的切换。它将单独显示 Coder ChangeSet 并清楚地标记 ID 和哈希值作为说明。JSON 字段名称和枚举标记在两种语言版本中保持相同；解释性散文和人类可读的示例值可以被翻译。

## 准确度规则

- 描述当前代码的行为，而不是提供商营销或隐藏的实施假设。
- 仅对架构或运行时验证强制执行的约束使用“must”。
- 将能力到能力的契约与适配器传输细节区分开来。
- 不要暗示 `changed_paths` 是规范的；运行时创建的ChangeSet是权威的。
- 不要暗示持续的 DSH 对话在各个功能之间共享或在Archmage重新启动后仍然存在。
- 请勿暗示任一提供商可以在冻结的团队图之外进行路由。
- 不要公开凭证、端点、本地数据值或真实的工件标识符。
- 保持英语和简体中文声明的语义一致。当实施细节发生变化时，在同一变更中更新两个文档。

## 验证

由于这只是文档工作，验证将包括：

- 在最终措辞之前检查每个引用的实现符号和行；
- 扫描新的和更新的文档以查找陈旧的声明、占位符和矛盾；
- 比较英文和简体中文的标题、示例、约束条件和源链接，以实现语义对等；
- 验证所有相关 Markdown 链接是否解析；
- 运行 `git diff --check`；
- 检查最终差异以确认运行时或架构文件没有更改。
