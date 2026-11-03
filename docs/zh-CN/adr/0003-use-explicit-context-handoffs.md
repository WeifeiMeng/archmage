简体中文 | [English](../../en/adr/0003-use-explicit-context-handoffs.md)

# ADR 0003：使用显式 Context Handoff

**状态：** 已接受

**日期：** 2026-09-16

## 背景

共享 Provider 对话历史会使下游 Agent 所见内容难以解释、Attempt 难以复现，也妨碍把 Capability 迁移到其他 Provider。

## 决策

每个 Node Run 接收由类型化引用和已提交 Artifact 构建的不可变 Context Snapshot。每个 S0 节点启动新的 Codex 会话。Provider 会话历史不是 Handoff 通道。

## 后果

- Context 可审计、可 hash、可重放。
- Handoff 保持 Provider 中立。
- 输入可能重复传输并消耗更多 token。
- Prompt 渲染与 Artifact 设计成为 Runtime 的一等职责。
