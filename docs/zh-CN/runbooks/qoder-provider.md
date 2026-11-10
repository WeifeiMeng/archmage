简体中文 | [English](../../en/runbooks/qoder-provider.md)

# Qoder ACP Provider

Archmage 可以通过 ACP v1，让任意 Designer、Coder、Tester 或 Reviewer Capability 使用本地 Qoder CLI 进程执行。本集成控制 CLI 进程及其 ACP session，不自动操作 Qoder IDE 或 WorkBuddy GUI。

## 安装与认证

按照 [Qoder 官方安装文档](https://docs.qoder.com/cli/installation)安装 Qoder CLI：

```sh
# macOS 或 Linux
curl -fsSL https://qoder.com/install | bash

# Windows PowerShell
irm https://qoder.com/install.ps1 | iex
```

先确认 `qoder --version` 可用，再运行 `qoder` 完成登录。Archmage 复用已有 Qoder CLI 登录状态，或继承启动环境中的 `QODER_PERSONAL_ACCESS_TOKEN`；不会保存或显示 Token。

## 配置实例

在 **Agent 管理**中添加 Qoder 实例，可配置：

- 可选的明确可执行路径；留空时从 `PATH` 查找；
- 可选模型与 Qoder Agent 名称；
- 显式扩展 JSON 对象，初始值为 `{ "mcpServers": [] }`；
- 对象内可选的 `pluginDirs`、`allowedTools`、`disallowedTools`、`agents` 与 `settings`。

Archmage 使用 ACP、严格 MCP 配置和空 setting sources 启动 Qoder。它只复用认证，不会隐式继承个人设置、Memory、默认 Agent、MCP、Plugin 或 Skill；只有实例中明确配置并冻结的扩展会参与 Run。

**检查 CLI**只读取本地版本。**验证连接**会创建临时只读 Git 仓库，协商 ACP v1、创建 session、执行一次真实模型请求，并且只接受要求的严格 JSON 对象。

## Session 与权限

每个存活的 `Run + Capability` 独占一个 Qoder 进程和 ACP session。同一 Capability 的后续 delivery 会复用该 session；不同 Capability 或 Run 绝不共享。咨询使用临时 session。

- Designer 和 Reviewer 只读；
- Coder 和 Tester 只能写入 Run worktree；
- ACP 文件路径会规范化，符号链接逃逸与外部路径会被拒绝；
- 终端 cwd 必须位于 worktree 内；提权命令和受保护的破坏性目标会被拒绝；
- 最终 assistant 输出必须是一个有界 JSON 对象；Markdown 代码围栏和修复猜测都会被拒绝。

Run 结束时关闭 session。取消会先发送 `session/cancel`，宽限期后再终止已登记的进程组。重启期间丢失的执行中 Attempt 会变为 `interrupted`，Archmage 不会静默伪造历史连续性。

## 诊断与验证

有界且私有的诊断文件保存在对应 Attempt 或 probe 目录中，不包含 Token、完整环境、无限会话正文或 thought 内容。

确定性测试使用仓库内的假 ACP 进程。真实认证 smoke test 命令：

```sh
cargo test -p archmage-core --test qoder_live -- --ignored --nocapture
```

真实测试需要已安装并登录的 Qoder CLI、网络和可用模型额度。

## 故障排查

- **可执行检查失败：**在启动 Archmage 的同一环境运行 `qoder --version`，或明确填写路径。
- **连接验证失败：**交互运行 Qoder 完成登录，或在 Archmage 启动环境提供 `QODER_PERSONAL_ACCESS_TOKEN`。
- **ACP 或契约错误：**确认 Qoder 版本支持 ACP v1，并让 Agent 只返回一个纯 JSON 对象。
- **配置被拒绝：**扩展必须是 JSON 对象；存在 `mcpServers` 时必须为数组。

另见[跨 Provider 的 Capability 交付](../architecture/cross-provider-handoff.md)。
