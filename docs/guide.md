# 当前版本使用指南

更新日期：2026-09-28。本文描述当前源码的真实入口；日常参数以 [CLI 命令手册](cli-reference.md) 为准，图形界面以 [桌面客户端使用手册](runbooks/desktop-s0.md) 为准。

## 当前能力

Archmage 已具备一条可运行的纵向主链：连接本机真实 Codex CLI，在每个 Run 的隔离 Git worktree 中依次调度 Designer、Coder、Tester、Reviewer 四种逻辑能力，并持久化 Run、Node、Attempt、Artifact、Handoff、ChangeSet、Agent 绑定快照和事件。

可用入口：

- CLI：`doctor`、`run`、`status`、`logs`、`cancel`；
- Tauri 桌面客户端：管理本地 Agent 与能力绑定、新建 Run、查看四节点进度和时间线、打开工作区、取消 Run；
- 进程恢复：启动时将无法确认存活的遗留 Attempt 收敛为 `interrupted`。

暂未提供：`resume`、`logs --follow`、后台 daemon、任意工作流编排。

## 1. 准备环境

```bash
cd /Users/mengweifei/Desktop/code/github/archmage
rustc --version
cargo --version
git --version
node --version
npm --version
codex --version
codex login status
```

Archmage 使用当前 `PATH` 中的 Git/Codex 和当前 Codex 登录、模型配置，不会静默切换账号或模型。

如需隔离开发数据：

```bash
export ARCHMAGE_HOME="$HOME/Library/Application Support/archmage-dev"
```

CLI 与桌面端必须使用相同的 `ARCHMAGE_HOME`，才能看到同一批 Run。

## 2. 安装并运行 CLI

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
export PATH="$HOME/.local/bin:$PATH"
archmage doctor --offline
archmage doctor
```

执行真实四能力流程：

```bash
archmage run \
  --repo /absolute/path/to/repository \
  --goal "实现一个可验证的小功能"
```

查询和取消：

```bash
archmage status <run-id> --json
archmage logs <run-id>
archmage cancel <run-id>
```

`run` 会等待终态；`logs` 读取持久化事件但暂不支持 `--follow`；`resume` 暂未实现。

## 3. 启动桌面客户端

```bash
./scripts/dev-desktop.sh
```

第一次使用时先进入“Agent 管理”，检查本地 Codex CLI，再点击“验证连接”执行真实只读请求。确认 Designer、Coder、Tester、Reviewer 都绑定到可用实例后，回到“新建 Run”，选择已有 commit 的 Git 仓库并填写目标即可。详细操作和关闭语义见 [桌面客户端使用手册](runbooks/desktop-s0.md)。

## 4. Runtime 的关键语义

- 一个 Run 包含多个 Node，当前内置四个能力节点；
- 多能力不等于必须启动多个长期 Agent 实例；底层支持多个 Codex 实例，首版默认创建一个 `codex-default`；
- 能力绑定只在 Agent 管理页全局配置，不支持 Run 级临时覆盖；
- Run 启动时冻结当前能力到具体 Agent 实例的绑定，后续修改全局设置不会改变历史 Run；
- 每个节点有独立 Codex thread/result 与显式 Context/Handoff；
- Tester 直接获得测试任务、工作区和上游产物，由绑定的真实 Codex 自主选择测试方式与命令；
- 所有状态转换与事件写入 SQLite；
- 每个 Run 使用独立 Git worktree，用户仓库 checkout 不作为执行工作区；
- 没有 daemon：客户端或 CLI 退出后不会继续调度新节点。

## 5. 开发验证

不调用模型的常规检查：

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm test --prefix apps/archmage-desktop
npm run build --prefix apps/archmage-desktop
cargo check -p archmage-desktop
git diff --check
```

需要真实 Codex 的测试默认标记为 ignored，普通 `cargo test` 不会消耗模型额度。在线环境探针：

```bash
cargo test -p archmage-core --test codex_live real_readonly_probe \
  -- --ignored --nocapture
```

Runtime 主链的最终验收应使用一个临时 Git fixture 和真实 Codex，确认四个节点都进入成功终态、Tester 记录实际选择的命令，并由 Reviewer 给出 outcome。

## 6. 数据与诊断

`archmage doctor --offline` 输出的 `config.data_root` 是实际数据目录。主要内容包括：

- `archmage.db`：schema v8 的执行账本、Agent 实例、能力绑定与事件；
- Run worktree 与 Artifact；
- Codex JSONL、stderr 和 metadata 诊断。

诊断可能包含本机路径、目标代码或模型上下文，不要未经脱敏就上传或提交 Git。不要手工修改 SQLite；Runtime 依赖 revision、事务、外键和事件顺序维持状态一致性。

## 常见问题

### CLI 查不到桌面端创建的 Run

检查两边的 `ARCHMAGE_HOME` 是否一致。未显式设置时，两者都使用系统应用数据目录。

### 离线 Doctor 成功，在线 Doctor 失败

离线检查不验证登录、网络、额度或模型服务。查看错误给出的 diagnostics 路径，并在分享前脱敏。

### 关闭客户端后 Run 会继续吗

不会。正常关闭时客户端会提示并取消当前会话启动的活跃 Run；异常退出留下的 Attempt 会在下次启动时被标记为 `interrupted`。当前需要重新创建 Run，等待后续实现 `resume`。
