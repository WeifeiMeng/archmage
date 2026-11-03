# Desktop Runtime Vertical Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 补齐由真实本地 Codex 驱动的四能力 Runtime 主链，并交付可启动、可观察、可取消和可回看历史 Run 的 Tauri 桌面客户端。

**Architecture:** `archmage-core` 新增 UI 无关的 Runtime facade、状态查询和固定工作流调度；CLI 与 Tauri 都只调用这套 API。桌面端采用 Tauri 2、React、TypeScript 和 Vite，使用 typed commands 读取 durable snapshot/events，实时事件只作为刷新信号。

**Tech Stack:** Rust 1.95、Tokio、SQLite/rusqlite、真实 Codex CLI、Git worktree、Tauri 2、React 19、TypeScript、Vite、Vitest。

---

## 实施约定

- 在独立 `codex/desktop-runtime-slice` worktree 实施，不直接修改 `main`。
- 不增加 Fake Agent；最终验收必须调用已登录的真实 Codex CLI。
- 单个 Run 固定包含 `designer`、`coder`、`tester`、`reviewer` 四个 Node Run。
- S0 的依赖是线性的，但 Ready Set 从节点状态和依赖表计算，不写成四次硬编码调用。
- 测试保持小：每个行为只增加一个聚焦测试；前端只测表单序列化与事件 reducer。
- Runtime 负责持久化、Git、Codex、检查和取消；React 不直接访问 SQLite、Git 或 Codex。
- 不实现 daemon、自动重试、Fix loop、Workflow 编辑器、Provider 设置或自动提交/合并。

## 文件布局

```text
crates/archmage-core/
  migrations/0006_runtime.sql          # Run definition、结果 artifact 与取消字段
  src/provider/codex.rs                # 通用 schema-bound Codex invocation
  src/runtime/mod.rs                   # Runtime facade 与 DTO
  src/runtime/scheduler.rs             # Ready Set 与终态归并
  src/runtime/dispatch.rs              # 单 Node 上下文、Codex、artifact、handoff
  src/runtime/recovery.rs              # 强退后的 interrupted 对账
  src/store/mod.rs                     # snapshot/list/query DTO 查询
  src/store/transitions.rs             # Runtime 原子状态迁移
  tests/runtime.rs                     # 小型 persisted lifecycle/Ready Set 测试
  tests/runtime_live.rs                # ignored 真实 Codex 四能力验收
crates/archmage-cli/
  src/main.rs                          # run/status/logs/cancel 接线
  src/output.rs                        # 人类可读及 JSON 输出
  tests/cli.rs                         # 非网络 CLI 查询测试
apps/archmage-desktop/
  package.json                         # 前端与 Tauri scripts
  src-tauri/Cargo.toml                 # Tauri shell，依赖 archmage-core
  src-tauri/src/lib.rs                 # AppState、commands、event forwarding
  src-tauri/tauri.conf.json            # Vite/Tauri 配置
  src-tauri/capabilities/default.json  # 最小 dialog/opener 权限
  src/api.ts                           # typed invoke/listen boundary
  src/model.ts                         # Runtime DTO 与 reducer
  src/App.tsx                          # workbench composition
  src/components/                      # RunForm/Sidebar/Progress/Timeline/Inspector
  src/styles.css                       # option A visual system
  src/model.test.ts                    # reducer/form tests
docs/cli-reference.md                  # 可复制的 run/status/logs/cancel 命令
docs/runbooks/desktop-s0.md            # 桌面启动与真实验收步骤
```

### Task 1：Runtime 数据契约与持久化查询

**Files:**
- Create: `crates/archmage-core/migrations/0006_runtime.sql`
- Create: `crates/archmage-core/tests/runtime.rs`
- Modify: `crates/archmage-core/src/store/mod.rs`
- Modify: `crates/archmage-core/src/store/transitions.rs`
- Modify: `crates/archmage-core/src/runtime/mod.rs`

- [ ] **Step 1: 写失败测试**

在 `tests/runtime.rs` 建临时 Store，用 `RunRequest` 创建 Run 后断言 definition 保存 repo/goal/checks、四个节点顺序固定，`list_runs()` 与 `snapshot()` 返回同一 Run。

```rust
#[test]
fn start_definition_is_queryable_as_one_four_node_run() {
    let fixture = RuntimeFixture::new();
    let request = RunRequest::new(fixture.repo(), "add health endpoint", vec![
        CheckSpec::new("check-1", "cargo test"),
    ]).unwrap();
    let run_id = fixture.runtime.create_run_definition(&request).unwrap();
    let snapshot = fixture.runtime.snapshot(&run_id).unwrap();
    assert_eq!(snapshot.goal, "add health endpoint");
    assert_eq!(snapshot.nodes.iter().map(|n| n.key.as_str()).collect::<Vec<_>>(),
               ["designer", "coder", "tester", "reviewer"]);
    assert_eq!(snapshot.checks[0].command, "cargo test");
}
```

- [ ] **Step 2: 运行 RED**

Run: `cargo test -p archmage-core --test runtime start_definition_is_queryable_as_one_four_node_run`

Expected: FAIL，因为 `RunRequest`、Runtime 查询和 v6 migration 尚不存在。

- [ ] **Step 3: 实现最小契约**

加入可序列化的 `RunRequest`、`CheckSpec`、`RunSummary`、`RunSnapshot`、`NodeSnapshot`；v6 为 `runs` 增加 repository/definition 所需字段，为 attempts/artifacts 增加结果引用查询所需字段。`Store::list_runs` 按 `created_at DESC`，`Store::run_snapshot` 一次组装 Run、Node、workspace 与 latest event sequence。

- [ ] **Step 4: 运行 GREEN 并提交**

Run: `cargo test -p archmage-core --test runtime`

Expected: PASS。

Commit: `feat: add runtime run snapshots`

### Task 2：通用真实 Codex Invocation

**Files:**
- Modify: `crates/archmage-core/src/provider/codex.rs`
- Modify: `crates/archmage-core/src/provider/mod.rs`
- Modify: `crates/archmage-core/src/process.rs`
- Test: `crates/archmage-core/tests/codex_protocol.rs`

- [ ] **Step 1: 写失败测试**

测试 `CodexInvocation::argv()` 对 designer/read-only 与 coder/workspace-write 生成显式 `--cd`、`--sandbox`、`--json`、`--output-schema`、`--output-last-message`，且 prompt 只从 stdin 传入。

```rust
#[test]
fn invocation_builds_schema_bound_argv_without_prompt_text() {
    let invocation = CodexInvocation::new(repo(), schema(), result(), Sandbox::WorkspaceWrite);
    let argv = invocation.argv();
    assert!(argv.windows(2).any(|v| v == ["--sandbox", "workspace-write"]));
    assert!(!argv.iter().any(|part| part.contains("secret goal")));
}
```

- [ ] **Step 2: 运行 RED**

Run: `cargo test -p archmage-core --test codex_protocol invocation_builds_schema_bound_argv_without_prompt_text`

Expected: FAIL，因为通用 invocation 类型不存在。

- [ ] **Step 3: 实现 invocation**

抽出 `CodexInvocation`、`CodexOutcome` 与 `invoke()`；保留完整 JSONL/stderr/result 文件，限制大小和 deadline，要求 exit code 0、`turn.completed`、thread ID 和合法 result 文件同时成立。不得把 stderr 原文拼入用户错误。

- [ ] **Step 4: 运行 GREEN 并提交**

Run: `cargo test -p archmage-core --test codex_protocol`

Expected: PASS。

Commit: `feat: generalize real Codex invocation`

### Task 3：Ready Set 与 Run 终态

**Files:**
- Create: `crates/archmage-core/src/runtime/scheduler.rs`
- Modify: `crates/archmage-core/src/runtime/mod.rs`
- Modify: `crates/archmage-core/src/store/transitions.rs`
- Test: `crates/archmage-core/tests/runtime.rs`

- [ ] **Step 1: 写失败测试**

对四节点状态分别断言：初始只 designer ready；designer succeeded 后只 coder ready；failed/cancelled 不再派发后继；reviewer succeeded 后 Run completed。

```rust
#[test]
fn ready_set_advances_from_persisted_dependencies() {
    assert_eq!(ready_keys(&states("pending,pending,pending,pending")), ["designer"]);
    assert_eq!(ready_keys(&states("succeeded,pending,pending,pending")), ["coder"]);
    assert!(ready_keys(&states("failed,pending,pending,pending")).is_empty());
}
```

- [ ] **Step 2: 运行 RED**

Run: `cargo test -p archmage-core --test runtime ready_set_advances_from_persisted_dependencies`

Expected: FAIL，因为 scheduler 不存在。

- [ ] **Step 3: 实现 Ready Set**

定义固定 dependency 表并从 snapshot 计算 Ready Set；增加事务方法 `mark_ready`、`complete_run`、`fail_run`、`cancel_run`，每次状态变化与 event 同事务提交。

- [ ] **Step 4: 运行 GREEN 并提交**

Run: `cargo test -p archmage-core --test runtime`

Expected: PASS。

Commit: `feat: schedule persisted workflow nodes`

### Task 4：四能力真实 Dispatch 与 Handoff

**Files:**
- Create: `crates/archmage-core/src/runtime/dispatch.rs`
- Modify: `crates/archmage-core/src/runtime/mod.rs`
- Modify: `crates/archmage-core/src/context.rs`
- Modify: `crates/archmage-core/src/artifact.rs`
- Modify: `crates/archmage-core/src/check_runner.rs`
- Test: `crates/archmage-core/tests/runtime.rs`

- [ ] **Step 1: 写失败测试**

使用已提交 artifact 和真实 SQLite，不启动 Provider，分别构造四个节点的 inputs，断言 required context keys、handoff producer、Tester check policy 和 Reviewer changeset/test result 完整。

```rust
#[test]
fn reviewer_context_uses_canonical_changeset_and_test_evidence() {
    let context = fixture.build_context(CapabilityKind::Reviewer).unwrap();
    assert_eq!(context.keys(), ["goal", "design", "implementation", "changeset", "test_result"]);
    assert!(context.handoffs.iter().all(|handoff| !handoff.content_hash.is_empty()));
}
```

- [ ] **Step 2: 运行 RED**

Run: `cargo test -p archmage-core --test runtime reviewer_context_uses_canonical_changeset_and_test_evidence`

Expected: FAIL，因为 dispatch/context builder 尚不存在。

- [ ] **Step 3: 实现单节点执行**

`dispatch_node` 完成 begin attempt、context persist/bind、schema 写入 attempt 目录、真实 Codex invocation、contract 校验、结果 artifact 发布和 attempt completion。Designer/Reviewer 用 read-only；Coder/Tester 使用 worktree-write。Tester 启动 attempt-specific check socket，只有 `archmage internal check` 可请求冻结 check；Reviewer 输入使用 Runtime 发布的 ChangeSet 与 canonical check evidence。

- [ ] **Step 4: 接入 `run_to_terminal`**

循环读取 snapshot → 计算 Ready Set → dispatch；节点失败时将 Run 标为 failed，不自动重试。Reviewer `approved` 对应 completed/approved，`changes_requested` 对应 completed/changes_requested。

- [ ] **Step 5: 运行 GREEN 并提交**

Run: `cargo test -p archmage-core --test runtime --test context --test contracts --test checks`

Expected: PASS。

Commit: `feat: execute four capability workflow`

### Task 5：取消、恢复和 CLI 主链

**Files:**
- Create: `crates/archmage-core/src/runtime/recovery.rs`
- Modify: `crates/archmage-core/src/runtime/control.rs`
- Modify: `crates/archmage-core/src/runtime/mod.rs`
- Modify: `crates/archmage-core/src/store/recovery.rs`
- Modify: `crates/archmage-cli/src/main.rs`
- Modify: `crates/archmage-cli/src/output.rs`
- Modify: `crates/archmage-cli/tests/cli.rs`
- Test: `crates/archmage-core/tests/runtime.rs`

- [ ] **Step 1: 写失败测试**

测试 cancel request 是 durable event；启动时发现 active attempt 但 owner lock 无持有者时标记 attempt/node/run interrupted；status/logs 只读且不会擅自改变状态。

```rust
#[test]
fn reconciliation_marks_abandoned_run_interrupted() {
    let run = fixture.running_attempt();
    fixture.runtime.reconcile_interrupted().unwrap();
    assert_eq!(fixture.runtime.snapshot(&run).unwrap().state, "interrupted");
}
```

- [ ] **Step 2: 运行 RED**

Run: `cargo test -p archmage-core --test runtime reconciliation_marks_abandoned_run_interrupted`

Expected: FAIL，因为 Runtime recovery 不存在。

- [ ] **Step 3: 实现 control/recovery 与 CLI**

`Runtime::cancel` 持久化请求并终止 owned process group；`reconcile_interrupted` 只在能取得 Run owner lock且进程身份不再存活时写 interrupted。接线 `archmage run/status/logs/cancel`，`run` 前台等待 terminal，`status --json` 输出 snapshot，`logs` 输出 events。

- [ ] **Step 4: 运行 GREEN 并提交**

Run: `cargo test -p archmage-core --test runtime && cargo test -p archmage-cli --test cli`

Expected: PASS。

Commit: `feat: expose runtime through CLI`

### Task 6：Tauri 2 + React 桌面骨架

**Files:**
- Create: `apps/archmage-desktop/package.json`
- Create: `apps/archmage-desktop/package-lock.json`
- Create: `apps/archmage-desktop/index.html`
- Create: `apps/archmage-desktop/tsconfig.json`
- Create: `apps/archmage-desktop/vite.config.ts`
- Create: `apps/archmage-desktop/src-tauri/Cargo.toml`
- Create: `apps/archmage-desktop/src-tauri/src/{main,lib}.rs`
- Create: `apps/archmage-desktop/src-tauri/{build.rs,tauri.conf.json}`
- Create: `apps/archmage-desktop/src-tauri/capabilities/default.json`
- Modify: `Cargo.toml`

- [ ] **Step 1: 生成并收紧官方 Tauri 2 React/TS/Vite 结构**

使用当前官方模板结构，workspace 加入 `apps/archmage-desktop/src-tauri`。只启用 core commands、dialog folder picker 和 opener；不启用 shell plugin，避免前端直接执行任意命令。

- [ ] **Step 2: 实现 typed Tauri commands**

`AppState` 保存 Runtime 和 `run_id -> JoinHandle`。命令实现 `doctor`、`start_run`、`list_runs`、`get_run_snapshot`、`list_events`、`cancel_run`、`reveal_workspace`；`start_run` 创建 durable Run 后 spawn `run_to_terminal`，通过 `run-event` 只发送 run_id/event_seq。

- [ ] **Step 3: 编译并提交**

Run: `npm install --prefix apps/archmage-desktop && npm run build --prefix apps/archmage-desktop && cargo check -p archmage-desktop`

Expected: TypeScript/Vite build 与 Tauri Rust check 均成功。

Commit: `feat: scaffold Tauri runtime client`

### Task 7：Workbench-first 客户端

**Files:**
- Create: `apps/archmage-desktop/src/{main.tsx,api.ts,model.ts,App.tsx,styles.css}`
- Create: `apps/archmage-desktop/src/components/{RunForm,RunSidebar,RunHeader,NodeProgress,Timeline,NodeInspector}.tsx`
- Create: `apps/archmage-desktop/src/model.test.ts`

- [ ] **Step 1: 写两个失败测试**

表单测试把 repository/goal/check rows 序列化为 `RunRequest` 并过滤空 check；reducer 测试按 event_seq 去重、检测 sequence gap 并要求 snapshot refresh。

```ts
it("serializes nonblank checks", () => {
  expect(toRunRequest("/repo", "goal", ["npm test", " "]).checks)
    .toEqual([{ checkId: "check-1", command: "npm test" }]);
});

it("requests refresh on an event gap", () => {
  expect(reduceEvent(stateAt(4), eventAt(6)).needsRefresh).toBe(true);
});
```

- [ ] **Step 2: 运行 RED**

Run: `npm test --prefix apps/archmage-desktop`

Expected: FAIL，因为 model/reducer 尚不存在。

- [ ] **Step 3: 实现数据层与布局 A**

建立 typed invoke wrappers；启动时 `list_runs`，选择 Run 时加载 snapshot+events，监听 `run-event` 后增量刷新。实现左侧 Run 列表、新 Run 表单、四节点进度、timeline、inspector、Cancel/Reveal Workspace。状态颜色和文案同时表达，不只依赖颜色。

- [ ] **Step 4: 实现窗口关闭保护**

存在 active Run 时阻止 close，并用原生确认框提供“返回应用”或“取消并退出”；不宣称关闭后后台继续。

- [ ] **Step 5: 运行 GREEN 并提交**

Run: `npm test --prefix apps/archmage-desktop && npm run build --prefix apps/archmage-desktop`

Expected: PASS。

Commit: `feat: build desktop run workbench`

### Task 8：真实 Codex 验收与使用文档

**Files:**
- Create: `crates/archmage-core/tests/runtime_live.rs`
- Create: `docs/runbooks/desktop-s0.md`
- Modify: `docs/cli-reference.md`
- Modify: `README.md`

- [ ] **Step 1: 增加 ignored 真实验收**

fixture repo 只包含一个可快速修改的小任务和一条检查命令。测试调用公开 Runtime API，断言四节点均产生不同真实 Codex thread ID、Run terminal、events 连续、worktree 可检查、source repo HEAD/status/content 未变。

- [ ] **Step 2: 运行真实 Core/CLI 验收**

Run: `cargo test -p archmage-core --test runtime_live -- --ignored --nocapture`

Expected: 真实 Codex 完成四次 capability invocation；若 Reviewer changes_requested，流程仍是合法 terminal，但不得伪装成 approved。

- [ ] **Step 3: 启动桌面端手工烟测**

Run: `npm run tauri dev --prefix apps/archmage-desktop`

验证：选择 repo、输入目标/检查、启动真实 Run、观察四节点/timeline、查看历史 Run、Reveal Workspace。只做这一轮桌面手工验证。

- [ ] **Step 4: 更新文档并做最终轻量门禁**

文档给出 `archmage run/status/logs/cancel` 与 `npm run tauri dev --prefix apps/archmage-desktop` 的可复制命令，明确 `resume` 尚未提供自动修复语义。

Run: `cargo fmt --all --check && cargo test --workspace && npm test --prefix apps/archmage-desktop && npm run build --prefix apps/archmage-desktop`

Expected: 所有非 ignored 检查通过。

Commit: `docs: add runtime and desktop guide`
