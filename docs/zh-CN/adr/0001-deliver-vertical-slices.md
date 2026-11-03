简体中文 | [English](../../en/adr/0001-deliver-vertical-slices.md)

# ADR 0001：交付垂直切片

**状态：** 已接受

**日期：** 2026-09-16

## 背景

原始说明先安排横向基础设施，再进行真实 Agent 执行与产品验证。这种顺序可能在协作契约尚未经过真实 Provider 和仓库验证前，就产生泛化抽象。

## 决策

以可运行的垂直阶段 S0–S4 交付。S0 横跨原始 M0–M5 层级，验证一条真实 Codex 协作路径。后续阶段仅在取得前序证据后再泛化。

## 后果

- S0 必须同时实现持久化、Workspace、Capability、Context、Contract 与 Workflow 的精简版本。
- 部分 S0 定义内置且有意不允许用户编辑。
- Roadmap 保持对 M0–M7 与 P0–P2 的显式可追踪性，确保调整顺序不会遗漏需求。
