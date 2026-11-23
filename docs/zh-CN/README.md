简体中文 | [English](../en/README.md)

# Archmage 文档

本目录是产品范围、概念、架构、交付阶段、决策和操作流程的持续维护事实来源。原始[产品说明](../../target.md)作为输入保留，但不视为可执行规格。

## 推荐阅读顺序

最新开发增量：[桌面 Runtime 垂直切片](superpowers/specs/2026-09-28-desktop-runtime-vertical-slice-design.md)。

Team 组成更新：[可选择的 Capability 与关系](superpowers/specs/2026-10-14-selectable-team-capabilities-design.md)。用户自行创建 Team；应用不会预先创建 Coder → Reviewer 组合。

当前 Team 增量：[消息流 Team](superpowers/specs/2026-10-14-message-flow-teams-design.md)为新 Team 增加由 Agent 选择的交付路由、反馈循环和显式终止信号。[实施计划](superpowers/plans/2026-10-14-message-flow-teams.md)记录其开发过程。已有 Team 保留咨询语义。

**想安装桌面预览版？** 请从 [GitHub Releases](https://github.com/WeifeiMeng/archmage/releases) 下载，然后按照 [macOS 安装、升级与卸载指南](runbooks/macos-installation.md)操作。当前仅支持运行 macOS 14+ 的 Apple Silicon Mac。

**想运行当前代码？** 请阅读 [CLI 命令手册](cli-reference.md)或[桌面客户端使用手册](runbooks/desktop-s0.md)。[当前版本指南](guide.md)还包含贡献者测试和架构细节。

1. [CLI 命令手册](cli-reference.md)
2. [桌面客户端使用手册](runbooks/desktop-s0.md)
3. [macOS 安装、升级与卸载指南](runbooks/macos-installation.md)
4. [产品愿景](product/vision.md)
5. [MVP 定义](product/mvp-definition.md)
6. [术语表](concepts/glossary.md)
7. [协作模型](concepts/collaboration-model.md)
8. [架构概览](architecture/overview.md)
9. [跨 Provider 的 Capability 交付](architecture/cross-provider-handoff.md)
10. [交付路线图](roadmap/README.md)
11. [S0：真实 Codex 垂直切片](roadmap/stages/s0-real-codex-slice.md)
12. [桌面 Runtime 实施计划](superpowers/plans/2026-09-28-desktop-runtime-vertical-slice.md)

## 文档层级

| 层级 | 用途 | 变更策略 |
|---|---|---|
| `product/` | 产品存在的原因与成功标准 | 产品范围变化时更新 |
| `concepts/` | 稳定词汇与协作语义 | 引入冲突术语前更新 |
| `architecture/` | 当前系统边界与不变量 | 与已批准设计和代码保持同步 |
| `roadmap/` | 交付顺序、范围、依赖和退出条件 | 排期变化时更新 |
| `adr/` | 重要决策的持久记录 | 使用新 ADR 取代旧决策 |
| `runbooks/` | 可重复的配置、诊断与验收流程 | 发布前使用真实 CLI 验证 |
| `superpowers/specs/` | 已批准的阶段或功能设计 | 编写实施计划前评审 |
| `superpowers/plans/` | 文件级实施计划 | 仅在对应规格批准后创建 |

## 文档状态

必要时，文档会在开头明确标注状态。`Approved` 表示设计决策已获接受，并不代表软件已经实现。Roadmap 阶段会区分计划范围与已完成范围。
