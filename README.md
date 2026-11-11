<div align="center">

[简体中文](README.md) · [English](README.en.md)

<img src="assets/readme/archmage-hero.png" alt="Archmage 编排四种工程能力，在本地代码工作区中协作" width="100%" />

# Archmage

### 让本地 Agents 像一个真正的工程团队一样交付

**连接你已经在使用的 Agent。定义能力、团队与交付方向。让每一次修改都发生在隔离、可审计的 Git 工作区中。**

[![License: MIT](https://img.shields.io/badge/license-MIT-7c5cff.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/runtime-Rust-f6b73c.svg)](Cargo.toml)
[![Desktop](https://img.shields.io/badge/desktop-Tauri-24c8db.svg)](apps/archmage-desktop)
[![Status](https://img.shields.io/badge/status-development_preview-72d9a2.svg)](#当前边界)

[快速开始](#快速开始) · [它解决什么问题](#archmage-解决什么问题) · [工作方式](#一次-run-如何工作) · [Agent Provider](#连接真实的-agent-provider) · [文档](#文档与排障)

</div>

---

## Archmage 是什么？

Archmage 是一个**本地优先的多 Agent 协作运行时**。它不试图再造一个模型或聊天客户端，而是把本地 Agent Provider 组织成一个以 Capability 为中心的工程团队。

你可以将真实的 Codex CLI、DeepSeek Harness 或 Qoder ACP 实例绑定到不同能力，在画布上定义消息流，然后让 Runtime 在同一个隔离 Git worktree 中执行、验证并记录它们的交付。

```text
一个目标 → 一个隔离工作区 → 多个明确角色 → 结构化交付 → 可审计结果
```

当前内置四种逻辑 Capability：

| Capability | 主要职责 | 默认工作区权限 |
| :--- | :--- | :---: |
| **Designer** | 理解目标、检查仓库并给出实现设计 | 只读 |
| **Coder** | 根据设计修改代码并发布 Changeset | 可写 |
| **Tester** | 验证行为、运行检查并报告证据 | 可写 |
| **Reviewer** | 审查实现与测试证据，决定完成或反馈 | 只读 |

Capability 是稳定的责任边界；真正执行它的 Agent 可以替换。

## Archmage 解决什么问题？

打开多个 Agent 窗口并不等于协作。工程任务还需要回答：谁先做、交付什么、下一个人能相信哪些输入、代码在哪里修改，以及失败后如何恢复。

Archmage 把这些问题放进 Runtime，而不是留给提示词碰运气：

- **真实 Agent，不是模拟角色**：直接连接本机 Provider，并保留各 Provider 的模型和会话语义。
- **显式团队拓扑**：选择参与的 Capability、入口、终止节点和有向交付边。
- **隔离代码执行**：每个 Run 使用独立 Git worktree，避免直接污染原仓库。
- **结构化交付契约**：每个角色必须返回符合 JSON Schema 的结果；Runtime 验证后才继续路由。
- **可信 Artifact**：结果、Changeset、检查输出和上下文快照由 Runtime 持久化并关联来源。
- **可观察、可取消、可恢复**：Run 由事件和状态机驱动，而不是一段不可检查的聊天记录。

## 一次 Run 如何工作？

<div align="center">
  <img src="assets/readme/archmage-flow.png" alt="Designer、Coder、Tester 与 Reviewer 在隔离 worktree 中通过 Artifact 交付，并形成反馈回路" width="100%" />
</div>

1. **冻结定义**：Archmage 固化本次 Run 的 Team、Capability 配置和 Agent 绑定，后续设置修改不会悄悄改变正在运行的任务。
2. **创建工作区**：Runtime 从目标仓库创建隔离 worktree，并生成可验证的仓库快照。
3. **解析上下文**：每个 Capability 只收到目标、可信上游 Artifact、当前工作区状态和它被允许使用的指导。
4. **调用 Provider**：Runtime 按能力绑定启动 Codex、DeepSeek Harness 或 Qoder，并应用角色对应的沙箱边界。
5. **验证与发布**：输出通过角色契约后才会成为不可变 Artifact；Coder 的代码变化会额外发布为 Changeset。
6. **继续或完成**：Agent 在 Team 允许的边中选择下一位接收者；只有配置的终止 Capability 可以完成 Run。

反馈回路是显式的。例如你可以配置 `Coder → Reviewer → Coder`，让 Reviewer 的意见触发新一轮独立交付。Runtime 还设置步骤上限，避免无界循环。

## 连接真实的 Agent Provider

| Provider | 集成方式 | Run 内会话行为 |
| :--- | :--- | :--- |
| **Codex CLI** | 本地命令行调用与结构化输出 | 每次执行彼此独立 |
| **DeepSeek Harness** | 已发布的 Python SDK 与 Runtime binary | 同一 Capability 重复交付时保留会话 |
| **Qoder** | 本地 ACP Provider | 同一 Capability 重复交付时保留会话 |

在桌面端的 **Agent 管理**中发现和验证 Provider；在 **Capability 管理**中绑定 Agent、添加补充提示词和 Skill/Tool 指引；在 **团队管理**中绘制实际消息流。

> Skill/Tool 列表目前是提供给 Agent 的人工指导，不会强制权限，也不证明对应能力已经安装。内置角色契约保持固定，配置修改只影响新建 Run。

## 快速开始

### 安装开发版 CLI

在仓库根目录执行：

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
archmage doctor --offline
archmage run --repo /absolute/path/to/repository --goal "your goal"
```

查看 Run 状态和事件：

```bash
archmage status <run-id>
archmage logs <run-id>
```

### 启动桌面工作台

```bash
./scripts/dev-desktop.sh
```

这条命令会检查本地工具链、安装缺失的前端依赖，并同时启动 Vite UI 和包含 Rust Runtime 的 Tauri 进程。Archmage 不需要单独的后端 daemon。

需要单独排查前端或 Tauri 启动时，可以使用：

```bash
npm install --prefix apps/archmage-desktop
npm run tauri dev --prefix apps/archmage-desktop
```

## 本地优先，也以证据为先

Archmage 的边界刻意保持清晰：模型可以提出结果和路由建议，但 Runtime 拥有运行状态、Artifact ID、文件路径和状态转换。下游 Capability 读取的是已经提交并校验过的交付，而不是从自由文本里猜测事实。

```text
Agent 负责推理与创造
Runtime 负责身份、边界、状态、证据与交付
Git 负责呈现最终代码事实
```

## 当前边界

Archmage 仍处于开发预览阶段。当前可执行版本聚焦于本地代码仓库中的协作交付：

- 内置 Capability 固定为 Designer、Coder、Tester、Reviewer。
- 新建消息流 Team 至少包含两个 Capability，并明确入口和终止节点。
- Codex、DeepSeek Harness 与 Qoder 的会话能力不同，Runtime 保留这种差异。
- Archmage 不是长期记忆平台，也不会把手工 Skill/Tool 指引当作可执行权限系统。
- 运行命令与行为应以当前 CLI 手册和 Runbook 为准。

## 文档与排障

- [中文文档索引](docs/zh-CN/README.md)
- [CLI 命令手册](docs/zh-CN/cli-reference.md)
- [桌面客户端使用手册](docs/zh-CN/runbooks/desktop-s0.md)
- [Qoder ACP Provider 运行手册](docs/zh-CN/runbooks/qoder-provider.md)
- [当前版本指南与架构说明](docs/zh-CN/guide.md)
- [原始产品说明](target.md)

---

<div align="center">

**一个工作区，一条可见的交付链，一支真正协作的 Agent 团队。**

</div>
