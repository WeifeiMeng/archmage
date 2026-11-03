# CLI 安装和参考设计

简体中文 | [English](../../../en/superpowers/specs/2026-09-17-cli-install-and-reference-design.md)

**状态：** 建议实施

## 目标

使开发版本可作为正常的 `archmage` 命令使用，并提供一个真实的命令参考，将工作功能与计划命令明确区分开来。

## 安装合同

Archmage 使用 Cargo 的标准本地包安装：

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
```

这会将二进制文件安装到 `$HOME/.local/bin` 中，该文件已经是目标开发环境 `PATH` 的一部分。它使安装保持明确，而无需修改 shell 启动文件。必须通过以下方式验证安装是否成功：

```bash
command -v archmage
archmage --version
archmage --help
```

`cargo build` 相对于用户的 `PATH` 保持无副作用；它只产生 `target/debug/archmage`。Archmage 不会添加构建后挂钩、shell 别名或存储库相关符号链接。源更改后重新安装是明确的。

## 文件合同

创建 `docs/cli-reference.md` 作为规范 CLI 手册。它包含：

- 先决条件和一次性安装；
- upgrade/reinstall 和卸载命令；
- 数据目录行为和 `ARCHMAGE_HOME`；
- `doctor --offline` 和 `doctor` 的确切语义和示例；
- 简洁的输出字段指南；
- `run`、`status`、`logs`、`resume` 和 `cancel` 的清晰分隔部分，在连接运行时工作流程之前，所有部分都标记为不可用；
- 常见错误、退出状态含义以及安全诊断处理。

存储库自述文件和当前使用指南链接到规范手册，而不是复制竞争命令参考。

## 验证

实施验收要求：

1. 所有工作空间测试均通过；
2. `cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"` 成功；
3. 解析后的 `archmage` 二进制文件报告当前包版本和帮助；
4. `archmage doctor --offline` 对一次性数据目录成功；
5. 文档中没有任何示例声称不完整的工作流命令成功运行；
6. 格式化并通过 `git diff --check`。

## 非目标

- 打包签名的桌面或平台安装程序。
- Homebrew、npm 或发布二进制发行版。
- 使不完整的工作流程命令显得可操作。
- 自动更改 shell 启动文件或全局 `PATH` 配置。
