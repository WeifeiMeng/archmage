# Archmage CLI 命令手册

更新日期：2026-09-18。本手册描述 `main` 分支当前代码的真实行为。

## 当前状态

当前可以使用：

- `archmage --help`
- `archmage --version`
- `archmage doctor --offline`
- `archmage doctor`

以下命令只有参数定义，尚未接入完整 Workflow：

- `archmage run`
- `archmage status`
- `archmage logs`
- `archmage resume`
- `archmage cancel`

调用这些 Workflow 命令会返回退出码 `20` 和 `workflow command is not implemented in this batch`。命令出现在 `--help` 中，不代表已经可用。

## 前置条件

从源码安装需要：

```bash
rustc --version
cargo --version
git --version
codex --version
```

在线环境检查还要求本机 Codex CLI 已登录，并能访问当前配置的模型。Archmage 使用当前终端的 `PATH` 和 Codex 配置，不会静默切换账号或模型。

## 安装

在 Archmage 仓库根目录执行一次：

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
```

Cargo 会构建 `archmage-cli` package，并把其中名为 `archmage` 的 executable 安装到 `$HOME/.local/bin`。当前开发环境已经把该目录加入 `PATH`，因此安装后可以直接输入 `archmage`，同时无需修改 shell 启动文件。

验证安装：

```bash
command -v archmage
archmage --version
archmage --help
```

如果 `command -v archmage` 没有输出，请确认 `$HOME/.local/bin` 已加入 `PATH`：

```bash
export PATH="$HOME/.local/bin:$PATH"
```

`cargo build -p archmage-cli` 只生成仓库内的 `target/debug/archmage`，不会安装全局命令，也不会修改 shell 配置。只想临时执行开发构建时可以使用：

```bash
./target/debug/archmage --help
```

## 更新与卸载

源码更新后，在仓库根目录重新安装：

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
```

卸载 executable：

```bash
cargo uninstall --root "$HOME/.local" archmage-cli
```

卸载 CLI 不会自动删除 Archmage 数据目录、数据库或诊断文件。

## 数据目录

Archmage 默认使用操作系统提供的当前用户应用数据目录。准确路径会显示在 Doctor 输出的 `config.data_root` 字段中。

开发或隔离验证时，可以显式指定专用目录：

```bash
export ARCHMAGE_HOME="$HOME/Library/Application Support/archmage-dev"
archmage doctor --offline
```

数据目录必须是专用目录，不能是文件系统根、用户家目录、系统临时目录根或源码仓库内部。Unix 上新目录使用 `0700`；已有目录如果向其他用户开放，Archmage 会拒绝使用。

当前数据包括：

- `archmage.db`：SQLite 状态数据库；
- `probe-*`：在线 Doctor 的独立诊断目录；
- 后续 Workflow 产生的 Artifact 与运行状态。

诊断日志可能包含本机路径、模型事件或其他敏感上下文。不要直接上传、提交 Git 或公开整个诊断目录。

## 环境检查：Doctor

Doctor 是 Workflow 启动前的环境预检，不会执行产品 Workflow，也不会修改目标代码仓库。

### 离线检查

```bash
archmage doctor --offline
```

离线检查会：

- 查找 Git 和 Codex executable；
- 读取版本信息；
- 验证 Codex CLI 是否具有 Archmage 所需参数；
- 创建或检查数据目录；
- 打开 SQLite、执行 migration，并检查 WAL、外键和同步配置。

它不会调用模型，也不能验证账号或模型服务是否可用。成功输出中：

```json
{
  "authentication": "not_verified_offline",
  "database": {
    "schema_version": 5,
    "foreign_keys": true,
    "journal_mode": "wal",
    "synchronous": 2,
    "busy_timeout_ms": 5000
  }
}
```

### 在线检查

```bash
archmage doctor
```

在线检查先执行全部离线检查，然后真实调用 Codex：

1. 在数据目录中创建临时 Git 仓库；
2. 使用 `read-only` sandbox 调用 `codex exec`；
3. 要求模型返回满足 JSON Schema 的 `{"ok":true}`；
4. 验证进程退出码、Codex thread ID、完成事件和结构化结果。

该命令使用本机 Codex 登录和默认模型配置，可能消耗使用额度，deadline 为 120 秒。成功输出包含：

- `authentication="verified"`；
- 非空 `probe.thread_id`；
- `probe.exit_code=0`；
- `probe.result={"ok":true}`；
- `probe.diagnostics_dir`。

在线 Doctor 通过只说明 Git、Codex CLI、本机登录、模型连接和最小结构化输出能够工作。它不证明完整 Workflow 已实现，也不证明目标 Repository 能构建或测试通过。

## 输出字段

Doctor 向 stdout 输出一个 JSON object：

| 字段 | 含义 |
|---|---|
| `tools` | Git/Codex executable 的绝对路径与版本 |
| `config` | 数据目录和当前资源上限 |
| `authentication` | `not_verified_offline` 或在线探针成功后的 `verified` |
| `probe` | 离线时为 `null`；在线时为真实 Codex 探针结果 |
| `database` | SQLite schema 与关键 pragma 状态 |

提示信息和错误写入 stderr，成功 JSON 写入 stdout，便于脚本单独解析。

## 当前命令参考

### 查看版本

```bash
archmage --version
```

### 查看帮助

```bash
archmage --help
archmage doctor --help
```

### 不调用模型地检查环境

```bash
archmage doctor --offline
```

### 调用真实 Codex 检查环境

```bash
archmage doctor
```

## 尚未实现的 Workflow 命令

以下是已经冻结的未来接口形态，不是当前可执行教程：

```bash
archmage run \
  --repo /absolute/path/to/repository \
  --goal "实现目标" \
  --check "cargo test"

archmage status <run-id> --json
archmage logs <run-id> --follow
archmage resume <run-id>
archmage cancel <run-id>
```

完整 Workflow 仍缺少 Ready Set Scheduler、Runtime 主链和面向 CLI 的状态控制接线。执行上述命令时不会创建一个模拟成功的 Run。

`archmage internal check` 是 Runtime 与 Tester 之间的隐藏内部协议，不是用户命令。不要手工调用或依赖它的接口稳定性。

## 退出码

当前实际行为：

| 退出码 | 当前含义 |
|---:|---|
| `0` | 命令成功 |
| `2` | Clap 参数解析失败，例如缺少 required option |
| `20` | Archmage 执行错误，包括环境预检失败和当前未实现的 Workflow 命令 |

完整 Workflow 计划保留以下终态退出码，但目前尚未接入 CLI：

| 退出码 | 预留含义 |
|---:|---|
| `10` | Reviewer 返回 `changes_requested` |
| `21` | Capability、Contract 或 Workspace invariant 失败 |
| `22` | Run 被取消 |

## 常见问题

### `archmage: command not found`

确认安装位置和 `PATH`：

```bash
cargo install --list | rg archmage-cli
printf '%s\n' "$PATH"
ls "$HOME/.local/bin/archmage"
```

如果自行省略了 `--root "$HOME/.local"`，Cargo 通常会改装到 `$CARGO_HOME/bin` 或 `$HOME/.cargo/bin`；此时需要自行保证该目录位于 `PATH`。

### `codex executable not found on PATH`

```bash
command -v codex
codex --version
codex login status
```

Archmage 不读取或打印认证凭据；它只调用当前 `PATH` 中解析到的 Codex CLI。

### 离线 Doctor 成功，在线 Doctor 失败

离线成功只证明本机 executable、数据目录和数据库可用。登录状态、网络、额度、模型配置或 CLI/模型兼容性仍可能导致在线请求失败。

错误信息会给出 `probe.diagnostics_dir`。可以在本机检查其中的 `metadata.json`、`events.jsonl` 和 `stderr.log`，但不要未经脱敏就对外分享。

### `workflow command is not implemented in this batch`

这是当前 `run`、`status`、`logs`、`resume` 和 `cancel` 的预期行为，不是安装失败。请以本手册顶部的“当前状态”为准。
