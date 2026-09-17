# 当前版本使用指南

更新日期：2026-09-17。本文以开发分支 `feat/s0-codex` 为准，描述当前代码实际可用的入口。

## 当前可以做什么

当前版本可以构建 CLI、检查本机 Git/Codex 环境、初始化本地 SQLite 数据库，并通过在线 `doctor` 调用真实 Codex。Core 已具备事务执行账本、不可变 Artifact 存储，以及真实 Git Worktree provisioning 和内容级 Snapshot 基础能力。

`run`、`status`、`logs`、`resume`、`cancel` 仍只有参数解析，尚未连到完整的四能力工作流。看到帮助项不代表该工作流已经可用。

## 1. 进入开发 Worktree

```bash
cd /Users/mengweifei/Desktop/code/github/archmage/.worktrees/s0-codex
git branch --show-current
git log -1 --oneline
```

预期分支是 `feat/s0-codex`。如果只想使用已经合入 `main` 的较早版本，可以回主目录；本指南的新数据库和 Worktree 能力以开发 Worktree 为准。

## 2. 检查依赖

```bash
rustc --version
cargo --version
git --version
codex --version
codex login status
```

当前验证环境是 macOS、Rust/Cargo 1.95.0、Codex CLI 0.154.0。在线探针使用 Codex 当前登录和默认模型配置；Archmage 不会静默切换模型。

## 3. 构建与查看命令

```bash
cargo build -p archmage-cli
./target/debug/archmage --help
./target/debug/archmage --version
./target/debug/archmage doctor --help
./target/debug/archmage run --help
```

也可以不预先构建：

```bash
cargo run -p archmage-cli -- --help
```

第一个 `--` 用于把后面的参数传给 Archmage。

## 4. 设置独立数据目录

建议为开发验证指定仓库外的专用目录：

```bash
export ARCHMAGE_HOME="$HOME/Library/Application Support/archmage-dev"
```

该目录保存 `archmage.db` 和在线探针诊断。Unix 上新目录为 `0700`，敏感日志为 `0600`。不要把 `ARCHMAGE_HOME` 设为家目录、系统临时目录根或源码仓库目录，也不要提交或上传其中的原始诊断日志。

## 5. 不调用模型的离线诊断

```bash
cargo run -p archmage-cli -- doctor --offline
```

这条命令会检查 Git、Codex executable、必要 flags、数据目录和 SQLite。它不会请求模型，也不会验证账号是否真的能完成推理。成功 JSON 中应包含：

```json
{
  "authentication": "not_verified_offline",
  "database": {
    "schema_version": 4,
    "foreign_keys": true,
    "synchronous": 2,
    "journal_mode": "wal",
    "busy_timeout_ms": 5000
  }
}
```

离线 doctor 会创建或打开 `$ARCHMAGE_HOME/archmage.db`，但不会创建 Workflow Run。

## 6. 调用真实 Codex

```bash
cargo run -p archmage-cli -- doctor
```

该命令会明确提示即将进行在线请求，然后以 `read-only` sandbox 调用真实 `codex exec`。它使用本机账号和当前模型配置，可能消耗 Codex 使用额度。成功结果包含：

- `authentication=verified`
- 非空的 `probe.thread_id`
- `probe.exit_code=0`
- `probe.result={"ok":true}`
- `probe.diagnostics_dir`

每次调用都会在数据目录创建独立 `probe-*` 目录，保存 JSONL 事件、stderr、最终结果、Schema 和元数据。失败时目录同样保留，以便诊断。

还可以直接运行两层真实接入测试：

```bash
cargo test -p archmage-core --test codex_live real_readonly_probe \
  -- --ignored --nocapture

cargo test -p archmage-cli --test doctor_live \
  -- --ignored --nocapture
```

这两条命令都会实际访问模型服务；普通 `cargo test` 默认忽略它们。

## 7. 查看本地数据库

先确保已运行一次 doctor，然后可用系统 `sqlite3` 做只读检查：

```bash
sqlite3 "$ARCHMAGE_HOME/archmage.db" \
  'SELECT version, applied_at FROM schema_migrations ORDER BY version;'

sqlite3 "$ARCHMAGE_HOME/archmage.db" \
  'SELECT id, state, created_at FROM runs ORDER BY created_at DESC LIMIT 10;'
```

当前 doctor 不创建 Run，所以第二条通常没有输出。不要手工修改数据库；后续 Runtime 依赖 revision、外键和事件事务保持一致。

## 8. 验证持久化、Artifact 与真实 Worktree

这些命令不调用模型，但会使用临时 SQLite 数据库和临时真实 Git 仓库：

```bash
cargo test -p archmage-core --test persistence
cargo test -p archmage-core --test artifacts
cargo test -p archmage-core --test workspace
cargo test -p archmage-core --test processes
cargo test -p archmage-core --test ownership
cargo test -p archmage-core --test contracts
cargo test -p archmage-core --test context
```

Worktree 测试验证 committed baseline 隔离、partial provisioning 对账和 ChangeSet Artifact。进程测试仅启动普通本地 shell，验证双流读取、输出上限、进程启动身份，以及 TERM → bounded wait → KILL 对整个进程组的清理。ownership 测试验证单 Run OS lock 和旧 generation 不能提交结果；contracts 测试验证四个 Capability 的严格输出和业务不变量。

当前这些 Worktree API 仍是 Core 能力，尚未暴露为可执行的 `archmage run`。

## 9. 完整开发检查

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

普通测试不会调用真实 Codex；输出中的 ignored 是额度保护，不代表在线门禁已通过。

## 10. 尚未可用的工作流命令

以下命令已经定义参数，但当前会返回退出码 `20` 和 `not implemented`：

```bash
cargo run -p archmage-cli -- run \
  --repo /absolute/path/to/repository \
  --goal "实现目标" \
  --check "cargo test"

cargo run -p archmage-cli -- status <run-id> --json
cargo run -p archmage-cli -- logs <run-id> --follow
cargo run -p archmage-cli -- resume <run-id>
cargo run -p archmage-cli -- cancel <run-id>
```

这些命令列在这里是为了说明最终接口形态，不是当前可用示例。现在还没有通过 CLI 产生的 Run ID。

## 常见问题

### `codex executable not found on PATH`

```bash
command -v codex
codex --version
```

Archmage 使用当前终端的 `PATH`。如果尚未登录，运行 `codex login`；不要把 token 或认证文件内容复制到项目中。

### 在线 doctor 失败但离线 doctor 成功

离线检查只证明 executable 和本地参数存在。网络、账号、额度、默认模型或 CLI/模型兼容性仍可能让真实请求失败。查看错误中给出的 `probe.diagnostics_dir`，但不要直接公开原始日志。

### `workflow command is not implemented in this batch`

这是 `run/status/logs/resume/cancel` 的当前预期行为。owner generation、Run lock、四个 Capability Contract 和 Context/Handoff 已在 Core 完成；还需要 Check Runner 和调度器，才会开放纵向工作流。

详细进度见 [B 批次进度](acceptance/s0/batch-b-progress.md)，总体范围见 [S0 实现计划](superpowers/plans/2026-09-16-s0-real-codex-vertical-slice.md)。
