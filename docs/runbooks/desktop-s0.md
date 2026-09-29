# Archmage 桌面客户端使用手册

桌面客户端是当前 Runtime 的本地工作台。它直接调用 `archmage-core`，不是解析 CLI stdout，也没有额外后台 daemon。

## 当前可以做什么

- 选择本地 Git 仓库；
- 配置本地 Codex Agent 实例并执行真实连通验证；
- 在独立的 Capability 管理页查看四个职责的固定拓扑，并选择各自的 Agent 实例；
- 只输入仓库和目标即可创建 Run；
- 启动 Designer → Coder → Tester → Reviewer 的真实 Codex Run；
- 查看最近 Run、四个能力节点状态和持久化事件时间线；
- 查看节点相关事件；
- 打开隔离工作区；
- 取消正在运行的 Run；
- 客户端重启时收敛遗留的 interrupted Attempt。

当前不包含图形化 resume、团队 Canvas、工作流编辑器、节点自由编排和后台常驻执行。

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

## 配置并验证本地 Agent

1. 点击侧栏“Agent 管理”。
2. 默认的 `codex-default` 会从当前 `PATH` 自动寻找 `codex`；如需使用指定安装，填写 Codex CLI 的绝对路径并保存。
3. 模型覆盖可以留空，此时沿用 Codex 自身配置；填写后仅该 Agent 实例使用指定模型。
4. 点击“检查 CLI”，只检查可执行文件、版本、必需参数和 Git；此时状态是“CLI 可用 · 未验证”，不能据此判断账号已连通。
5. 点击“验证连接”。该操作会使用当前实例的可执行文件和模型覆盖发起一次真实、只读的 Codex 请求，并展示版本、可用状态和上次验证时间。
6. 点击侧栏“Capability 管理”，在拓扑中逐个选择 Designer、Coder、Tester、Reviewer，确认其“执行 Agent”指向预期实例。需要调整时选择 Agent，点击“保存 Agent 绑定”。能力由 Agent 和职责提示词组成；本版提示词由 Runtime 内置，暂不提供编辑入口。

“添加 Codex 实例”允许创建例如 `codex-personal` 和 `codex-work` 的多个配置。能力绑定是全局设置，不在单个 Run 中临时覆盖；Run 启动时会冻结当时的绑定及能力配置。修改只影响之后创建的 Run。

拓扑中的实线是执行依赖：Designer → Coder → Tester → Reviewer。虚线是额外的上下文传递：Designer → Tester/Reviewer、Coder → Reviewer；相邻节点的实线也会传递上下文。这一版不能拖动节点或改线，之后的团队管理 Canvas 才会设计可编辑拓扑。

绑定修改未保存时，切换节点、离开 Capability 页面或关闭窗口都会先询问是否放弃；若另一处已更新同一能力，保存会提示冲突，可点击“重新加载”后重新选择。

未绑定的实例可以禁用或删除；已绑定的实例需要先把相关能力切换到其他实例。新建 Run 页面会显示缺失或不可用的能力绑定，并提示尚未通过真实连接验证的实例。

Claude、OpenClaw 和 Hermes 在当前页面只显示“尚未支持”，不会执行假探测或被绑定到能力。

## 创建第一个 Run

1. 点击“新建 Run”。
2. 点击“选择”，选一个至少已有一次 commit 的本地 Git 仓库。
3. 在“目标”中写清希望最终实现和验证的结果。
4. 点击“启动真实 Run”。Tester 会读取目标、设计、实现、ChangeSet 与工作区，自主选择并执行适合该项目的验证命令。

启动后界面会切换到 Run 工作台：

- 顶部显示 Run state、仓库和 Run ID；
- 四能力状态条显示当前执行到哪一环；
- 每个能力节点显示该 Run 冻结的 Agent 实例；
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
