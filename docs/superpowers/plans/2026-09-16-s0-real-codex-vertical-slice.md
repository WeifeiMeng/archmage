# S0 Real Codex Vertical Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended when delegation is authorized) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 通过一个本地 Codex Provider，让 Designer、Coder、Tester、Reviewer 四个 Capability Agent 在真实 Git Worktree 中完成可追踪、可中断恢复的协作。

**Architecture:** 两个 Rust crate：`archmage-core` 承担领域、存储、Workspace、Provider、Context、Contract 与调度；`archmage-cli` 只负责参数、交互输出和退出码。每个 attempt 调用真实 `codex exec`；同 Run 的并发由依赖和 Workspace 访问租约控制，不用 Fake Agent。

**Tech Stack:** Rust、Tokio、Serde/JSON、SQLite（rusqlite）、Clap、JSON Schema、Git CLI、Codex CLI。依赖在 Task 01 锁入 Cargo.lock，具体 API 以锁定版本为准。

---

## 0. 执行约定

- 当前文档是待执行计划，不代表下列源码、命令或测试已存在。
- 基线：`07f8589`；批准依据：[S0 Spec](../specs/2026-09-16-real-codex-vertical-slice-design.md)。启动实施前重查分支、`AGENTS.md` 和用户未提交内容。
- 本轮只编写计划。实施时使用隔离开发分支/worktree；不要直接在 `main` 上开发。
- 所有路径均相对仓库根目录；产品数据通过 `ARCHMAGE_HOME` 指向仓库之外。
- 首个真实验收环境为本机 macOS；Unix 进程控制实现必须显式标记平台边界，不能声称 Windows 已支持。
- 四个 Agent 都执行真实 Codex。离线测试只测试状态转移、数据库、Git、文件、协议解析及普通进程控制；不提供模拟 Provider。
- 每个任务的测试先写、确认失败，再实现；每个提交前运行对应测试。只有最后的真实集成门禁能证明协作链路可用。
- 不建立通用 DAG 编辑器、插件框架、消息代理、PTY、后台 daemon 或第二 Provider。
- 仓库内容、模型输出、工具输出都是不可信输入；不能把 Prompt 里的权限文字当作 OS 强制隔离。

## 1. 交付批次与依赖

| 批次 | 任务 | 可检查结果 | 必须停下来检查的门禁 |
|---|---|---|---|
| A：真实接入 | 01–03 | Core/CLI 骨架、命令参数、真实 Codex 协议冒烟 | 真实只读 Codex 调用得到合法 JSON、thread ID、退出码 |
| B：可持久化执行 | 04–07 | 状态事务、Artifact、真实 Worktree、进程归属与取消 | 事务回滚、dirty source 隔离、取消进程树测试通过 |
| C：能力间通信 | 08–11 | 四份 Contract、Context/Handoff、Tester 证据、并发调度 | 真实四角色链路首次跑通，失败不会隐式变成功 |
| D：可恢复交付 | 12–14 | CLI 全命令、恢复、真实验收记录 | 完整流程、崩溃恢复、取消、源工作区不变均有证据 |

关键路径：01 → 02 → 03 → 04 → 05 → 06 → 07 → 08 → 09 → 10 → 11 → 12 → 13 → 14。
不要为了等待第二个 Provider 或 UI 而推迟任何一项门禁。

## 2. 文件布局与职责

```text
Cargo.toml                         # workspace 成员与共享依赖
Cargo.lock                         # 可复现依赖
rust-toolchain.toml                # 实施时锁定已验证工具链
crates/
  archmage-core/
    Cargo.toml
    src/
      lib.rs                       # 对外 API
      error.rs                     # 类型化错误，无 UI 格式化
      config.rs                    # 数据路径、限额、Provider 配置
      domain.rs                    # ID、状态、attempt、WorkspaceAccess
      event.rs                     # 版本化事件 envelope
      store/mod.rs                 # Store 生命周期、迁移、只读查询
      store/transitions.rs         # 状态 CAS、事件、attempt 原子提交
      store/recovery.rs            # 非终态与归属查询
      artifact.rs                  # 原子发布、hash、大小、路径校验
      workspace/mod.rs             # canonical repo、baseline、provision/reconcile
      workspace/git.rs             # Git argv 调用、状态与完整差异
      workspace/snapshot.rs        # tracked/untracked 文件清单与内容摘要
      process.rs                   # 进程组、超时、双流读取、退出确认
      provider/mod.rs              # 中立 invocation/event/outcome 类型
      provider/codex.rs            # Codex argv、stdin、兼容性检查
      provider/codex_event.rs      # JSONL 解析与中立事件投影
      capability.rs               # 四个内置版本化能力
      contract.rs                 # Schema 和业务不变量
      context.rs                  # typed refs、snapshot、prompt、handoff
      check_runner.rs             # 固定 check ID 到命令、durable 证据
      scheduler.rs                # Ready Set、并发配额、读写兼容
      runtime/mod.rs              # 应用服务、生命周期编排
      runtime/dispatch.rs         # attempt 创建、运行、结果提交
      runtime/control.rs          # cancel 请求与 owner 控制
      runtime/recovery.rs         # owner/process/worktree 对账
      report.rs                   # 运行报告、结论和 Artifact 导航
    migrations/0001_initial.sql
    capabilities/v1/
      designer.md
      coder.md
      tester.md
      reviewer.md
      designer.schema.json
      coder.schema.json
      tester.schema.json
      reviewer.schema.json
    tests/
      domain.rs
      config.rs
      codex_protocol.rs
      codex_live.rs
      persistence.rs
      artifacts.rs
      workspace.rs
      processes.rs
      contracts.rs
      context.rs
      checks.rs
      scheduler.rs
      recovery.rs
      support/mod.rs              # 真实临时 repo/DB，不包含假 Provider
      fixtures/codex-events.jsonl
  archmage-cli/
    Cargo.toml
    src/main.rs
    src/args.rs
    src/output.rs
    tests/cli.rs
    tests/acceptance_live.rs
    tests/fixtures/health-repo/
      health.py
      test_health.py
docs/runbooks/s0-acceptance.md       # 实施后写入验证过的命令
docs/acceptance/s0/                 # 真实门禁记录，不包含凭据或原始敏感日志
```

初期保持两个 crate；模块增长再按职责拆文件，不先拆出十几个小 crate。Core 不依赖 Clap、Tauri 或前端。

## 3. 共同实现契约

### 3.1 身份、状态与结果

所有 ID 由 Runtime 生成，不接受模型指定本地路径或数据库 ID。Run、NodeRun、attempt、Provider session 分开存储。

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceAccess { None, Read, Write }

pub fn compatible(a: WorkspaceAccess, b: WorkspaceAccess) -> bool {
    use WorkspaceAccess::*;
    matches!((a, b), (None, _) | (_, None) | (Read, Read))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptState { Starting, Running, Succeeded, Failed, Interrupted, Cancelled }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionOutcome { Approved, ChangesRequested }

pub fn completion_exit_code(outcome: CompletionOutcome) -> i32 {
    match outcome {
        CompletionOutcome::Approved => 0,
        CompletionOutcome::ChangesRequested => 10,
    }
}
```

NodeRun 是逻辑节点，attempt 是不可覆盖的执行历史。节点 `pending → ready → running → succeeded/failed/interrupted/cancelled`；重试增加 attempt 而不把旧 attempt 改回 running。Run 使用 `created/provisioning/running/interrupted/completed/failed/cancelled`，completion outcome 单独存储。

Tester 合法返回 `failed` 代表“测试执行完但检查未通过”：Node 仍可成功完成 Contract，由 Reviewer 获得失败证据。缺证据/缺必跑项才是 `contract_invalid`。所有 required checks 未通过时，Reviewer `approved` 必须被业务校验拒绝，不能返回 CLI 0。

### 3.2 Provider 与调用记录

Invocation 包含 attempt ID、绝对 worktree、stdin prompt bytes、schema path、原始结果路径、sandbox、deadline、显式 model override（可无）、Provider 配置摘要。禁止字符串拼接 shell 命令；使用 argv 调用 Codex/Git。

Provider 输出中立的 Started、Item、Usage、Failed、Exited 事件；完整原始 JSONL 另外保留。最终接受必须同时满足进程退出成功、无 turn.failed/error 终态、结果 JSON 校验成功、业务不变量成功、durable Artifact 注册成功。

只验证本机 CLI 的已观察能力，不承诺版本号范围。启动前检查必要 flags；记录 `codex --version` 和配置摘要。模型默认使用用户已配置模型；不擅自更换型号。

### 3.3 持久化与边界

- Tables：projects、providers、runs、capability_snapshots、node_runs、attempts、workspaces、contexts、artifacts、handoffs、events、check_runs、control_requests、schema_migrations。
- Unique：`(run_id,node_key)`、`(node_run_id,attempt_number)`、`(run_id,event_seq)`、每 NodeRun 一个 active attempt。
- 所有可变记录有 revision；状态更新用 expected revision/state CAS。CAS 失败不发事件。
- 同一事务：attempt 终态 + NodeRun 状态 + validated output refs + Handoff 元数据 + Event；只在 commit 后唤醒订阅者。
- 每 Run 一个 OS owner lock（调度器所有权，不是 Node 串行锁）；数据库 owner generation 隔离旧调度器写入。`status` 不能把别人的 active run 标为 interrupted。
- SQLite WAL、foreign_keys=ON、synchronous=FULL、busy_timeout=5000ms；迁移在显式事务中完成。DB 忙超时归类 storage_failed，不吞错。
- 文件先 write/flush/sync、rename、sync parent，再注册元数据。无引用孤儿只报告，不在 S0 自动删除。
- 只有 Runtime 可以发布 canonical result/context/evidence；模型不可以写整个数据根。Agent 可写内容先落在 attempt 临时接收位置，由 Runtime 导入、校验、hash 后发布。

### 3.4 默认资源与失败策略

默认 `max_parallelism=2`、单 Provider 并发 2、每节点 30 分钟 deadline、取消宽限 5 秒、每 check 10 分钟。策略写入 Run snapshot；CLI 可覆盖节点 deadline 和并发，required check 的超时随 policy 冻结。

初始上限：单 JSONL 行 8 MiB、final JSON 8 MiB、每流日志 128 MiB、单 Context 渲染输入 16 MiB。超限停止 attempt 并报告具体资源及上限，不静默截断 Contract 或 Context；用小输入验收，不做 token 自动压缩。

失败 attempt 不自动重试。`resume` 只重试明确可恢复的失败（超时、瞬时 Provider 错误、有效配置下的中断）；权限、身份不确定、Contract/Workspace 不变量失败要求先诊断，不能循环消耗账号额度。

## Task 01：工程骨架与最小 CLI

**Create:** `Cargo.toml`、`rust-toolchain.toml`、两个 crate 的 `Cargo.toml`、`archmage-core/src/{lib,error,config,domain}.rs`、`archmage-cli/src/{main,args,output}.rs`。
**Test:** `crates/archmage-core/tests/domain.rs`、`crates/archmage-cli/tests/cli.rs`。

- [ ] 初始化两成员 Cargo workspace，edition 2024，锁定实施时已安装且验证的 Rust 工具链。加入 `.gitignore` 的 `/target/`；保留原 `.superpowers/`。
- [ ] 将 §3.1 的 WorkspaceAccess/兼容函数、CompletionOutcome/退出码函数作为第一组 API；先写下列测试。

```rust
use archmage_core::domain::{compatible, completion_exit_code, CompletionOutcome, WorkspaceAccess};

#[test]
fn readers_share_but_writer_is_exclusive() {
    use WorkspaceAccess::*;
    let modes = [None, Read, Write];
    for a in modes {
        for b in modes {
            let expected = a == None || b == None || (a == Read && b == Read);
            assert_eq!(compatible(a, b), expected);
        }
    }
    assert_eq!(completion_exit_code(CompletionOutcome::Approved), 0);
    assert_eq!(completion_exit_code(CompletionOutcome::ChangesRequested), 10);
}
```

- [ ] 运行 `cargo test -p archmage-core --test domain`，确认 API 未实现时失败，然后加入 §3.1 实现并重跑通过。
- [ ] 用 Clap 定义 `doctor/run/status/logs/resume/cancel` 参数，required repo/goal/check 缺失时由 parser 拒绝；尚未接线的子命令明确返回未实现错误，不返回模拟成功。
- [ ] 运行 `cargo run -p archmage-cli -- --help`，检查六个命令；`cargo run -p archmage-cli -- run` 必须非零且不创建数据目录。
- [ ] `cargo fmt --all --check` 与 `cargo test --workspace` 通过，提交 `chore: bootstrap core and CLI workspace`。

## Task 02：配置、数据根与本地诊断

**Create:** `crates/archmage-core/src/provider/mod.rs`、`crates/archmage-core/src/provider/codex.rs`。
**Modify:** `config.rs`、`args.rs`。
**Create/Test:** `tests/config.rs`、`tests/codex_protocol.rs`。

- [ ] 先写 table tests：默认平台应用目录；`ARCHMAGE_HOME` 显式路径；绝对路径标准化；不可写目录；root symlink 指向源仓库时拒绝把产品数据置入 source。
- [ ] 定义配置快照：data root、Codex/Git binary、provider version、sandbox policy、显式模型覆盖、§3.4 限额。不记录完整环境变量或 auth 内容。
- [ ] 执行 `cargo test -p archmage-core --test config` 确认失败，然后实现路径解析和受限权限的目录创建（Unix 0700，敏感日志 0600）。
- [ ] 实现只读 `--version/exec --help` 检查；flags 缺失列出具体名称，不能继续启动 Run。Git 验证通过 `git --version`。
- [ ] 将认证探针设计为有明确提示、deadline 的真实只读 Codex 调用；`doctor --offline` 仅做本地检查并输出“认证未验证”，不得与完整 doctor 混淆。
- [ ] `cargo test -p archmage-core --test config --test codex_protocol` 通过，提交 `feat: validate local execution configuration`。

## Task 03：尽早打通一次真实 Codex 调用

**Create:** `provider/codex_event.rs`、`process.rs`、`tests/codex_live.rs`、`tests/fixtures/codex-events.jsonl`。
**Modify:** `provider/{mod,codex}.rs`。

- [ ] Parser 测试先覆盖 thread.started、turn.completed、turn.failed、error、未知事件、UTF-8 跨 read 分片、无结尾换行、超长行。无 session ID 的普通事件不能自行产生 session ID。
- [ ] `cargo test -p archmage-core --test codex_protocol` 确认失败，实现逐行缓冲和中立投影；未知事件保留 raw，不当作成功终态。
- [ ] 实现 `Command::new(binary)` + argv + piped stdin/stdout/stderr；并发 drain 两个流、关闭 stdin、等待子进程，避免 stderr 满导致 deadlock。
- [ ] 在 `codex_live.rs` 增加 `#[ignore = "requires real authenticated Codex"]` 测试：tempdir 内初始化真实 Git repo，read-only 调用要求返回以下 schema，验证 `ok==true`、进程 0、thread ID 非空。Prompt 使用 `Return the JSON object {"ok":true}. Do not edit files or run commands.`；即使 Prompt 禁止动作也仍启用 read-only sandbox。

```json
{"type":"object","properties":{"ok":{"type":"boolean"}},"required":["ok"],"additionalProperties":false}
```

- [ ] 运行 `cargo test -p archmage-core --test codex_live real_readonly_probe -- --ignored --nocapture`。这是实际使用账号/网络的门禁；认证失败或无法访问模型就停在 A 门禁，不能替换 Fake Provider。
- [ ] 把真实观测事件脱敏后保存为 parser fixture，检查它不含 prompt、源码、路径隐私或凭据。
- [ ] 记录 version、argv（无 secret）、退出码、thread ID、schema 结果。提交 `feat: invoke real Codex non-interactively`。

## Task 04：数据库、attempt 与原子事件

**Create:** `store/{mod,transitions,recovery}.rs`、`event.rs`、`migrations/0001_initial.sql`、`tests/{persistence,support/mod}.rs`。
**Modify:** `domain.rs`。

- [ ] 先编写真实临时 SQLite 测试：两次 open 不重复 migration；FK 生效；状态更新后注入事务错误，revision 与 event count 均不变；重复 completion 不产生重复事件；两个连接竞争 attempt 仅一个成功。
- [ ] `cargo test -p archmage-core --test persistence` 确认缺少 store 实现而失败。
- [ ] 按 §3.3 建表；定义 IDs/newtypes、序列化状态与 typed event payload；model/provider 名称不可进入调度器分支。
- [ ] 用下列事务顺序实现所有状态变更，不将文件写入置于 SQLite write transaction 内。

```sql
BEGIN IMMEDIATE;
-- 先 UPDATE ... WHERE id=? AND revision=? AND state=?。
-- affected_rows 必须恰好为 1，否则回滚。
-- 同事务更新关联 attempt/node/run 与 validated artifact refs。
-- 分配 Run 内下一个 event_seq，插入唯一 Event。
COMMIT;
-- COMMIT 成功之后才通知进程内 subscribers。
```

- [ ] 建立 Handoff 行，字段为 producer attempt/artifact、consumer node/input key、content hash；consumer attempt 启动时绑定到具体 snapshot。
- [ ] 重新打开 DB 验证完整状态与有序事件；`cargo test -p archmage-core --test persistence` 通过，提交 `feat: persist attempts and events atomically`。

## Task 05：Artifact 发布与不可变输入

**Create:** `artifact.rs`、`tests/artifacts.rs`。
**Modify:** `store/mod.rs`。

- [ ] 测试：`../` 与绝对模型路径拒绝；symlink 逃逸拒绝；hash/length 不符拒绝；rename 前故障无 DB 引用；rename 后 DB 失败只有 orphan；缺失已登记文件阻止 Context resolution。
- [ ] `cargo test -p archmage-core --test artifacts` 确认失败，实现 Runtime 分配路径、同目录临时写、sync、rename、parent sync、SHA-256 与元数据登记。
- [ ] 区分 raw diagnostics 和 validated outputs；未完成 attempt 的 raw 文件不能被 ContextRef 当作业务输入。
- [ ] model result 使用 attempt-owned staging file；核心数据库/已发布 artifacts 不加入 Codex writable roots。只由 parent Runtime 提交最终结果。
- [ ] 原始日志可能含仓库敏感内容：本地限制权限，不上传/加入 Git；派生诊断做脱敏，但不承诺正则能清除任意第三方泄漏。
- [ ] `cargo test -p archmage-core --test artifacts --test persistence` 通过，提交 `feat: publish immutable local artifacts`。

## Task 06：真实 Git Worktree 与完整差异

**Create:** `workspace/{mod,git,snapshot}.rs`、`tests/workspace.rs`。
**Modify:** `tests/support/mod.rs`。

- [ ] 临时真实仓库测试：clean/dirty source、带空格路径、unborn HEAD、branch/path collision、两个 Run、tracked/staged/untracked/binary 文件、partial provisioning recovery。
- [ ] `cargo test -p archmage-core --test workspace` 确认失败；实现 `rev-parse --show-toplevel`、`rev-parse --verify HEAD`、`status --porcelain=v1 -z`、`worktree list --porcelain` 的 argv 调用。
- [ ] 先登记 provisioning intent，再执行 `git worktree add -b archmage/run-<id> <absolute-path> <baseline>`；成功后登记 workspace.created。重启根据 path/branch/common-dir/baseline 对账，不删除碰撞资源。
- [ ] 不执行 `git add -N` 修改索引来收集新文件。tracked diff 使用 baseline diff，untracked 文件单独以路径/类型/hash/内容 Artifact 保存，组合为 Reviewer 的 ChangeSet。
- [ ] `SnapshotId` 来自 canonical manifest：baseline、HEAD、tracked staged/unstaged 内容摘要、non-ignored untracked 内容；Git status 文字本身不足以证明内容相同。symlink 记录 link target，不越界遍历。
- [ ] 比较 Source 前后 HEAD/status/content manifest/remotes 不变；worktree 注册和新分支会改变共享 Git 元数据，这属于预期，不宣称整个 `.git` 字节不变。
- [ ] `cargo test -p archmage-core --test workspace` 通过，提交 `feat: isolate runs in real Git worktrees`。

## Task 07：进程归属、并发租约与取消基础

**Modify:** `process.rs`、`store/transitions.rs`。
**Create/Test:** `runtime/control.rs`、`tests/processes.rs`。

- [ ] 先用普通 OS 命令测试 stdout/stderr 大流量、timeout、TERM 后退出、忽略 TERM 后 KILL、child tree 清理。它们不是 Provider，也不生成 Agent 结果。
- [ ] `cargo test -p archmage-core --test processes` 确认失败，实现每 attempt 独立 process group，记录 PID/PGID/进程启动标识/owner generation。
- [ ] spawn 前持久化 Starting intent；spawn 后登记身份并转 Running。补测 spawn 后、身份提交前的崩溃窗口；不允许只凭旧 PID 发送信号。
- [ ] 每 Run 取 OS owner lock；write/read lease 由唯一 owner 管理，并用 attempt generation fencing 拒绝旧完成结果。OS owner lock 不限制多个 Node 同时运行。
- [ ] 取消按 TERM → bounded wait → KILL → exit confirmation 执行；未确认退出前不得释放 Workspace writer lease 或派发替代 writer。
- [ ] 进程身份不可确认时阻止写入并给出诊断；不能把“查不到进程信息”当作“进程已死”。
- [ ] `cargo test -p archmage-core --test processes --test persistence` 通过，提交 `feat: own and cancel execution processes safely`。

## Task 08：四个 Capability Contract 与 Prompt

**Create:** `capability.rs`、`contract.rs`、`capabilities/v1/` 的四份 Markdown 与四份 JSON Schema、`tests/contracts.rs`。

- [ ] 先按下面固定 shape 写 accepted/rejected JSON cases，所有 schema 根对象拒绝 additionalProperties，required 字段齐全；enum 不能由任意字符串替代。

| 能力 | 固定字段（全部 required） | 业务验证 |
|---|---|---|
| Designer | schema_version=`1`; summary_markdown:string; design_markdown:string; acceptance_criteria:string[]; implementation_steps:string[] | design 和 criteria 非空；只读 Workspace 不变 |
| Coder | schema_version=`1`; summary_markdown:string; status=`implemented`/`blocked`; changed_paths:string[]; notes:string[] | blocked 停止流程；实际 ChangeSet 由 Runtime 决定，不能只信 changed_paths |
| Tester | schema_version=`1`; summary_markdown:string; status=`passed`/`failed`; checks:array of `{check_id:string,evidence_id:string,status:passed/failed}` | required check 全部匹配本 attempt canonical evidence；不能漏跑或引用其他 attempt |
| Reviewer | schema_version=`1`; summary_markdown:string; status=`approved`/`changes_requested`; issues:array of `{severity:P0/P1/P2/P3,path:string,message:string}` | 测试 failed 时不能 approved；路径不得越界；批准和执行成功分开 |

- [ ] 每种结果测试：缺字段、错误版本、额外字段、malformed JSON、错误 enum、空 mandatory design、伪造 evidence ref。
- [ ] `cargo test -p archmage-core --test contracts` 确认失败，实现 schema loading 与业务 validators；无效 schema 在 preflight 失败，不启动 active Run。
- [ ] 为每角色写完整 Prompt：角色职责、可用输入、可执行动作、禁止 commit/merge/push、如何报告 blocked、输出字段。Tester 必须调用 named check helper；Reviewer 不修复代码。
- [ ] Prompt 不规定先前 session ID、不调用 `resume --last`、不隐藏失败测试。四能力均绑定 Codex 的独立 Invocation。
- [ ] `cargo test -p archmage-core --test contracts` 通过，提交 `feat: define four capability contracts`。

## Task 09：Context Snapshot 与 Handoff

**Create:** `context.rs`、`tests/context.rs`。
**Modify:** `artifact.rs`、`store/transitions.rs`。

- [ ] 类型化 ContextRef 只允许 WorkflowInput、CommittedNodeOutput、Artifact、WorkspaceSnapshot；不实现任意模板脚本或 SQL/路径表达式。
- [ ] 测试：相同输入不同 map 插入顺序得到相同 hash；missing predecessor 阻止调度；上游失败/未提交 artifact 不可引用；manifest tamper 被检测；新 Prompt 版本改变 snapshot hash。
- [ ] `cargo test -p archmage-core --test context` 确认失败，实现 canonical JSON（递归 key 排序）+ SHA-256；渲染并保存实际发送的 stdin bytes。
- [ ] 固定 bindings：Designer=goal+repo view；Coder=goal+design；Tester=goal+design+implementation+check policy+workspace manifest；Reviewer=goal+design+implementation+ChangeSet+TestResult。
- [ ] 同一 read lease 内生成 manifest 与必要文件视图，再启动读者；不把“只读”误解释成“没有仓库访问”。Codex 能读取的 AGENTS.md/配置/工具访问需要单独记入执行 provenance。
- [ ] Context Snapshot 表示 Archmage 提供的输入，不宣称完整重放模型隐式系统 prompt 或确定性输出。受管理工作区外的人为编辑只能检测，不能用调度租约阻止。
- [ ] `cargo test -p archmage-core --test context --test artifacts` 通过，提交 `feat: resolve explicit capability handoffs`。

## Task 10：Tester 发起的 Check Runner 与证据

**Create:** `check_runner.rs`、`tests/checks.rs`。
**Modify:** `args.rs`、`main.rs`、`tester.md`、`store/transitions.rs`、`process.rs`。

- [ ] 内部子命令 `archmage internal check --endpoint <socket> --attempt <id> --check <id>` 仅向活跃 Runtime 请求已冻结的 check ID，不能接受模型传入任意命令或数据目录。
- [ ] Runtime 负责 check 的真正启动和输出落盘。Tester 仍决定调用、理解证据并形成 TestResult；Runtime 不替代 Tester 生成能力结论。
- [ ] helper 与 parent 使用每 attempt 短生命周期 Unix socket；验证 attempt 活跃、owner generation、请求归属和固定 check ID。证明 Codex 当前 sandbox 能连接此 socket；否则停止该任务报告兼容性，不开启 full access。
- [ ] `--check` 是用户显式授权的 shell command。只此入口使用 `/bin/sh -c`，cwd 固定为 worktree；stdin 关闭；环境 allowlist，排除 provider auth；日志显示 shell/command/cwd/timeout。
- [ ] parent 运行 check 也必须受支持的 workspace sandbox 限制，不能把 helper 做成 Codex sandbox 绕过通道。实现时使用 Provider 实际支持的 sandbox launcher 并以真实 read/write escape probe 验证；不支持就明确阻塞 check 执行。
- [ ] 测试矩阵：exit 0、exit 7、timeout、unknown ID、过期 attempt、重复请求、另一 attempt evidence、漏跑项、source 改动、必跑失败却 passed。普通 check shell 进程不是 Agent 模拟器。
- [ ] `cargo test -p archmage-core --test checks` 红绿通过。证据包含执行 command、exit/signal、start/end/duration、stdout/stderr hashes、check ID、attempt、workspace before/after manifest。
- [ ] check 非零也返回 evidence 给 Tester；transport failure/证据丢失才使能力失败。模型不能写 canonical evidence，重复请求返回已完成 evidence 或明确 in-progress，不双重执行。
- [ ] 真实 Codex Tester 运行一个成功 check 和一个失败 check，验证 TestResult=`failed`。提交 `feat: capture tester-invoked validation evidence`。

实现前的定向兼容性核对：读取已安装 CLI 的 `codex sandbox --help` 及本平台子命令帮助，确认支持的 argv、网络策略、工作目录与环境传递。使用一个 disposable repo 验证允许在 run worktree 创建 build output、禁止写 sibling source、socket helper 可用且无额外 shell 权限。这里验证的是支持能力，不预填未经观测的 CLI flag。官方资料只作为本机帮助不足时的补充。任何一项失败都属于 C 批次工程阻塞，不能降低边界后标为通过。

## Task 11：Ready Set 调度与完整协作主链

**Create:** `scheduler.rs`、`runtime/{mod,dispatch}.rs`、`tests/scheduler.rs`。
**Modify:** `store/transitions.rs`、`provider/mod.rs`、`lib.rs`。

- [ ] 纯调度测试使用输入状态数据，不执行 Fake Provider。矩阵：两个 readers 同时 ready；read/write 冲突；none/write 共存；配额耗尽；前驱失败；两父节点仅一个成功；同 node 双重 dispatch。
- [ ] `cargo test -p archmage-core --test scheduler` 确认失败，实现 Ready Set 计算和稳定 node 顺序；逐项 grant 后更新资源视图，避免同一批次错误发出两个 writer。
- [ ] Runtime service 定义 start/status/logs/resume/cancel；固定 definition 四节点，版本写入 Run snapshot。
- [ ] 每 dispatch 依次：CAS ready → create attempt/lease → resolve context → publish immutable input → spawn Codex → drain events → validate output/workspace/evidence → commit outputs/Handoff/state/Event → release lease。
- [ ] 完成提交必须检查 owner generation、attempt generation 和 cancel request；旧结果不能覆盖新 attempt。任何失败都停止新节点派发并清理所属进程，不自动 Fix。
- [ ] Reviewer 的 valid changes_requested 产生 completed+ChangesRequested；failed checks 不能得到 Approved。Runtime 单独记录 execution failure，不混为业务审查结论。
- [ ] 真实完整 Workflow 先从测试 API 调用，四个 session IDs 不同；结束后查 Handoff 三条主边与 Reviewer 的多输入 references。
- [ ] `cargo test -p archmage-core --test scheduler --test contracts --test persistence` 通过，提交 `feat: orchestrate capability agents with ready-set scheduling`。

## Task 12：重启对账、重试与部分修改

**Create:** `runtime/recovery.rs`、`tests/recovery.rs`。
**Modify:** `store/recovery.rs`、`runtime/control.rs`、`workspace/snapshot.rs`。

- [ ] 测试断点：创建 Run 后；worktree 建好但未登记；attempt Starting 后；spawn 前后；产出 result 后但 DB commit 前；DB commit 后但通知前。用 store/process 实际操作注入故障，不伪造 Agent 会话成功。
- [ ] `status` 只读；只有取得 owner lock 的 resume/control 路径可以执行 reconciliation。另一个正常运行的 owner 不能被扫描误判。
- [ ] 确认旧 Codex/check 进程已退出或安全终止后，再把旧 attempt 记 interrupted 并释放 writer lease；不以 PID 存活检查代替归属验证。
- [ ] 同一 successful prefix 保留；pending downstream 尚未开始可重新生成 workspace refs；重试新 attempt 必须有新结果文件路径，旧 result 不能通过文件存在检查被接受。
- [ ] 中断 writer 的部分修改保留。新 attempt 引用原始 goal/design 等不可变输入，同时创建新的 recovery ContextSnapshot，明确包含当前 Workspace manifest、相对原 snapshot 的 delta、旧 attempt ID 与“检查已有修改后继续”的指令。
- [ ] 不自动 reset/stash/checkout，也不宣称重跑相同 prompt 等价于无副作用执行。HEAD/branch 身份变化阻止恢复；源未知写入者/进程身份不清时拒绝继续写入。
- [ ] Tester 重跑使用新 attempt 证据；之前成功 check 的证据不跨 attempt 冒用。old attempt diagnostics 保留但不能作为新 Node output。
- [ ] 两个 `resume` 同时发起只允许一个 owner；cancel 与 completion race 用事务顺序裁决，取消不能因迟到结果回到成功。
- [ ] `cargo test -p archmage-core --test recovery --test processes --test workspace` 通过，提交 `feat: reconcile interrupted runs without duplicating writers`。

## Task 13：CLI 全命令、Timeline 与报告

**Create:** `report.rs`。
**Modify:** CLI `src/{main,args,output}.rs`、`tests/cli.rs`、`lib.rs`。

- [ ] CLI 测试：required args；`status <missing>` 非零；`status --json` 无日志杂质；logs sequence 单调；invalid flags 不创建 Run；terminal resume no-op 不调用 Codex；completed/cancelled exit mapping。
- [ ] `cargo test -p archmage-cli --test cli` 确认失败，将六个命令连接 runtime services，不在 CLI 写 SQL/调度分支。
- [ ] `logs --follow` 以 SQLite cursor 查询为权威，通知仅用来唤醒；丢失 in-process notification 不能丢事件，末次 cursor 后补齐再退出。
- [ ] `cancel` 用 durable control_request 唤醒/被 owner 轮询，活动 owner 执行终止；无 owner 时执行安全 reconciliation。明确输出“已请求”与“已终止”的区别。
- [ ] `run` 前台 Timeline；Ctrl-C 创建 cancel request 并等待 bounded cleanup；非交互模式不弹未建模的审批输入。
- [ ] final report 输出 Run ID、lifecycle、review outcome、各 attempt、worktree/branch、Context/Handoff/artifact links、check 证据、source warning、未执行项；Artifact 路径由 Runtime 产生。
- [ ] `cargo test --workspace`、`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings` 通过，提交 `feat: expose observable workflow control through CLI`。

## Task 14：真实 Codex 端到端门禁与交付

**Create:** `crates/archmage-cli/tests/acceptance_live.rs`、`tests/fixtures/health-repo/{health,test_health}.py`、`docs/acceptance/s0/README.md`。
**Modify:** `docs/runbooks/s0-acceptance.md`、`docs/runbooks/codex-provider.md`、`docs/README.md`、`docs/roadmap/stages/s0-real-codex-slice.md`。

- [ ] 用 Python 标准库的无网络 fixture 初始化临时真实 Git 仓库。fixture 初始只实现 `/`；目标为新增 `/health`，未知路径返回 404。初始文件如下。

```python
# health.py
def response(path):
    if path == "/":
        return 200, {"service": "fixture"}
    return 404, {"error": "not found"}
```

```python
# test_health.py
import unittest
from health import response

class ResponseTests(unittest.TestCase):
    def test_root(self):
        self.assertEqual(response("/")[0], 200)

    def test_unknown(self):
        self.assertEqual(response("/missing")[0], 404)

if __name__ == "__main__":
    unittest.main()
```

- [ ] 验收 harness 复制 fixture、init/config local git identity/commit，记录 source baseline；为每个场景使用独立 `ARCHMAGE_HOME`，不要让命令写入个人产品数据或本项目源码。
- [ ] Scenario `complete`：真实 `archmage run`，goal 要求 `/health → (200,{"status":"ok"})` 并添加测试，`--check "python3 -m unittest -v"`。验证四个不同 thread IDs、全部 contracts、checks、三次主链 Handoff；独立 oracle 直接 import run-worktree 的 `health.response` 验证目标，不能只信模型自写测试。
- [ ] Scenario `interrupt_resume`：check 使用 gate 文件在 ready 时等待 release，而非固定 sleep 猜测时机。等待 durable check.started 后杀 parent Runtime，确认实际 orphan 处理，再 release gate 并 resume；上游 session ID 不变，Tester attempt/session 更新，旧输出不复用。
- [ ] Scenario `cancel`：运行中从第二 CLI 请求 cancel，验证所有受管进程退出、状态 cancelled、没有迟到 success、Worktree 保留。
- [ ] Scenario `dirty_source`：源工作区改 tracked 文件并新增 untracked 文件；Run 仍从 HEAD 创建，warning 明确且源内容保留。
- [ ] Scenario `failed_check`：添加 required check `exit 7`；真实 Tester 记录 failed，Reviewer 不能让最终 CLI 0。允许 valid changes_requested（10）或因错误 approved 被 Contract 拒绝（21），报告必须区分两者。
- [ ] 为并发增加 opt-in 真实 Provider integration test：两个独立 read-only Codex Invocation 受 scheduler 配额启动，events 中运行区间重叠；不能仅用纯兼容矩阵宣称并发调度已集成。生产 workflow 仍是固定线性，不增加用户 DAG 功能。
- [ ] 执行以下发布门禁；live tests 不自动包含在普通 cargo test，不得把 ignored 当 passed。

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p archmage-core --test codex_live -- --ignored --nocapture
cargo test -p archmage-cli --test acceptance_live -- --ignored --nocapture --test-threads=1
```

- [ ] 报告写入 `docs/acceptance/s0/README.md`：实际 commit、OS/Rust/Git/Codex 版本、场景结果、Run IDs、脱敏 evidence 摘要、自动化测试结果、限制。只有真实数据，不提前生成“通过”结果。
- [ ] 每个 live test 有总 deadline；账号/网络/沙箱受限记为 blocked，不增加权限绕过、Fake Agent 或无限重试。
- [ ] 更新 stage 实施状态及 runbook 命令；检查文档链接和 `git diff --check`，提交 `test: verify real Codex collaboration and recovery`。

## 4. Spec 覆盖与最终审查

| Spec 要求 | 实现任务 | 验证 |
|---|---|---|
| Core 与 CLI 解耦 | 01、11、13 | Core 无 Clap/Tauri；CLI 仅调用服务 |
| 真实本地 Codex | 02、03、07、14 | read-only probe、四 session、无 Fake 路径 |
| 能力为协作主体 | 08、09、11 | 版本化 role/contract、独立会话、多输入 Handoff |
| 状态与 Event 原子提交 | 04、11 | rollback、CAS、restart 后事件有序 |
| Artifact 与 Context 可追踪 | 05、09 | hash、orphan、missing/tampered ref 拒绝 |
| Git Worktree 隔离 | 06、14 | dirty source、双 Run、完整 diff、新文件 |
| 并发 Ready Set | 07、11、14 | 兼容矩阵、同批 writer 冲突、真实 reader 并发 |
| Tester 的真实证据 | 10、14 | required checks、parent-owned evidence、failed_check |
| Contract 与业务结论 | 08、11、13 | failed checks 不变 approval；exit 0/10/20/21/22 |
| 取消、超时和恢复 | 07、12、14 | orphan、owner fencing、partial writer、cancel race |
| 不 commit/merge/push/delete | 06、08、12、14 | HEAD invariant、source 前后、保留 Workspace |
| CLI 和本地观察 | 13、14 | status JSON、follow cursor、最终报告 |

最后复查不得遗漏：未完成输出不得当成功；仅 PID 不足以归属验证；部分代码不是半成品 JSON；测试失败不是 transport failure；Worktree 不是恶意代码安全沙箱；本地 shell/MCP/hooks 可形成旁路，需要受限配置与真实边界验证。

## 5. 执行交接

先执行 A 批次，以真实只读 Codex 接入为首个检查点。通过后按 B/C/D 推进，每批报告已通过测试、未执行 live tests 和真实阻塞，不用构建成功代替协作验收。

可以采用当前会话分批执行（`executing-plans`），也可以在用户授权后采用子 Agent 分任务实现与审查（`subagent-driven-development`）。不自动创建新 Codex task，不自动 push，不提前启动 S1。
