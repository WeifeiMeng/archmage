# 当前版本使用指南

> **2026-09-17 更新：** 开发 Worktree 已支持在线 doctor、增量解析和持久化诊断日志，见 [在线 Doctor 使用说明](guide-online-doctor.md)。下文保留上次 main 版本的指南；其中“在线 doctor 未实现”仅适用于尚未合入本次更新的 main。

更新日期：2026-09-16。适用范围：已合入本地 `main` 的 S0 A 批次阶段代码。

## 先说现在能做什么

目前是开发中的 CLI 原型，**还不能输入开发目标后自动完成设计、编码、测试、Review**。现在可以编译 CLI、检查本机运行条件，以及通过集成测试真正调用一次 Codex。

| 入口 | 当前状态 |
|---|---|
| `--help`、`--version`、子命令 `--help` | 可用 |
| `doctor --offline` | 可用：检查 Git/Codex、必要参数和本地数据目录 |
| 真实只读 Codex 探针 | 可用，但入口暂时是 Cargo 集成测试 |
| `doctor`（不带 `--offline`） | 尚未接入在线探针，返回未实现错误 |
| `run`、`status`、`logs`、`resume`、`cancel` | 只有参数解析，业务尚未实现 |
| 桌面 UI、持久化 Run、多能力协作 | 尚未实现 |

帮助里出现某个命令，不代表它的业务已经实现。路线图、Spec 和实现计划描述的是目标，本指南描述的是当前实际行为。

## 1. 进入正确的代码目录

这台机器上，阶段代码已经合入本地主仓库，可以直接在主目录运行：

```bash
cd /Users/mengweifei/Desktop/code/github/archmage
git branch --show-current
```

预期分支为 `main`，该目录现在包含 `Cargo.toml`。后续命令均在这个目录运行。

Provider 和探针代码已合入本地 `main`，尚未推送远端。重新 clone 远端仓库不一定包含这些能力。原来的 `.worktrees/s0-codex` 开发目录仍保留，但日常试用不需要切换过去。

## 2. 检查前置工具

```bash
rustc --version
cargo --version
git --version
codex --version
codex login status
```

当前验证环境：macOS、Rust/Cargo 1.95.0、Codex CLI 0.154.0，Codex 已通过 ChatGPT 登录。

工程的 `rust-toolchain.toml` 固定为 1.95.0；使用 rustup 时需要对应工具链。首次 Cargo 构建需要下载依赖。Codex 必须在当前终端的 `PATH` 中；未登录时先自行运行 `codex login`。不要把 token、API key 或认证文件内容粘贴到项目里。

## 3. 编译并查看帮助

```bash
cargo build -p archmage-cli
./target/debug/archmage --help
./target/debug/archmage --version
```

版本输出为 `archmage 0.1.0`。目前不需要全局安装 Archmage，也不需要 Node.js 前端或 Tauri。

也可以用 Cargo 直接运行：

```bash
cargo run -p archmage-cli -- --help
```

第一个 `--` 将后面的参数传给 Archmage，而不是 Cargo。

## 4. 运行本地诊断

为了便于找到数据，建议显式设置一个仓库之外的专用目录：

```bash
ARCHMAGE_HOME="$HOME/Library/Application Support/archmage-dev" \
  cargo run -p archmage-cli -- doctor --offline
```

这条命令会：

- 定位 Git、Codex 可执行文件并读取版本。
- 检查 Codex 是否支持所需的非交互参数。
- 创建或验证 Archmage 数据目录，并通过临时文件检查可写性。
- 输出 JSON，包括 `tools`、`config`、`authentication` 和 `database`。

其中两个字段应当是：

```json
{
  "authentication": "not_verified_offline",
  "database": "not_implemented_batch_b"
}
```

这不是异常：离线诊断不调用模型，不验证认证是否可用于请求；数据库还未实现。退出码为 `0` 只代表这些本地检查通过，不代表工作流已可用。

不设置 `ARCHMAGE_HOME` 时采用系统应用数据目录，实际路径见输出的 `config.data_root`。Unix 上新建目录权限为 `0700`；已有目录权限过宽时会拒绝使用，不会自动修改已有目录权限。不要把该变量设置为家目录、临时目录根或源码仓库目录。

## 5. 真正调用一次 Codex

目前真实接入入口是这个显式启用的测试：

```bash
cargo test -p archmage-core --test codex_live real_readonly_probe \
  -- --ignored --nocapture
```

它会使用你本机 Codex 的登录和默认模型配置，实际访问模型服务并消耗相应使用额度。没有模拟 Agent，也不会自动替换模型。

执行过程：

1. 创建临时 Git 仓库，不修改你的业务仓库。
2. 以 `read-only` sandbox 启动真实 `codex exec`。
3. 通过 stdin 发送只要求返回 `{"ok":true}` 的提示。
4. 获取 JSONL 事件、独立 thread ID 和 Schema 约束的最终结果。
5. 检查进程退出成功、完成事件存在、结果恰好符合探针 Contract。

成功输出包含：

```text
version=codex-cli 0.154.0 thread=<实际生成的 ID> exit=0 result={"ok":true}
test real_readonly_probe ... ok
test result: ok. 1 passed; 0 failed; 0 ignored
```

`thread` 每次不同。探针调用限时 120 秒；网络、账号、配额或模型不兼容都会使测试失败。

`--ignored` 是必须的：该测试为避免默认消耗额度而标记为忽略。只运行普通 `cargo test` 时看到 `ignored` 不代表真实接入验证通过。

探针的临时仓库和结果文件在结束后清理；当前没有 Run ID、SQLite Timeline 或持久化探针日志。Codex 自身的会话存储不由这个临时目录清理。此次只证明连接与结构化输出可用，尚不是四个 Capability Agent 的协作验收。

## 6. 跑开发检查

```bash
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

目前离线测试覆盖 CLI 参数、读写兼容规则、目录权限与路径验证、JSONL 分片解析、进程双流输出、超时及输出大小限制。最近一次离线结果为 20 项通过、1 项真实 Codex 测试默认忽略。

## 7. 常见问题

### `could not find Cargo.toml`

当前目录不包含源码，或使用了尚未更新的远端 checkout。回到第 1 节的本地主仓库绝对路径，确认存在 `Cargo.toml`。

### `codex executable not found on PATH`

在同一终端运行 `command -v codex`。Archmage 使用当前终端 PATH，不读取桌面应用内部的 Codex 二进制路径。

### 模型要求更新的 Codex

本机曾出现 Codex 0.142.4 无法使用配置的 `gpt-5.6-sol`，升级至 0.154.0 后同一探针通过。不要只凭 `login status` 成功就判断模型请求可用，也不要为通过测试而静默换模型。

### `data root ... mode 0700` 或不可写

选择你拥有的全新专用目录。诊断会实际创建临时文件；目录权限、ACL 和只读文件系统都可能导致失败。不要为解决权限问题使用 `sudo` 运行 Agent。

### `online authentication probe is not implemented in this batch`

你运行了不带 `--offline` 的 `doctor`。当前在线探针还没连到 CLI；请使用第 5 节的集成测试。注意该命令报错前仍可能已经创建数据目录。

### `workflow command is not implemented in this batch`

`run/status/logs/resume/cancel` 暂不可用，返回退出码 `20` 是当前预期行为。现在还没有可供这些命令使用的 Run ID。

## 下一步会增加什么

先完成在线 `doctor`、增量事件流和诊断日志，再接入 SQLite、真实 per-run Worktree、四个能力 Contract 和协作调度。

详细范围见 [S0 实现计划](superpowers/plans/2026-09-16-s0-real-codex-vertical-slice.md)；真实接入记录见 [A 批次进度](acceptance/s0/batch-a-progress.md)。
