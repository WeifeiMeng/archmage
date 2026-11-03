简体中文 | [English](../../en/adr/0005-use-real-codex-only-in-product-flow.md)

# ADR 0005：产品流程只使用真实 Codex

**状态：** 已接受

**日期：** 2026-09-16

## 背景

S0 用于验证真实的本地 Agent 协作。模拟 Provider 可能让编排看似完成，却没有证明 Codex 调用、权限、Event、Contract 或 Workspace 行为。

## 决策

每次 S0 Capability Agent 调用都使用已安装且已认证的 Codex CLI。产品不提供 Fake Agent、dry-run Workflow 或模拟 Provider 模式。

自动化测试可以使用固定 JSONL 文档作为解析器 fixture，但它们不能执行 Workflow，也不能满足阶段验收。

## 后果

- 真实验收需要 Codex 认证、可用额度与受控仓库。
- 大多数 Runtime 逻辑仍可通过聚焦的单元与集成测试在无网络调用时验证。
- 发布证据必须区分基于 fixture 的解析器测试与真实 Codex 端到端 Run。
