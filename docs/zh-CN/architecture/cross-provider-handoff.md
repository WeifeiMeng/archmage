简体中文 | [English](../../en/architecture/cross-provider-handoff.md)

# 跨 Provider 的 Capability 交付

本文说明一个 Capability 运行在 Codex、下一个 Capability 运行在 DeepSeek Harness（DSH）时的当前交付契约。未来新增 Provider 适配器也使用同一边界。

## 一句话结论

Capability A 与 Capability B 不会直接建立聊天。Archmage 中介每次交付：

```text
Capability A → Provider A 返回 JSON → Archmage 校验并提交 artifact
             → Archmage 排入 delivery → Context Resolver 构造快照
             → Provider B 执行 Capability B 并返回 JSON
```

两者共同的机器语言是**受 JSON Schema 约束的 UTF-8 JSON**。`summary_markdown` 等字段可以包含给人阅读的说明，但自然语言正文不是控制协议。各 Provider 的传输方式被封装在适配器内部。

## 三层协议

| 层级 | 所有者 | 契约 |
|---|---|---|
| 协作层 | Archmage Runtime | 交付物、路由、artifact、delivery 台账、Context Snapshot |
| Capability 层 | Archmage 校验器 | Designer、Coder、Tester、Reviewer 或咨询结果的版本化 JSON Schema |
| Provider 传输层 | Provider 适配器 | Codex CLI JSONL/结果文件，或 DSH bridge JSONL/session 帧 |

无论由哪个 Provider 执行，Capability 都消费相同的逻辑上下文，并满足相同的输出 Schema。

## 输出信封

消息流的每次成功执行都必须返回且只返回一个信封：

```json
{
  "deliverable": {
    "schema_version": "1",
    "summary_markdown": "已完成请求的修改。",
    "status": "implemented",
    "changed_paths": ["src/example.rs"],
    "notes": []
  },
  "routing": {"decision": "continue", "next": ["reviewer"]}
}
```

`deliverable` 必须匹配当前 Capability 的 Schema。`routing` 单独校验：

- `continue` 要求至少一个且不重复的直接下游；
- 只有配置的终止 Capability 可以返回 `complete`，此时 `next` 必须为空；
- Reviewer 的批准状态必须与 `complete` 一致，要求修改时必须沿已配置的边继续；
- 被阻塞的 Coder 或 Tester 不能继续路由，测试失败也不能完成流程；
- 非法 JSON、未声明字段、矛盾状态或非法接收方都会以 `contract_invalid` 失败。

可执行事实来源是 [`contract.rs`](../../../crates/archmage-core/src/contract.rs)。

## Archmage 提交和转发什么

校验通过后，Archmage 将完整信封发布为不可变的 `application/json` artifact，并在一个事务中提交 Attempt、路由、artifact 引用和下游 delivery。每个接收方得到独立 delivery，指向同一个已提交结果 artifact；多条 delivery 不会合并。

目标执行会收到有界 Context Snapshot，可包含：

- `goal` 与仓库视图；
- `incoming`：本次 delivery 携带的确切 artifact；
- `changeset`：来源 Coder 发布的规范 ChangeSet manifest；
- `previous_result`：同一 Run 中目标已有更早结果时提供；
- 冻结的 Agent 绑定、prompt 版本、Capability 指引、Skill/Tool 提示、下游、终止 Capability 和 delivery sequence。

大内容保留在 artifact store 中，通过 artifact ID 与 SHA-256 hash 引用。调用前会持久化渲染后的快照及其 hash。参见 [`context.rs`](../../../crates/archmage-core/src/context.rs)、[`runtime/flow.rs`](../../../crates/archmage-core/src/runtime/flow.rs)和 [`store/flow.rs`](../../../crates/archmage-core/src/store/flow.rs)。

## Codex 交付给 DSH 的示例

假设 `coder` 使用 Codex，`reviewer` 使用 DSH：

1. Archmage 向 Codex 发送渲染后的 Coder 上下文与消息流 Schema。
2. Codex 从 stdin 读取上下文，输出 CLI JSONL 生命周期事件，并写入受 `--output-schema` 约束的 JSON。
3. Archmage 校验结果，发布结果 artifact 与 Coder ChangeSet，再为 `reviewer` 排入 delivery。
4. Reviewer 快照包含确切的已提交 Coder 结果与 ChangeSet 引用，而不是对隐藏 Codex 历史的推测性摘要。
5. Archmage 通过 DSH bridge 发送渲染后的任务和逻辑上相同的 Schema。
6. DSH 通过换行分隔的 JSON 帧返回最终 assistant 文本。适配器解析一个 JSON 对象，Archmage 再执行标准 Capability 与路由校验。
7. Reviewer 路由修改请求回 `coder` 时，新 delivery 会把明确的 Reviewer artifact 作为下一次 Coder 调用的 `incoming`。

交换 Provider 绑定只改变适配器传输，不改变 artifact 或 delivery 契约。

## 对话语义

- 每次 Codex 执行都是独立调用；连续性来自 artifact 与 Context Snapshot，而不是共享对话历史。
- DSH 为每个 `Run + Capability` 保留一个存活的命名对话；重复调用同一 Capability 时复用其 sidecar 与 session。
- 即使绑定同一个 Agent 实例，不同 Capability 也绝不共享 DSH session。
- DSH sidecar 意外退出后不会被静默替换，因为新 session 会制造历史连续的假象。
- Archmage 重启后不会恢复存活 DSH session；受影响的旧进程收敛到 `interrupted`。
- 咨询使用临时隔离的 DSH session，不进入 Capability 主对话。

这些 Provider session 规则不能替代明确的交付 artifact。

## 失败归属

| 失败 | 结果 |
|---|---|
| 可执行文件、SDK、凭证、超时或进程失败 | Provider 适配器使 Attempt 失败并记录有界诊断 |
| 非 JSON 或不符合 Schema | 契约层以 `contract_invalid` 拒绝 |
| 接收方重复、未知或不是直接下游 | 路由校验拒绝 Attempt |
| artifact 或 ChangeSet 发布失败 | 不提交下游 delivery |
| DSH 意外退出 | sidecar 标记为不可继续，不伪造连续性 |
| 达到步数上限 | Runtime 停止；消息流 Run 当前最多完成 24 步 |

绝不能仅根据退出码或自然语言正文推断成功。只有经过校验的 artifact 与 delivery 台账条目提交后，交付才成立。

## 新增其他 Provider

新增适配器不定义新的 Agent 间语言。它必须接收 Context Snapshot 与 JSON Schema、执行读写权限、返回一个有界 JSON 值及非空执行身份、暴露生命周期和有界诊断，并把校验、artifact、路由与持久化留给 Archmage。

如果 Provider 不能可靠返回受 Schema 约束的 JSON，它就不兼容当前边界。

## 相关文档

- [上下文与契约](context-and-contracts.md)
- [协作模型](../concepts/collaboration-model.md)
- [消息流 Team 设计](../superpowers/specs/2026-10-14-message-flow-teams-design.md)
- [DeepSeek Harness Provider 设计](../superpowers/specs/2026-10-30-deepseek-harness-provider-design.md)

