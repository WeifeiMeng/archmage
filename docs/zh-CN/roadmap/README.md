简体中文 | [English](../../en/roadmap/README.md)

# 交付路线图

## 策略

原始说明把系统拆成横向里程碑 M0–M7。Archmage 改为交付垂直阶段 S0–S4，使每个阶段都产出可运行、可测试的增量。这改变的是顺序，而不是产品意图。

## 阶段

| 阶段 | 可运行成果 | 依赖 | 退出条件 |
|---|---|---|---|
| [S0](stages/s0-real-codex-slice.md) | 真实 Codex Provider 在固定 Workflow 中协调 Designer、Coder、Tester 与 Reviewer | 无 | 一条命令完成并恢复真实仓库 Run，且 Handoff 可审计 |
| S1：闭环 Workflow | 定义外部化；条件、重试、Fixer 循环与人工门禁可用 | S0 | Review 能批准，或在有界 Fix/Test/Review 迭代中路由 |
| S2：Provider 扩展 | Capability 可通过 Adapter 绑定 Claude Code、Qoder 或 Codex | S1 | 至少两个 Provider 在不改变 Capability 契约的情况下共同完成一个 Workflow |
| S3：桌面 MVP | Tauri 与 React 展示 Project、Run、节点上下文、Timeline、Workspace 和 Provider 管理 | S2 | 可从桌面启动、观察、中断并恢复 S1 Workflow |
| S4：Alpha 加固 | 故障恢复、权限、打包、诊断和可观测性达到 Alpha 质量 | S3 | 支持平台上的故障矩阵与发布清单全部通过 |

## 原始里程碑可追踪性

| 原始说明 | 主要交付 | 备注 |
|---|---|---|
| M0 Domain & Runtime Foundation | S0、S1 | S0 只实现真实垂直切片需要的模型；S1 泛化定义与控制流 |
| M1 Agent Execution Layer | S0、S2 | S0 验证 Codex；S2 用更多 Provider 验证 Adapter 架构 |
| M2 Workspace Runtime | S0 | 第一个切片就要求每个 Run 使用真实 Git worktree |
| M3 Capability & Contract | S0、S1 | S0 提供四个内置 Capability；S1 外部化带版本定义 |
| M4 Context + Workflow Engine | S0、S1 | S0 提供显式上下文与调度；S1 增加条件、循环、重试和人工门禁 |
| M5 Coding Collaboration Loop | S0、S1 | S0 验证 Design/Code/Test/Review；S1 增加 Fix 循环 |
| M6 Desktop Product MVP | S3 | Runtime 继续独立于 Tauri |
| M7 Stability & Alpha | S0、S4 | S0 建立崩溃安全不变量；S4 扩展故障矩阵与发布质量 |

## 优先级可追踪性

| 原始优先级 | 交付安排 |
|---|---|
| P0 | S0 完成协作主干；S1 闭环；S3 提供 Runtime 视图 |
| P1 | S1 开始人工门禁与权限；S2 增加 Provider 发现与使用；S3 增加 IDE 操作 |
| P2 | 保持在 S0–S4 之外，除非之后批准的 Roadmap 修订提升其优先级 |

## 阶段文档规则

只有当前或下一阶段会展开为详细规格与实施计划。之后的阶段只保留目标、依赖集合、范围边界和退出条件，直到前序证据足以支持负责任的详细设计。
