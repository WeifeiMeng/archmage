简体中文 | [English](../../en/product/mvp-definition.md)

# MVP 定义

## 目标成果

MVP 用于证明 Archmage 能够连接本地 Agent Provider，并通过显式、结构化的交付，围绕同一个代码仓库协调多个 Capability Agent。

## MVP 完成状态

给定一个本地 Git 仓库和开发目标，Archmage 能够创建隔离工作区，并协调 Designer、Coder、Tester、Reviewer，以及 S1 之后的 Fixer Capability Agent。系统会保留它们的输入、上下文、输出、事件和工作区变更，并在不需要人工复制粘贴的情况下完成协作闭环。

## 交付阶段

MVP 采用增量方式交付：

- **S0 验证协作主干：** 一个真实 Codex Provider、四个 Capability Agent、固定线性工作流、真实 Git 隔离、契约、持久化与恢复。
- **S1 闭合循环：** 外部化定义、条件、重试、Review-to-Fix 循环与人工干预。
- **S2 验证 Provider 独立性：** 增加其他本地 Provider，并支持可配置的 Capability 绑定。
- **S3 产品化 Runtime：** 通过 Tauri 桌面应用展示 Run、上下文、时间线、Artifact 与工作区。
- **S4 达到 Alpha 可用性：** 强化可靠性、安全性、打包与故障恢复。

## 成功标准

当用户可以完成以下操作时，MVP 即告完成：

1. 选择本地 Git 仓库并提供目标。
2. 运行 Capability Agent，而无需人工转移其输出。
3. 查看每个节点精确的 Context Snapshot、Artifact、执行日志与决策。
4. 在进程或桌面应用重启后恢复已持久化的工作流。
5. 让并发工作流 Run 隔离在各自独立的 worktree 中。
6. 当重试或循环次数耗尽时，在显式的人工决策点停止。

## 范围控制

原始 P0、P1 和 P2 要求已映射到[路线图](../roadmap/README.md)中的交付阶段。某项功能即使已有支撑它的抽象，也不会自动属于某个阶段；它必须明确列在该阶段的范围和退出条件中。
