简体中文 | [English](../../en/concepts/collaboration-model.md)

# 协作模型

## Provider 实例之上的逻辑 Agent

Archmage 通过彼此独立的 Capability 建模协作，而不要求使用多个供应商产品或持久 Agent 进程。

```text
Codex CLI Provider
  ├── Designer Capability Agent
  ├── Coder Capability Agent
  ├── Tester Capability Agent
  └── Reviewer Capability Agent
```

每个 Capability Agent 都有自己的指令、权限、输入契约、输出契约、Context Snapshot 与 Provider 会话。共享同一个 Provider 不会合并这些边界。

## 通信

Capability Agent 通过 Handoff 通信：

```text
workflow input
  → Designer → Design artifacts
  → Coder    → Workspace diff and implementation result
  → Tester   → Test evidence and TestResult
  → Reviewer → ReviewResult
```

Runtime 会解析 Context Reference、冻结 Context Snapshot、渲染 Capability prompt、调用 Provider、校验结果并发布 Artifact。下游节点不会读取上游 Provider 的隐藏对话历史。

## 为什么会话彼此独立

- UI 可以准确展示 Agent 实际看到的内容。
- 失败的 Attempt 可以使用相同输入重放。
- Provider 特定历史不会变成未记录的依赖。
- Capability 之后可以切换到其他 Provider，而无需改变消费者。

代价是需要重复传输上下文。相较于不透明的会话连续性，Archmage 更重视可复现性与 Provider 独立性。

## 共享工作区规则

节点声明 `none`、`read` 或 `write` 工作区访问权限。当依赖和访问约束兼容时，Scheduler 可以并发执行多个已就绪节点。写节点独占该 Run 的 worktree lease；`none` 节点可以与其同时执行，多个读节点也可以基于冻结的 Context Snapshot 并行执行。
