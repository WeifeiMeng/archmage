简体中文 | [English](../../en/product/vision.md)

# 产品愿景

## 目的

Archmage 是面向软件开发的本地优先协作 Runtime。它连接用户机器上已有的 Agent Provider，围绕共享代码仓库组织以 Capability 为中心的 Agent，让用户不再需要在不同工具之间复制输出。

产品的第一个验证目标是：通过一条命令，把一个目标转化为可追踪的设计、实现、验证与评审序列，并全部运行在隔离的 Git worktree 中。

## 产品模型

```text
Provider          执行 Agent 会话，例如 Codex CLI
Capability Agent  使用自己的指令与契约承担一个专业角色
Workflow          协调 Capability Agent 及其依赖关系
Handoff           在 Agent 之间传递显式 Artifact 与上下文
Workspace         为会修改代码的工作提供共享的 Git 事实
```

多个协作 Agent 并不要求使用多个供应商产品，也不要求保持多个长生命周期进程。即使四个会话都由同一个本地 Codex Provider 执行，Designer、Coder、Tester 与 Reviewer 仍然是彼此独立的逻辑 Agent。

## 原则

1. **本地优先。** 源码访问、Agent 执行、Git 操作、日志、Artifact 与状态默认都保留在本地。
2. **Capability 优先。** Workflow 依赖 Capability，而不是供应商特定的 Agent 名称。
3. **显式上下文。** 每个节点接收可复现的 Context Snapshot，而不是继承不透明的对话历史。
4. **契约驱动交付。** 人类可读 Artifact 与机器校验结果一同传递。
5. **工作区是共享事实。** 会修改代码的 Agent 通过真实仓库、基线 commit、branch、worktree 与 diff 协作。
6. **决策可观测。** Runtime 记录谁执行了任务、看到了什么、产出了什么，以及 Workflow 为什么继续或停止。

## MVP 的产品非目标

- 构建新的基础模型或托管 Agent 服务
- 云端 SaaS、组织或多用户协作
- 自动选择 Provider 或构建 Agent 市场
- 语义记忆、RAG 或隐式的跨 Run 记忆
- 自由形式的 DAG 画布
- 自动 merge 或 push 到用户分支
