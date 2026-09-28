# Archmage 桌面客户端使用手册

桌面客户端是当前 Runtime 的本地工作台。它直接调用 `archmage-core`，不是解析 CLI stdout，也没有额外后台 daemon。

## 当前可以做什么

- 选择本地 Git 仓库；
- 输入目标和一条或多条验收命令；
- 启动 Designer → Coder → Tester → Reviewer 的真实 Codex Run；
- 查看最近 Run、四个能力节点状态和持久化事件时间线；
- 查看节点相关事件；
- 打开隔离工作区；
- 取消正在运行的 Run；
- 客户端重启时收敛遗留的 interrupted Attempt。

当前不包含图形化 resume、工作流编辑器、节点自由编排和后台常驻执行。

## 启动开发客户端

前置检查：

```bash
git --version
cargo --version
node --version
npm --version
codex --version
codex login status
```

在 Archmage 仓库根目录一键启动：

```bash
./scripts/dev-desktop.sh
```

脚本会检查 Git、Cargo、Node.js、npm 和 Codex CLI；仅当 `node_modules` 不存在时安装前端依赖，然后启动 Tauri。Tauri 会同时启动 Vite 界面和内嵌 `archmage-core` 的 Rust Runtime，不需要单独启动后端 daemon。

第一次启动会安装或编译依赖，因此会比后续启动慢。看到 Vite local URL 和 `Running .../archmage-desktop` 后，桌面窗口会打开。

如果需要隔离数据，在启动前设置专用目录：

```bash
export ARCHMAGE_HOME="$HOME/Library/Application Support/archmage-desktop-dev"
./scripts/dev-desktop.sh
```

脚本会原样传递 `ARCHMAGE_HOME` 和当前 Codex 配置。

如果需要排查 npm 或 Tauri 启动问题，可以分步执行底层命令：

```bash
npm install --prefix apps/archmage-desktop
npm run tauri dev --prefix apps/archmage-desktop
```

桌面端和 CLI 使用相同的数据格式。只要 `ARCHMAGE_HOME` 相同，就能看到相同的 Run。

## 创建第一个 Run

1. 点击“新建 Run”。
2. 点击“选择”，选一个至少已有一次 commit 的本地 Git 仓库。
3. 在“目标”中写清希望最终实现和验证的结果。
4. 填写验收命令，例如 `cargo test`；需要多条时点击“添加命令”。
5. 点击“启动真实 Run”。

启动后界面会切换到 Run 工作台：

- 顶部显示 Run state、仓库和 Run ID；
- 四能力状态条显示当前执行到哪一环；
- 时间线显示 SQLite 中的持久化事件；
- 点击能力节点可在右侧检查相关事件；
- “工作区”在 Finder 中定位该 Run 的隔离 worktree；
- “取消”终止当前真实执行并持久化 cancelled 状态。

## 关闭与恢复

只要还有本客户端启动的 Run 在执行，关闭窗口时会询问是否取消这些 Run。选择继续关闭后，客户端先发出取消，再退出。

Archmage 当前不在客户端退出后继续调度。若进程崩溃或被强制结束，下次启动会把无法确认仍存活的遗留 Attempt 标记为 `interrupted`。图形化 resume 尚未提供。

## 与 CLI 配合

查看桌面端创建的 Run：

```bash
archmage status <run-id> --json
archmage logs <run-id>
```

从另一个终端取消：

```bash
archmage cancel <run-id>
```

如果 CLI 查不到桌面端的 Run，通常是两边的 `ARCHMAGE_HOME` 不一致。

## 简单验证

前端单测和生产构建：

```bash
npm test --prefix apps/archmage-desktop
npm run build --prefix apps/archmage-desktop
```

Rust/Tauri 编译检查：

```bash
cargo check -p archmage-desktop
```

完整命令说明见 [CLI 命令手册](../cli-reference.md)。
