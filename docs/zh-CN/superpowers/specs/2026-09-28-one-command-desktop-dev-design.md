简体中文 | [English](../../../en/superpowers/specs/2026-09-28-one-command-desktop-dev-design.md)

# 一条命令启动桌面开发环境设计

**状态：** 方案已批准，等待实施

## 目标

提供一条仓库根目录命令，启动 Archmage 桌面前端及其内嵌 Rust Runtime，同时保留底层命令便于诊断。

## 命令

添加可执行仓库脚本：

```bash
./scripts/dev-desktop.sh
```

脚本可从任意工作目录运行。它根据自身位置解析仓库根目录，并从该根目录启动桌面应用。

## 行为

脚本依次执行：

1. 检查 `git`、`cargo`、`node`、`npm` 与 `codex` 是否在 `PATH` 中。
2. 缺少依赖时输出简洁、可操作的错误并非零退出。
3. 仅当 `apps/archmage-desktop/node_modules` 不存在时，运行 `npm install --prefix apps/archmage-desktop` 安装前端依赖。
4. 使用 `npm run tauri dev --prefix apps/archmage-desktop` 替换脚本进程。

Tauri 开发命令负责应用两端：启动 Vite 前端并编译/运行包含 `archmage-core` 的 Rust shell。脚本不得额外启动后端 daemon 或第二个 Vite 进程。

`ARCHMAGE_HOME` 与调用者其他环境变量原样传递。脚本不选择账号、不改变 Codex 配置，也不在启动前运行在线模型 probe。

## 文档

根 README 将 `./scripts/dev-desktop.sh` 作为推荐桌面开发命令，并说明 Rust Runtime 内嵌在 Tauri 进程中，同时保留安装和排障的底层命令：

```bash
npm install --prefix apps/archmage-desktop
npm run tauri dev --prefix apps/archmage-desktop
```

桌面 Runbook 引用同一个单命令入口，避免两份指南分歧。

## 错误处理与验证

脚本使用严格 shell 模式并引用所有路径。依赖缺失错误指出具体 executable，依赖安装或 Tauri 失败则透传原始非零退出码。

验证保持精简：

- 使用 `bash -n scripts/dev-desktop.sh` 检查 shell 语法；
- 用受限 `PATH` 做轻量依赖缺失检查；
- 进行短时启动 smoke check，确认命令到达 Vite/Tauri 启动后终止，而不是运行 Codex Workflow。

脚本不添加 Fake Agent 行为，也不声称窗口启动即可证明真实 Codex Run 成功。
