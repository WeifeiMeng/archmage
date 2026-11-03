[简体中文](../../../zh-CN/superpowers/plans/2026-09-16-s0-real-codex-vertical-slice.md) | English

# S0 Real Codex Vertical Slice Implementation Plan


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended when delegation is authorized) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Through a local Codex Provider, the four Capability Agents of Designer, Coder, Tester, and Reviewer can complete traceable, interruptible and recoveryable collaboration in the real Git Worktree.

**Architecture:** Two Rust crates: `archmage-core` is responsible for domain, storage, Workspace, Provider, Context, Contract and scheduling; `archmage-cli` is only responsible for parameters, interactive output and exit codes. Each attempt calls the real `codex exec`; concurrency with Runs is controlled by dependencies and Workspace access leases, no Fake Agent is used.

**Tech Stack:** Rust, Tokio, Serde/JSON, SQLite (rusqlite), Clap, JSON Schema, Git CLI, Codex CLI.Depends on locking Cargo.lock in Task 01. The specific API is subject to the lock version.

---

## 0. Execute the agreement

- The current document is a plan to be executed, which does not mean that the following source code, commands or tests already exist.
- Baseline: `07f8589`; Approval basis: [S0 Spec](../specs/2026-09-16-real-codex-vertical-slice-design.md). Recheck branches, `AGENTS.md` and user uncommitted content before launching implementation.
- This round only writes plans. Implement using an isolated development branch/worktree; do not develop directly on `main`.
- All paths are relative to the warehouse root; product data points outside the warehouse via `ARCHMAGE_HOME`.
- The first real experimental harvest environment is native macOS; Unix process control implementations must explicitly mark platform boundaries and cannot claim to be supported by Windows.
- All four Agents execute the real Codex. Offline testing only tests state transfer, database, Git, files, protocol parsing and ordinary process control; no simulated Provider is provided.
- The tests for each task are written first, failure is confirmed, and then implemented; the corresponding test is run before each submission. Only the final true integrated access proves that the collaborative link is available.
- Does not build a generic DAG editor, plugin framework, message broker, PTY, backend daemon or secondary provider.
- Warehouse content, model output, and tool output are all untrusted inputs; the permission text in the Prompt cannot be regarded as OS forced isolation.

## 1. Delivery batches and dependencies

| Batch | Task | Checkable results | Doors that must be stopped to check |
|---|---|---|---|
| A: Real access | 01–03 | Core/CLI skeleton, command parameters, real Codex protocol smoke | Real read-only Codex call to get legal JSON, thread ID, exit code |
| B: Persistent execution | 04–07 | State transactions, Artifact, real Worktree, process ownership and cancellation | Transaction rollback, dirty source isolation, process tree cancellation test passed |
| C: Inter-capability communication | 08–11 | Four contracts, Context/Handoff, Tester evidence, concurrent scheduling | The real four-role link ran through for the first time, and failure will not implicitly become success |
| D: Recoverable delivery | 12–14 | CLI full command, recovery, real test acceptance record | Complete process, crash recovery, cancellation, source workspace unchanged are all evidenced |

Critical path: 01 → 02 → 03 → 04 → 05 → 06 → 07 → 08 → 09 → 10 → 11 → 12 → 13 → 14.
Don't delay any access control waiting for a second Provider or UI.

## 2. Document layout and responsibilities

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

In the initial stage, two crates will be maintained; as the module grows, the files will be split according to responsibilities instead of splitting out a dozen small crates first. Core does not rely on Clap, Tauri or front-end.

## 3. Jointly realize the contract

### 3.1 Identity, status and results

All IDs are generated by the runtime and do not accept model-specified local paths or database IDs. Run, NodeRun, attempt, and Provider sessions are stored separately.

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

NodeRun is a logical node, and attempt is an execution history that cannot be overwritten. Node `pending → ready → running → succeeded/failed/interrupted/cancelled`;Retry adding attempt without changing old attempt back to running. Run uses `created/provisioning/running/interrupted/completed/failed/cancelled`, and completion outcome is stored separately.

Tester legally returns `failed`, which means "the test was executed but the check failed": Node can still successfully complete the Contract, and the Reviewer obtains the failure evidence. Lack of evidence/lack of must-run items is `contract_invalid`. When all required checks fail, Reviewer `approved` must be rejected by the business verification and cannot return CLI 0.

### 3.2 Provider and call records

Invocation contains attempt ID, absolute worktree, stdin prompt bytes, schema path, original result path, sandbox, deadline, explicit model override (optional), Provider configuration summary. Disable string splicing shell commands; use argv to call Codex/Git.

The Provider outputs neutral Started, Item, Usage, Failed, and Exited events; the complete original JSONL is otherwise retained. The final acceptance must simultaneously meet the requirements of successful process exit, no turn.failed/error final state, successful JSON verification of the result, successful business invariant, and successful registration of durable Artifact.

Only verifies observed capabilities of native CLI, does not commit to version number ranges. Check necessary flags before starting; log `codex --version` and configuration summary. By default, the model uses the model configured by the user; do not change the model without authorization.

### 3.3 Persistence and Boundaries

- Tables：projects、providers、runs、capability_snapshots、node_runs、attempts、workspaces、contexts、artifacts、handoffs、events、check_runs、control_requests、schema_migrations。
- Unique: `(run_id,node_key)`, `(node_run_id,attempt_number)`, `(run_id,event_seq)`, one active attempt per NodeRun.
- All mutable records have revision; status updates are with expected revision/state CAS.No event occurs if CAS fails.
- The same transaction: attempt final state + NodeRun state + validated output refs + Handoff metadata + Event; only wake up subscribers after commit.
- One OS owner lock (scheduler ownership, not Node serial lock) per Run; database owner generation isolates old scheduler writes.`status` You cannot mark someone else's active run as interrupted.
- SQLite WAL, foreign_keys=ON, synchronous=FULL, busy_timeout=5000ms; migration is done in an explicit transaction. DB busy timeout is classified as storage_failed and does not cause errors.
- The file first write/flush/sync, rename, sync parent, and then register metadata. Unreferenced orphans are only reported and are not automatically deleted in S0.
- Only the runtime can publish canonical result/context/evidence; the model cannot write the entire data root. The writable content of Agent first falls into the temporary receiving position of attempt, and is imported, verified, and hashed by Runtime before being released.

### 3.4 Default resources and failure strategies

Default `max_parallelism=2`, single Provider concurrency 2, 30-minute deadline per node, cancellation grace 5 seconds, and 10 minutes per check. The policy is written to Run snapshot; CLI can cover node deadline and concurrency, and the timeout of required check is frozen with the policy.

Initial caps: 8 MiB per JSONL line, 8 MiB for final JSON, 128 MiB per stream log, 16 MiB per Context rendering input. If the limit is exceeded, the attempt will be stopped and the specific resources and upper limit will be reported. Contract or Context will not be cut off silently. Small input acceptance will be used, and token automatic compression will not be performed.

Failed attempts are not automatically retried.`resume` Only retries clearly recoverable failures (timeouts, transient provider errors, interruptions under valid configuration); permissions, identity uncertainties, and Contract/Workspace invariant failures require diagnosis first, and account credits cannot be consumed cyclically.

## Task 01: Project skeleton and minimal CLI

**Create:** `Cargo.toml`, `rust-toolchain.toml`, `Cargo.toml`, `archmage-core/src/{lib,error,config,domain}.rs`, `archmage-cli/src/{main,args,output}.rs` for two crates.
**Test:** `crates/archmage-core/tests/domain.rs`、`crates/archmage-cli/tests/cli.rs`。

- [ ] Initializing the two-member Cargo workspace, edition 2024, locking the installed and verified Rust toolchain at the time of implementation. Added `/target/` to `.gitignore`; retained original `.superpowers/`.
- [ ] Consider the WorkspaceAccess/compatibility functions and CompletionOutcome/exit code functions of §3.1 as the first set of APIs; write the following tests first.

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

- [ ] Run `cargo test -p archmage-core --test domain` to confirm that it fails when the API is not implemented, then add §3.1 to implement it and rerun it to pass.
- [ ] Use Clap to define the `doctor/run/status/logs/resume/cancel` parameter. When required repo/goal/check is missing, it will be rejected by the parser; subcommands that have not yet been wired will explicitly return an unimplemented error and will not return simulation success.
- [ ] Run `cargo run -p archmage-cli -- --help`, checking the six commands; `cargo run -p archmage-cli -- run` must be non-zero and the data directory is not created.
- [ ] `cargo fmt --all --check` and `cargo test --workspace` passed, submitted `chore: bootstrap core and CLI workspace`.

## Task 02: Configuration, data root and local diagnostics

**Create:** `crates/archmage-core/src/provider/mod.rs`、`crates/archmage-core/src/provider/codex.rs`。
**Modify:** `config.rs`、`args.rs`。
**Create/Test:** `tests/config.rs`、`tests/codex_protocol.rs`。

- [ ] First write table tests: default platform application directory; `ARCHMAGE_HOME` explicit path; absolute path standardization; unwritable directory; when the root symlink points to the source warehouse, refuse to put product data into the source.
- [ ] Define configuration snapshot: data root, Codex/Git binary, provider version, sandbox policy, explicit model overrides, §3.4 limits. Does not log full environment variables or auth content.
- [ ] Execute `cargo test -p archmage-core --test config` to acknowledge failure, then implement path resolution and directory creation with restricted permissions (Unix 0700, Sensitive Log 0600).
- [ ] Implement read-only `--version/exec --help` check; flags are missing and specific names are listed, and Run cannot continue to be started. Git verification passed `git --version`.
- [ ] Design the authentication probe as a true read-only Codex call with a clear prompt and deadline; `doctor --offline` only does a local check and outputs "Authentication Not Verified" and should not be confused with the full doctor.
- [ ] `cargo test -p archmage-core --test config --test codex_protocol` Passed, submitted `feat: validate local execution configuration`.

## Task 03: Get through a real Codex call as soon as possible

**Create:** `provider/codex_event.rs`、`process.rs`、`tests/codex_live.rs`、`tests/fixtures/codex-events.jsonl`。
**Modify:** `provider/{mod,codex}.rs`。

- [ ] The Parser test first covers thread.started, turn.completed, turn.failed, error, unknown events, UTF-8 cross-read fragmentation, no ending newline, and super long lines. Ordinary events without a session ID cannot generate a session ID on their own.
- [ ] `cargo test -p archmage-core --test codex_protocol` Acknowledgment fails, line-by-line buffering and neutral projection are implemented; unknown events are retained as raw and are not regarded as successful final states.
- [ ] Implement `Command::new(binary)` + argv + piped stdin/stdout/stderr; concurrently drain two streams, close stdin, and wait for child processes to avoid deadlock caused by full stderr.
- [ ] Add `#[ignore = "requires real authenticated Codex"]` test in `codex_live.rs`: initialize the real Git repo in tempdir, read-only call requires returning the following schema, verify that `ok==true`, process 0, and thread ID are not empty. Prompt uses `Return the JSON object {"ok":true}. Do not edit files or run commands.`; the read-only sandbox is still enabled even if the Prompt disables the action.

```json
{"type":"object","properties":{"ok":{"type":"boolean"}},"required":["ok"],"additionalProperties":false}
```

- [ ] Run `cargo test -p archmage-core --test codex_live real_readonly_probe -- --ignored --nocapture`. This is the access control that actually uses the account/network; if the authentication fails or the model cannot be accessed, it will stop at access control A and cannot replace the Fake Provider.
- [ ] Desensitize the real observation event and save it as a parser fixture, and check that it does not contain prompts, source code, path privacy or credentials.
- [ ] Record version, argv (no secret), exit code, thread ID, schema results. SUBMIT `feat: invoke real Codex non-interactively`.

## Task 04: Database, attempt and atomic events

**Create:** `store/{mod,transitions,recovery}.rs`、`event.rs`、`migrations/0001_initial.sql`、`tests/{persistence,support/mod}.rs`。
**Modify:** `domain.rs`。

- [ ] First write a real temporary SQLite test: open twice without repeating migration; FK takes effect; transaction errors are injected after status update, revision and event count remain unchanged; repeated completion does not generate repeated events; only one of the two connection competition attempts succeeds.
- [ ] `cargo test -p archmage-core --test persistence` Acknowledgment failed due to missing store implementation.
- [ ] Create a table according to §3.3; define IDs/newtypes, serialization status and typed event payload; the name model/provider cannot enter the scheduler branch.
- [ ] All state changes are implemented using the following transaction sequence, without placing file writes within a SQLite write transaction.

```sql
BEGIN IMMEDIATE;
-- 先 UPDATE ... WHERE id=? AND revision=? AND state=?。
-- affected_rows 必须恰好为 1，否则回滚。
-- 同事务更新关联 attempt/node/run 与 validated artifact refs。
-- 分配 Run 内下一个 event_seq，插入唯一 Event。
COMMIT;
-- COMMIT 成功之后才通知进程内 subscribers。
```

- [ ] Create a Handoff row with the fields producer attempt/artifact, consumer node/input key, and content hash; the consumer attempt is bound to a specific snapshot when it starts.
- [ ] Reopen DB to verify complete status and ordered events; `cargo test -p archmage-core --test persistence` passed, submitted `feat: persist attempts and events atomically`.

## Task 05: Artifact publishing and immutable input

**Create:** `artifact.rs`、`tests/artifacts.rs`。
**Modify:** `store/mod.rs`。

- [ ] Tests: `../` and absolute model path rejection; symlink escape rejection; hash/length non-conformance rejection; no DB reference before rename failure; only orphan after rename DB failure; missing registered files prevent Context resolution.
- [ ] `cargo test -p archmage-core --test artifacts` Confirmation failed to implement Runtime allocation path, temporary write to the same directory, sync, rename, parent sync, SHA-256 and metadata registration.
- [ ] Distinguish between raw diagnostics and validated outputs; raw files that have not completed the attempt cannot be used as business input by ContextRef.
- [ ] model result uses attempt-owned staging file; core database/published artifacts do not join Codex writable roots. Only the parent Runtime submits the final result.
- [ ] The original log may contain sensitive content of the warehouse: local restrictions on permissions, no uploading/joining to Git; derived diagnosis for desensitization, but there is no guarantee that regularization can eliminate any third-party leaks.
- [ ] `cargo test -p archmage-core --test artifacts --test persistence` passed, submitted `feat: publish immutable local artifacts`.

## Task 06: Real Git Worktree and full diff

**Create:** `workspace/{mod,git,snapshot}.rs`、`tests/workspace.rs`。
**Modify:** `tests/support/mod.rs`。

- [ ] Temporary real warehouse test: clean/dirty source, path with spaces, unborn HEAD, branch/path collision, two Run, tracked/staged/untracked/binary files, partial provisioning recovery.
- [ ] `cargo test -p archmage-core --test workspace` confirmation failed; implementing argv calls for `rev-parse --show-toplevel`, `rev-parse --verify HEAD`, `status --porcelain=v1 -z`, `worktree list --porcelain`.
- [ ] Register provisioning intent first, then execute `git worktree add -b archmage/run-<id> <absolute-path> <baseline>`; register workspace.created after success. Restart the reconciliation according to path/branch/common-dir/baseline and do not delete the collision resources.
- [ ] Not executing `git add -N` Modify index to collect new files. Tracked diff uses baseline diff, and untracked files are saved separately as path/type/hash/content artifact and combined into Reviewer's ChangeSet.
- [ ] `SnapshotId` from canonical manifest: baseline, HEAD, tracked staged/unstaged content summary, non-ignored untracked content; Git status text itself is not enough to prove that the content is the same. Symlink records the link target and does not traverse out of bounds.
- [ ] Compare Source before and after HEAD/status/content manifest/remotes unchanged; worktree registration and new branches will change the shared Git metadata, which is expected and does not claim that the entire `.git` bytes are unchanged.
- [ ] `cargo test -p archmage-core --test workspace` Passed, submitted `feat: isolate runs in real Git worktrees`.

## Task 07: Process ownership, concurrent lease and cancellation basis

**Modify:** `process.rs`、`store/transitions.rs`。
**Create/Test:** `runtime/control.rs`、`tests/processes.rs`。

- [ ] First use ordinary OS commands to test stdout/stderr for large traffic, timeout, exit after TERM, ignore TERM and then KILL, and clean up the child tree. They are not Providers and do not generate Agent results.
- [ ] `cargo test -p archmage-core --test processes` confirmation fails, each attempt is implemented as an independent process group, and the PID/PGID/ process startup ID/owner generation is recorded.
- [ ] The Starting intent is persisted before spawning; after spawning, the identity is registered and turned to Running. Supplementary testing of the crash window after spawning and before identity submission; sending signals based solely on the old PID is not allowed.
- [ ] Get OS owner lock per Run; write/read lease is managed by unique owner and reject old completions with attempt generation fencing. OS owner lock does not restrict multiple Nodes from running at the same time.
- [ ] To cancel, press TERM → bounded wait → KILL → exit confirmation; the Workspace writer lease shall not be released or a replacement writer shall be dispatched before exit confirmation.
- [ ] When the identity of the process cannot be confirmed, writing is blocked and a diagnosis is given; "process information cannot be found" cannot be regarded as "the process is dead".
- [ ] `cargo test -p archmage-core --test processes --test persistence` Passed, submitted `feat: own and cancel execution processes safely`.

## Task 08: Four Capability Contracts and Prompts

**Create:** Four copies of Markdown for `capability.rs`, `contract.rs`, `capabilities/v1/` and four copies of JSON Schema, `tests/contracts.rs`.

- [ ] First write accepted/rejected JSON cases according to the fixed shape below. All schema root objects reject additionalProperties and the required fields are complete; enum cannot be replaced by any string.

| Capabilities | Fixed fields (all required) | Business verification |
|---|---|---|
| Designer | schema_version=`1`; summary_markdown:string; design_markdown:string; acceptance_criteria:string[]; implementation_steps:string[] | design and criteria are not empty; read-only Workspace unchanged |
| Coder | schema_version=`1`; summary_markdown:string; status=`implemented`/`blocked`; changed_paths:string[]; notes:string[] | blocked stops the process; the actual ChangeSet is determined by Runtime, you cannot just trust changed_paths |
| Tester | schema_version=`1`; summary_markdown:string; status=`passed`/`failed`; checks:array of `{check_id:string,evidence_id:string,status:passed/failed}` | required check All match this attempt canonical evidence; cannot miss or reference other attempts |
| Reviewer | schema_version=`1`; summary_markdown:string; status=`approved`/`changes_requested`; issues:array of `{severity:P0/P1/P2/P3,path:string,message:string}` | Test failed cannot be approved; the path must not cross the boundary; approval and execution success are separated |

- [ ] Tests for each result: missing fields, wrong version, extra fields, malformed JSON, wrong enum, empty mandatory design, fake evidence ref.
- [ ] `cargo test -p archmage-core --test contracts` Confirmation failed to implement schema loading and business validators; invalid schema failed in preflight and active Run was not started.
- [ ] Write a complete Prompt for each role: role responsibilities, available inputs, executable actions, forbidden commit/merge/push, how to report blocked, output fields. Tester must call the named check helper; Reviewer does not fix the code.
- [ ] Prompt does not specify a previous session ID, does not call `resume --last`, and does not hide failed tests. The four abilities are all bound to Codex's independent Invocation.
- [ ] `cargo test -p archmage-core --test contracts` Passed, submitted `feat: define four capability contracts`.

## Task 09: Context Snapshot and Handoff

**Create:** `context.rs`、`tests/context.rs`。
**Modify:** `artifact.rs`、`store/transitions.rs`。

- [ ] Typed ContextRef only allows WorkflowInput, CommittedNodeOutput, Artifact, WorkspaceSnapshot; does not implement arbitrary template scripts or SQL/path expressions.
- [ ] Test: Same input and different map insertion orders result in the same hash; missing predecessor prevents scheduling; upstream failed/uncommitted artifact cannot be referenced; manifest tamper is detected; new Prompt version changes snapshot hash.
- [ ] `cargo test -p archmage-core --test context` Acknowledgment failed, implementing canonical JSON (recursive key sorting) + SHA-256; rendering and saving the actual sent stdin bytes.
- [ ] Fixed bindings: Designer=goal+repo view; Coder=goal+design; Tester=goal+design+implementation+check policy+workspace manifest; Reviewer=goal+design+implementation+ChangeSet+TestResult.
- [ ] Generate manifest and necessary file views in the same read lease, and then start the reader; do not misinterpret "read-only" as "no warehouse access".Access to AGENTS.md/Configuration/Tools that can be read by the Codex needs to be logged separately to perform provenance.
- [ ] Context Snapshot represents the input provided by the Archmage and does not claim an implicit system prompt or deterministic output for the full replay model. Human editing outside the managed workspace can only be detected, not blocked with scheduling leases.
- [ ] `cargo test -p archmage-core --test context --test artifacts` Passed, submitted `feat: resolve explicit capability handoffs`.

## Task 10: Check Runner and evidence initiated by Tester

**Create:** `check_runner.rs`、`tests/checks.rs`。
**Modify:** `args.rs`、`main.rs`、`tester.md`、`store/transitions.rs`、`process.rs`。

- [x] The internal subcommand `archmage internal check --endpoint <socket> --attempt <id> --check <id>` only requests the frozen check ID from the active runtime and cannot accept any commands or data directories passed in by the model.
- [x] Runtime is responsible for the actual startup of check and the placement of output to disk. Tester still decides to call, understand the evidence, and form TestResult; Runtime does not replace Tester's ability to generate conclusions.
- [x] Helper and parent use a per-attempt short-lived Unix socket; verify attempt activity, owner generation, request ownership, and fixed check ID.Prove that the current sandbox of Codex can connect to this socket; otherwise, the task will be stopped to report compatibility and full access will not be enabled.
- [x] `--check` is a shell command that the user explicitly authorized. Only this entry uses `/bin/sh -c`, cwd is fixed to worktree; stdin is closed; environment allowlist excludes provider auth; the log shows shell/command/cwd/timeout.
- [x] Parent running check must also be restricted by the supported workspace sandbox, and the helper cannot be made into a Codex sandbox bypass channel. When implementing, use the sandbox launcher actually supported by the Provider and verify it with the real read/write escape probe; if it is not supported, check execution will be explicitly blocked.
- [x] Test matrix: exit 0, exit 7, timeout, unknown ID, expired attempt, repeated request, another attempt evidence, missing items, source changes, must run failed but passed. The normal check shell process is not an Agent emulator.
- [x] `cargo test -p archmage-core --test checks` Red and green passed. The evidence includes executing command, exit/signal, start/end/duration, stdout/stderr hashes, check ID, attempt, and workspace before/after manifest.
- [x] If check is non-zero, evidence will also be returned to Tester; transport failure/loss of evidence will cause the capability to fail. The model cannot write canonical evidence. Repeated requests will return completed evidence or explicit in-progress, and no double execution will be performed.
- [x] Real Codex Tester runs a success check and a failure check, verifying TestResult=`failed`. SUBMIT `feat: capture tester-invoked validation evidence`.

Targeted compatibility check before implementation: Read the `codex sandbox --help` of the installed CLI and the subcommand help of this platform to confirm the supported argv, network policy, working directory and environment transfer. Using a disposable repo verifies that build output is allowed to be created in the run worktree, writing to sibling sources is disabled, socket helpers are available, and no additional shell permissions are required. What is verified here is the support capability, and unobserved CLI flags are not prefilled. Official information is only used as a supplement when the native help is insufficient. Any failure is considered blocked by the C batch project and will be marked as passed after the boundary cannot be lowered.

## Task 11: Ready Set Scheduling and Complete Collaboration Main Chain

**Create:** `scheduler.rs`、`runtime/{mod,dispatch}.rs`、`tests/scheduler.rs`。
**Modify:** `store/transitions.rs`、`provider/mod.rs`、`lib.rs`。

- [ ] Purely scheduled tests use input state data and do not execute the Fake Provider. Matrix: two readers are ready at the same time; read/write conflicts; none/write coexists; quota is exhausted; predecessor fails; only one of the two parent nodes succeeds; double dispatch of the same node.
- [ ] `cargo test -p archmage-core --test scheduler` Confirmation failure, implement Ready Set calculation and stabilize node order; update resource view after grant item by item to avoid issuing two writers in the same batch incorrectly.
- [ ] Runtime service definition start/status/logs/resume/cancel; fixed definition four nodes, version written to Run snapshot.
- [ ] Each dispatch sequence: CAS ready → create attempt/lease → resolve context → publish immutable input → spawn Codex → drain events → validate output/workspace/evidence → commit outputs/Handoff/state/Event → release lease.
- [ ] Completing the commit must check the owner generation, attempt generation, and cancel request; old results cannot overwrite the new attempt. Any failure will stop the distribution of new nodes and clean up the associated processes, without automatic Fix.
- [ ] Reviewer's valid changes_requested produces completed+ChangesRequested; failed checks cannot get Approved. Runtime records execution failures separately and does not mix them with business review conclusions.
- [ ] The real and complete Workflow first calls the test API, the four session IDs are different; after the end, check the three main edges of Handoff and the multiple input references of Reviewer.
- [ ] `cargo test -p archmage-core --test scheduler --test contracts --test persistence` Passed, submitted `feat: orchestrate capability agents with ready-set scheduling`.

## Task 12: Restart reconciliation, retry and make partial modifications

**Create:** `runtime/recovery.rs`、`tests/recovery.rs`。
**Modify:** `store/recovery.rs`、`runtime/control.rs`、`workspace/snapshot.rs`。

- [ ] Test breakpoints: after Run is created; worktree is built but not registered; after attempt Starting; before and after spawn; after result is produced but before DB commit; after DB commit but before notification. Use store/process actual operation to inject fault without forging Agent session successfully.
- [ ] `status` Read-only; only the resume/control path that obtains the owner lock can perform reconciliation. Another functioning owner cannot be misidentified by the scan.
- [ ] After confirming that the old Codex/check process has exited or terminated safely, mark the old attempt as interrupted and release the writer lease; do not replace ownership verification with PID survival check.
- [ ] The same successful prefix remains; the pending downstream has not yet started to regenerate workspace refs; retrying a new attempt must have a new result file path, and the old result cannot be accepted through the file existence check.
- [ ] Interrupts the writer's partial modification retention. The new attempt references the original goal/design and other immutable inputs, and creates a new recovery ContextSnapshot, which explicitly contains the current Workspace manifest, the delta relative to the original snapshot, the old attempt ID and the instruction to "check for modifications and continue".
- [ ] Does not automatically reset/stash/checkout, nor declares that rerunning the same prompt is equivalent to execution without side effects. HEAD/branch Identity change prevents recovery; refusal to continue writing when source unknown writer/process identity is unknown.
- [ ] Tester reruns using new attempt evidence; evidence of previous successful checks will not be used across attempts.old attempt diagnostics are retained but cannot be used as new Node output.
- [ ] Only one owner is allowed when two `resume` are initiated at the same time; cancel and completion races are decided by transaction order, and cancellation cannot return to success due to late results.
- [ ] `cargo test -p archmage-core --test recovery --test processes --test workspace` Passed, submitted `feat: reconcile interrupted runs without duplicating writers`.

## Task 13: CLI full commands, Timeline and reports

**Create:** `report.rs`。
**Modify:** CLI `src/{main,args,output}.rs`、`tests/cli.rs`、`lib.rs`。

- [ ] CLI test: required args; `status <missing>` is non-zero; `status --json` has no log impurities; logs sequence is monotonic; invalid flags does not create Run; terminal resume no-op does not call Codex; completed/cancelled exit mapping.
- [ ] `cargo test -p archmage-cli --test cli` Confirmation failed, connecting six commands to runtime services, not writing SQL/scheduling branch in CLI.
- [ ] `logs --follow` uses the SQLite cursor query as the authority, and the notification is only used to wake up; if the in-process notification is lost, the event cannot be lost, and the last cursor is completed before exiting.
- [ ] `cancel` Use durable control_request to wake up / be polled by owner, active owner execution is terminated; perform security reconciliation when there is no owner. Clearly output the difference between "requested" and "terminated".
- [ ] `run` Frontend Timeline; Ctrl-C creates cancel request and waits for bounded cleanup; non-interactive mode does not play unmodeled approval input.
- [ ] The final report outputs Run ID, lifecycle, review outcome, each attempt, worktree/branch, Context/Handoff/artifact links, check evidence, source warning, and unexecuted items; the Artifact path is generated by Runtime.
- [ ] `cargo test --workspace`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` passed, submitted `feat: expose observable workflow control through CLI`.

## Task 14: Real Codex end-to-end access control and delivery

**Create:** `crates/archmage-cli/tests/acceptance_live.rs`、`tests/fixtures/health-repo/{health,test_health}.py`、`docs/acceptance/s0/README.md`。
**Modify:** `docs/runbooks/s0-acceptance.md`、`docs/runbooks/codex-provider.md`、`docs/README.md`、`docs/roadmap/stages/s0-real-codex-slice.md`。

- [ ] Initialize a temporary real Git repository using the Python standard library's network-less fixtures. The fixture initially only implements `/`; the target is to add `/health`, and unknown paths return 404. The initial files are as follows.

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

class ResponseTests(unittest. TestCase):
    def test_root(self):
        self.assertEqual(response("/")[0], 200)

    def test_unknown(self):
        self.assertEqual(response("/missing")[0], 404)

if __name__ == "__main__":
    unittest.main()
```

- [ ] Accept the harness, copy the fixture, init/config local git identity/commit, record the source baseline; use independent `ARCHMAGE_HOME` for each scene, do not let the command write personal product data or the source code of this project.
- [ ] Scenario `complete`: real `archmage run`, goal requires `/health → (200,{"status":"ok"})` and adds test, `--check "python3 -m unittest -v"`. Verify four different thread IDs, all contracts, checks, and three main chain handoffs; the independent oracle directly imports the `health.response` verification target of run-worktree. You cannot just trust the model and write the test yourself.
- [ ] Scenario `interrupt_resume`: check uses gate files to wait for release when ready instead of fixed sleep guessing timing. Wait for durable check.started and then kill the parent runtime, confirm the actual orphan processing, then release gate and resume; the upstream session ID remains unchanged, Tester attempt/session is updated, and the old output is not reused.
- [ ] Scenario `cancel`: Request cancel from second CLI while running, verify all managed processes exit, status cancelled, no late success, Worktree retained.
- [ ] Scenario `dirty_source`: The source workspace changes tracked files and adds untracked files; Run is still created from HEAD, the warning is clear and the source content is retained.
- [ ] Scenario `failed_check`: Add required check `exit 7`; real Tester record failed, Reviewer cannot make final CLI 0. Reports must distinguish between allowing valid changes_requested (10) or being rejected by Contract due to error approval (21).
- [ ] Add opt-in real Provider integration test for concurrency: two independent read-only Codex Invocations are started by scheduler quota, and the running range in events overlaps; you cannot just use a pure compatibility matrix to claim that concurrent scheduling is integrated. The production workflow is still fixed and linear, and no user DAG function is added.
- [ ] Execute the following release gates; live tests are not automatically included in normal cargo tests and must not be ignored as passed.

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p archmage-core --test codex_live -- --ignored --nocapture
cargo test -p archmage-cli --test acceptance_live -- --ignored --nocapture --test-threads=1
```

- [ ] The report is written to `docs/acceptance/s0/README.md`: actual commit, OS/Rust/Git/Codex version, scenario results, Run IDs, desensitization evidence summary, automated test results, and limitations. Only real data, no "pass" results generated in advance.
- [ ] Each live test has a total deadline; account/network/sandbox restrictions are recorded as blocked, and no permission bypass, fake agent or infinite retries are added.
- [ ] Update stage implementation status and runbook commands; check documentation links and `git diff --check`, commit `test: verify real Codex collaboration and recovery`.

## 4. Spec coverage and final review

| Spec requirements | Implementation tasks | Verification |
|---|---|---|
| Core and CLI decoupling | 01, 11, 13 | Core None Clap/Tauri; CLI only calls services |
| Real local Codex | 02, 03, 07, 14 | read-only probe, four sessions, no Fake path |
| Ability as Collaborating Subject | 08, 09, 11 | Versioning role/contract, independent session, multi-input Handoff |
| Status and Event atomic submission | 04, 11 | Event ordering after rollback, CAS, restart |
| Artifact and Context traceable | 05, 09 | hash, orphan, missing/tampered ref reject |
| Git Worktree isolation | 06, 14 | dirty source, double Run, full diff, new file |
| Concurrency Ready Set | 07, 11, 14 | Compatibility matrix, writer conflicts in the same batch, real reader concurrency |
| Tester’s real evidence | 10, 14 | required checks, parent-owned evidence, failed_check |
| Contract and business conclusion | 08, 11, 13 | failed checks unchanged approval; exit 0/10/20/21/22 |
| Cancellation, timeout and recovery | 07, 12, 14 | orphan, owner fencing, partial writer, cancel race |
| No commit/merge/push/delete | 06, 08, 12, 14 | HEAD invariant, source before and after, reserved Workspace |
| CLI and local observation | 13, 14 | status JSON, follow cursor, final report |

The final review must not miss: incomplete output is not appropriate for success; PID alone is not enough for attribution verification; part of the code is not half-finished JSON; test failure is not transport failure; Worktree is not a malicious code security sandbox; local shell/MCP/hooks can form a bypass and requires restricted configuration and real boundary verification.

## 5. Perform handover

Execute batch A first, with real read-only Codex access as the first checkpoint. After passing, press B/C/D to advance. Each batch of reports has passed the test, no live tests and real blocking have been performed, and no successful build is used in lieu of collaborative acceptance.

You can use the current session to execute in batches (`executing-plans`), or you can use sub-Agents to implement and review tasks in batches after user authorization (`subagent-driven-development`). Do not automatically create new Codex tasks, do not automatically push, and do not start S1 in advance.
