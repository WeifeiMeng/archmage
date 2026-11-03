简体中文 | [English](../../en/adr/0002-separate-provider-and-capability-agent.md)

# ADR 0002：分离 Provider 与 Capability Agent

**状态：** 已接受

**日期：** 2026-09-16

## 背景

即使用户最初只安装了一个本地 Agent 产品，产品也必须支持多 Agent 协作。

## 决策

Provider 是物理执行后端。Capability Agent 是把 Capability 绑定到 Provider 后形成的逻辑角色。S0 使用一个 Codex CLI Provider 执行 Designer、Coder、Tester 与 Reviewer 四个 Capability Agent。

## 后果

- 多 Agent 语义不要求多个供应商产品或持久实例。
- Provider 配置与 Capability 契约可独立演进。
- S2 可以把某个 Capability 移到其他 Provider，而无需改变 Handoff 消费者。
- UI 与文档在实际意指 Provider 或 Capability Agent 时，不得含糊地使用“Agent”。
