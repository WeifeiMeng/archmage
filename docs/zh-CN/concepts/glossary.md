简体中文 | [English](../../en/concepts/glossary.md)

# 术语表

## Provider

能够启动并控制 Agent 会话的物理执行后端。Codex CLI 是 S0 唯一的 Provider。

## Capability

带版本的专业工作定义。它声明指令、所需上下文、输出契约、权限与默认执行配置。

## Capability Agent

将 Capability 绑定到 Provider 后形成的逻辑协作角色。即使共享同一个 Codex CLI 安装，Designer、Coder、Tester 与 Reviewer 仍是独立的 Capability Agent。

## Workflow Definition

由节点、依赖、上下文绑定、条件、重试策略和循环上限构成的带版本图。S0 内置一个固定定义；S1 会将定义外部化。

## Workflow Run

Workflow Definition、用户输入、基线 commit、配置与工作区引用的一次执行快照。

## Node Run

一个 Workflow 节点的 Runtime 状态。Node Run 可以在失败和恢复过程中拥有多个 Attempt。

## Agent Run

Node Run 的某个 Attempt 所对应的一次具体 Provider 调用。在 S0 中，它是一个 `codex exec` 进程及其 JSONL 事件流。

## Context Reference

声明节点输入来源的类型化定义，例如 Workflow 输入、先前节点的 Artifact 或工作区快照。

## Context Snapshot

实际提供给某个 Node Run Attempt 的不可变、内容寻址输入视图。它记录值、来源引用、hash 与渲染元数据。

## Artifact

持久输出，例如 Markdown、已校验 JSON、Git diff、命令证据或事件日志。Artifact 内容保存在文件系统中，可搜索元数据保存在 SQLite 中。

## Handoff

将选定 Artifact 与 Context Reference 从一个 Capability Agent 显式传递给另一个 Capability Agent。Handoff 不是共享聊天历史。

## Workspace

分配给一个 Workflow Run 的隔离 Git worktree 与 branch。

## Event

Runtime 发出的有序、不可变事实，例如 `node.started`、`artifact.created` 或 `node.failed`。

## Provider session

Provider 特定的对话或执行标识。S0 为每个节点启动一个新的 Codex 会话，其 thread ID 只用于可观测性记录。
