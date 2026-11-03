简体中文 | [English](../../../en/superpowers/specs/2026-10-30-investor-overview-deck-design.md)

# Archmage 投资人概览演示设计

## 目标

创建五页中文演示，向投资人解释 Archmage。先讲清产品价值，再介绍技术架构。

## 受众与信息

受众是评估问题、产品主张、差异化与系统可扩展性的投资人。核心信息是：

> Archmage 把彼此孤立的本地编码 Agent，转化为围绕同一真实代码工作区运行的可追踪协作系统。

不得声称仓库材料无法支持的市场规模、收入、采用量或性能数字。

## 叙事

1. **产品介绍** — Archmage 是面向软件开发的本地优先协作 Runtime，连接已有 Agent Provider，并围绕一个仓库组织 Capability Agent。
2. **协作痛点** — 用户目前需要手工搬运上下文，Agent 决策不透明，多个代码修改任务会冲突，执行证据也很分散。
3. **产品主张** — 设计以 Capability 为中心，使用显式 Context Snapshot 与契约驱动 Handoff，把 Git Workspace 作为共享事实，并记录可观测决策。
4. **解决方案流程** — 用户提供仓库和目标；Archmage 无需人工复制即可协调设计、编码、测试、评审与修复，同时让 Run 可检查、可恢复。
5. **架构与防御性** — 轻量桌面端和 CLI 位于 Rust 协作 Runtime 之上。Provider Adapter、Context/Contract 服务、Workspace 隔离与本地持久化带来 Provider 独立性和可复现执行。

## 视觉方向

- 16:9，暖白背景。
- 近黑文字，加一种取自当前 Archmage 视觉语言的柔和绿色强调色。
- 每页一个观点，大字号、充足留白，不使用装饰性卡片网格。
- 封面只放 Archmage 标志、名称和一句定位。
- 图表只保留必要节点，细线条、短标签。
- 不用渐变、纹理、图库图片、装饰图标或密集 UI 截图。
- 正文最小 18 pt；标题至少 32 pt。

## 内容约束

- 只使用产品与架构文档支持的主张。
- 准确呈现当前可执行路径：Codex Provider、Capability Agent、显式 Handoff、隔离 Git worktree、本地持久化、CLI 和 Tauri 桌面工作台。
- 将多 Provider 支持描述为架构方向，而不是当前已完成功能。
- PowerPoint 中全部文字与图表可编辑。

## 交付物与验证

最终交付一个五页 `.pptx` 文件。验证覆盖页数、文字适配、视觉重叠、与极简设计的一致性、可编辑性和演示文件校验成功。
