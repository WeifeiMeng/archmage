简体中文 | [English](../../en/adr/0004-use-per-run-git-worktrees.md)

# ADR 0004：每个 Run 使用独立 Git Worktree

**状态：** 已接受

**日期：** 2026-09-16

## 背景

Capability Agent 需要共享的代码事实，而并发 Workflow Run 不能相互覆盖，也不能覆盖用户正在使用的 checkout。

## 决策

从源仓库已持久化的 `HEAD` 为每个 Workflow Run 创建一个 branch 与 Git worktree。同一 Run 的所有节点在读写 lease 约束下共享该 worktree。

## 后果

- 并发 Run 相互隔离。
- Dirty source 变更不会带入 Run，且必须明确报告。
- S0 保留 worktree 与 branch 供检查，绝不自动 merge 或 push。
- 清理与 branch 集成保留为后续显式功能。
