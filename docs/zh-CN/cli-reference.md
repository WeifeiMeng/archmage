简体中文 | [English](../en/cli-reference.md)

# Archmage CLI 命令手册

更新日期：2026-09-28。本手册描述当前代码的真实行为。

## 一分钟开始

前置条件：本机已有 Git、Rust/Cargo 和已登录的 Codex CLI。

```bash
git --version
cargo --version
codex --version
codex login status
```

在 Archmage 仓库根目录安装命令：

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
export PATH="$HOME/.local/bin:$PATH"
archmage doctor --offline
```

进入真实模型前先做一次在线检查：

```bash
archmage doctor
```

随后可以运行一个完整的四能力流程：

```bash
archmage run \
  --repo /absolute/path/to/your-repository \
  --goal "实现一个可验证的小功能"
```

`run` 会调用本机真实 Codex，并依次执行 Designer、Coder、Tester、Reviewer。命令会一直等待到 Run 结束，最后输出完整 JSON snapshot。记录输出中的 `id`，可用于后续查询。

## 已可用与暂不可用

当前可用：

- `archmage doctor [--offline]`
- `archmage run --repo ... --goal ...`
- `archmage status <run-id> [--json]`
- `archmage logs <run-id>`
- `archmage cancel <run-id>`

当前暂不可用：

- `archmage resume <run-id>`
- `archmage logs <run-id> --follow`

暂不可用的命令会明确报错，不会伪造执行结果。

## 安装、更新与卸载

安装或在源码更新后覆盖旧版本：

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
```

验证：

```bash
command -v archmage
archmage --version
archmage --help
```

如果 `command -v archmage` 没有输出，把安装目录加入 `PATH`：

```bash
export PATH="$HOME/.local/bin:$PATH"
```

只执行 `cargo build -p archmage-cli` 时，命令位于仓库的 `target/debug/archmage`，不会自动安装到 `PATH`：

```bash
cargo build -p archmage-cli
./target/debug/archmage --help
```

卸载：

```bash
cargo uninstall --root "$HOME/.local" archmage-cli
```

卸载 CLI 不会删除 Archmage 的数据库、工作区或诊断日志。

## 数据目录

Archmage 默认使用当前用户的操作系统应用数据目录。`archmage doctor --offline` 输出中的 `config.data_root` 是准确位置。

需要隔离开发数据时，为所有相关命令设置同一个 `ARCHMAGE_HOME`：

```bash
export ARCHMAGE_HOME="$HOME/Library/Application Support/archmage-dev"
archmage doctor --offline
```

`run`、`status`、`logs` 和 `cancel` 必须读取同一个数据目录，否则找不到对应 Run。

数据目录包含：

- `archmage.db`：Run、Node、Attempt、Event 与控制请求；
- `workspaces/`：每个 Run 的隔离 Git worktree；
- `diagnostics/` 或 `probe-*`：Codex 调用诊断文件。

这些内容可能包含本机路径、目标代码和模型事件，不要未经检查就公开或提交到 Git。

## `doctor`：启动前环境检查

不调用模型的检查：

```bash
archmage doctor --offline
```

它检查 Git/Codex executable、CLI 参数能力、数据目录和 SQLite migration。`authentication` 会显示 `not_verified_offline`。

调用一次真实 Codex 的检查：

```bash
archmage doctor
```

在线模式会在独立临时仓库中以只读权限请求满足 schema 的 `{"ok":true}`，验证当前 Codex 登录、模型连接、thread ID 和结构化输出。它可能消耗 Codex 使用额度，deadline 为 120 秒。

Doctor 只验证环境，不修改目标仓库，也不执行四能力 Workflow。

## `run`：执行四能力主链

最小用法：

```bash
archmage run \
  --repo /absolute/path/to/repository \
  --goal "为解析器增加空输入错误并验证行为"
```

`run` 不接受预制验收命令。Tester 能力背后的 Agent 会根据目标、仓库和上游产物自行决定测试与检查方法。

运行语义：

1. 为 Run 创建隔离 Git worktree；
2. Designer 产出设计和显式 handoff；
3. Coder 在该工作区实现并产出 ChangeSet；
4. Tester 自主选择测试命令，并在结构化结果中记录命令、退出码、结果和限制；
5. Reviewer 根据实现、Tester 结果和 handoff 给出 `approved` 或 `changes_requested`；
6. 全部状态和事件持久化到 SQLite。

这里的四个 Agent 是四种逻辑能力，并不要求四套不同的 provider 或常驻进程。当前可在桌面端把它们绑定到一个或多个真实 Codex 实例；CLI 使用已持久化的全局绑定，并在 Run 创建时冻结绑定快照。

## `status`：查看 Run

简洁输出：

```bash
archmage status <run-id>
```

JSON 输出：

```bash
archmage status <run-id> --json
```

输出包含 Run state/outcome、四个 Node 的状态、workspace 路径和最后事件序号。

## `logs`：读取持久化事件

```bash
archmage logs <run-id>
```

每行是一个 JSON event，按 sequence 顺序输出，适合配合 `jq`：

```bash
archmage logs <run-id> | jq -c '{sequence,event_type,node_id,payload}'
```

`--follow` 当前尚未实现；实时观察请使用桌面客户端。

## `cancel`：取消 Run

```bash
archmage cancel <run-id>
```

Archmage 会持久化取消请求；如果该 Run 有正在执行的 Codex 子进程，会先终止它，再把 Run 和未完成节点收敛到 `cancelled`。

## 进程意外退出后的恢复语义

当前没有后台 daemon。CLI 或桌面客户端退出后，不会继续在后台调度新节点。桌面客户端下次启动会检查遗留 Attempt：无法确认仍存活的执行会标记为 `interrupted`，而不是假装成功。

`resume` 尚未实现，所以被中断的 Run 目前需要重新创建。

## 常见问题

### `archmage: command not found`

```bash
ls "$HOME/.local/bin/archmage"
printf '%s\n' "$PATH"
export PATH="$HOME/.local/bin:$PATH"
```

### `codex executable not found on PATH`

```bash
command -v codex
codex --version
codex login status
```

Archmage 使用当前 `PATH` 和 Codex 配置，不读取或打印认证凭据。

### 离线 Doctor 成功，在线 Doctor 失败

离线成功不代表登录、网络、额度或模型服务可用。按错误中提供的 diagnostics 路径检查 `metadata.json`、`events.jsonl` 和 `stderr.log`，对外分享前先脱敏。

### 如何找到 Run ID

`archmage run` 启动时会在 stderr 打印 Run ID，结束后的 JSON 也包含 `id`。也可以在桌面客户端左侧的“最近运行”中查看。

`archmage internal check` 是保留的底层 CheckRunner 协议，不是当前内置主链的用户入口，不应手工调用或依赖其接口稳定性。
