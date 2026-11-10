简体中文 | [English](README.en.md)

# Archmage

Archmage 是一个本地优先的协作运行时，用于连接本地 Agent Provider，让以 Capability 为中心的多个 Agent 在同一个代码工作区中协作。

当前可执行版本可以连接真实的 Codex CLI、已发布的 DeepSeek Harness Python SDK 或本地 Qoder ACP Provider，并在隔离的 Git worktree 中，通过显式且可审计的交付运行四个逻辑 Capability：Designer、Coder、Tester 和 Reviewer。用户既可以通过 CLI 使用，也可以通过 Tauri 桌面工作台使用。

在仓库根目录安装当前开发版 CLI：

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
archmage doctor --offline
archmage run --repo /absolute/path/to/repository --goal "your goal"
```

在仓库根目录启动完整的桌面开发应用：

```bash
./scripts/dev-desktop.sh
```

这条命令会检查本地工具链、安装缺失的前端依赖，并同时启动 Vite UI 和内嵌 Rust Runtime 的 Tauri 进程。Archmage 不需要单独的后端 daemon。

在桌面窗口中，使用 **Agent 管理**验证本地 Codex、Qoder ACP 连接，或者验证包含匹配版本 `deepseek-harness-sdk` 与 `deepseek-harness-runtime-bin` 的 Python 环境。DeepSeek Harness Agent 会显式配置 Provider route 和 model，同时继承 Archmage 环境中的凭证与 endpoint；Qoder 实例显式配置模型、Agent 与扩展，只复用 CLI 认证。使用 **Capability 管理**为每个角色绑定 Agent，并编辑补充提示词以及手工维护的 Skill/Tool 指引；使用 **团队管理**创建消息流 Team。至少选择两个 Capability，指定入口和终止 Capability，然后在画布上绘制有向边。例如，可以自行绘制 Coder → Reviewer 与 Reviewer → Coder；系统不会自动创建这样的 Team。同一 Run 中，重复交付到某个 DSH 或 Qoder Capability 会保留该 Capability 的会话，而 Codex 的每次执行彼此独立。已有 Team 保留之前的咨询行为。内置角色契约保持固定；Skill/Tool 列表只为真实 Agent 提供指引，不会强制权限，也不证明相关能力实际可用。配置修改只影响新建的 Run。

排查问题时，可以使用底层命令：

```bash
npm install --prefix apps/archmage-desktop
npm run tauri dev --prefix apps/archmage-desktop
```

可执行命令以[CLI 命令手册](docs/zh-CN/cli-reference.md)和[桌面客户端使用手册](docs/zh-CN/runbooks/desktop-s0.md)为准。Qoder 配置见 [Qoder ACP Provider 运行手册](docs/zh-CN/runbooks/qoder-provider.md)。[当前版本指南](docs/zh-CN/guide.md)还包含更深入的贡献者验证与架构说明。

建议从[中文文档索引](docs/zh-CN/README.md)开始阅读。原始产品说明仍保存在 [target.md](target.md)。
