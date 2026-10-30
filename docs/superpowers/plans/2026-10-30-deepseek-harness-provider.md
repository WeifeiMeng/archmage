# DeepSeek Harness Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current experimental DeepSeek Harness headless-CLI integration with an Archmage-owned Python SDK sidecar, so a `Run + Capability` keeps one DSH conversation for its full live Run and Agent Management can configure, discover, verify, and bind that provider.

**Architecture:** Archmage ships a small JSONL Python bridge plus a Cordis composition and launches them through the published `deepseek-harness-sdk` / matching `deepseek-harness-runtime-bin`. Rust owns one long-lived bridge process per `(run_id, capability)`, uses temporary bridges for consultations, strictly parses the final assistant text as one JSON object, and destroys all sidecars at Run terminal/cancel boundaries. The existing Codex path remains stateless and unchanged; no DeepSeek Harness repository change is part of this delivery.

**Tech Stack:** Rust 2024, Tokio process/IO primitives, SQLite/rusqlite, Python 3 + `deepseek-harness-sdk`, JSONL, Cordis YAML, React/TypeScript/Vitest, Cargo tests.

---

## Scope and replacement rules

- Work only in the Archmage repository. The existing DSH worktree/branch created for the superseded CLI design is not merged, edited, reset, or deleted by this plan.
- Rewrite the unshipped migration `0016_deepseek_harness.sql` in place. This feature branch has not been released, so retaining the obsolete `provider_sessions` table or its seeded guessed-model Agent would create needless compatibility debt.
- Preserve the already-added `ProviderKind::DeepseekHarness`, but replace every headless CLI flag, create/resume lease, and persisted-provider-session assumption with the SDK bridge/pool model.
- Keep verification focused: unit/integration tests use controlled bridge/SDK fakes; the real credentialed smoke is one optional final command and is reported as skipped when prerequisites are absent.

## Task 1: Correct the Agent schema and frozen configuration

**Files:**
- Modify: `crates/archmage-core/migrations/0016_deepseek_harness.sql`
- Modify: `crates/archmage-core/src/agent.rs`
- Modify: `crates/archmage-core/src/store/mod.rs`
- Modify: `crates/archmage-core/src/runtime/mod.rs`
- Modify: `crates/archmage-core/tests/agents.rs`
- Modify: `crates/archmage-core/tests/runtime.rs`

- [ ] **Step 1: Write failing schema and validation tests**

Add tests proving:

1. schema 8 upgrades preserve existing Codex instances/bindings and create no DSH instance automatically;
2. `agent_instances` has nullable `provider_route` and no `provider_sessions` table;
3. DSH input requires nonblank `provider_route` and `model`;
4. Codex rejects `provider_route` but retains its optional `model` behavior;
5. saving a changed Python path, route, or model invalidates verification;
6. frozen Run bindings retain resolved Python path, route, and model after the global Agent is edited.

Use this input shape in all new fixtures:

```rust
AgentInstanceInput {
    id: "dsh-sdk".into(),
    provider_kind: ProviderKind::DeepseekHarness,
    display_name: "DSH SDK".into(),
    executable_path: Some("/usr/bin/python3".into()),
    provider_route: Some("deepseek-official".into()),
    model: Some("deepseek-v4-flash".into()),
    enabled: true,
}
```

- [ ] **Step 2: Run the focused tests and confirm the red state**

Run: `cargo test -p archmage-core --test agents --test runtime`

Expected: compile/assertion failures because `provider_route` is absent and migration 16 still seeds an Agent and creates `provider_sessions`.

- [ ] **Step 3: Rewrite migration 16 and the Rust data model**

Make migration 16 rebuild `agent_instances` with this column order:

```sql
id, provider_kind, display_name, executable_path, provider_route, model,
enabled, connection_state, discovered_version, last_verified_at, last_error,
verification_thread_id, created_at, updated_at
```

Copy existing rows with `NULL AS provider_route`, rebuild `capability_bindings` to preserve foreign keys, and finish with only `INSERT INTO schema_migrations(version) VALUES(16);`. Do not seed a DSH Agent and do not create `provider_sessions`.

Add `provider_route: Option<String>` to `AgentInstanceInput`, `AgentInstance`, and `FrozenAgentBinding`. Validation must trim stored values and enforce this matrix:

```rust
match self.provider_kind {
    ProviderKind::Codex if self.provider_route.is_some() => {
        Err(Error::Preflight("Codex does not use a provider route".into()))
    }
    ProviderKind::DeepseekHarness
        if self.provider_route.as_deref().is_none_or(|v| v.trim().is_empty())
            || self.model.as_deref().is_none_or(|v| v.trim().is_empty()) =>
    {
        Err(Error::Preflight(
            "DeepSeek Harness provider route and model are required".into(),
        ))
    }
    _ => Ok(()),
}
```

Update every Agent SELECT/row index and upsert. Verification state is retained only when `executable_path`, `provider_route`, and `model` are all unchanged. Bump `FrozenRunDefinition.version` from 5 to 6 and freeze all three DSH launch values.

- [ ] **Step 4: Remove the persisted session lease API**

Delete `ProviderSessionMode`, `ProviderSessionLease`, `Store::begin_provider_session`, and `Store::mark_provider_session_ready`, plus their tests. Session ownership will move to the live sidecar pool in Task 4.

- [ ] **Step 5: Re-run focused tests and commit**

Run: `cargo test -p archmage-core --test agents --test runtime`

Expected: PASS.

Commit: `git add crates/archmage-core && git commit -m "feat: configure DeepSeek Harness SDK agents"`

## Task 2: Add the Archmage-owned SDK bridge and Cordis composition

**Files:**
- Create: `crates/archmage-core/resources/deepseek_harness_bridge.py`
- Create: `crates/archmage-core/resources/deepseek_harness.cordis.yml`
- Create: `crates/archmage-core/tests/fixtures/deepseek_harness_sdk/deepseek_harness/__init__.py`
- Create: `crates/archmage-core/tests/deepseek_harness_bridge.rs`

- [ ] **Step 1: Write a failing black-box bridge test**

The test launches the real bridge script with `PYTHONPATH` pointing at the fake `deepseek_harness` package, sends two JSONL `invoke` frames, then a `close` frame. Assert:

- stdout contains exactly `ready`, two matching `result` frames, and `closed`;
- both results carry the same configured session ID;
- the fake SDK log shows one `DeepSeekHarness` instance and two ordered `Session.run()` calls;
- invalid input, duplicate invocation ID, oversized final text, and SDK exception each return one bounded `error` frame and terminate nonzero;
- protocol output never contains SDK diagnostics written to stderr.

Run: `cargo test -p archmage-core --test deepseek_harness_bridge`

Expected: FAIL because the bridge/resources do not exist.

- [ ] **Step 2: Implement the bridge protocol using only the public high-level SDK**

The bridge reads one startup JSON object from stdin:

```json
{"op":"start","protocol":1,"cwd":"/repo","provider":"deepseek-official","model":"deepseek-v4-flash","sessionId":"archmage-...","cordis":"/private/cordis.yml","sessionRoot":"/private/sessions","maxResultBytes":1048576,"requestTimeoutSeconds":900}
```

It must instantiate exactly:

```python
config = DeepSeekHarnessConfig(
    provider=start["provider"],
    model=start["model"],
    cwd=start["cwd"],
    runtime_cwd=start["cwd"],
    session_root=start["sessionRoot"],
    cordis=start["cordis"],
    request_timeout_seconds=start["requestTimeoutSeconds"],
)
harness = DeepSeekHarness(config)
session = harness.start_session(start["sessionId"])
```

For each unique request:

```json
{"op":"invoke","id":"attempt-or-consultation-id","task":"...","schema":{}}
```

render one prompt containing the task followed by the compact schema and the instruction `Return exactly one JSON object matching this schema, without Markdown fences.`; call `session.run(prompt)`; reject non-`completed` finish reasons, empty responses, invalid UTF-8 at the process boundary, and responses over `maxResultBytes`; then emit:

```json
{"op":"result","id":"...","sessionId":"archmage-...","text":"{...}","finishReason":"completed"}
```

`close` calls `harness.close()` in `finally` and emits `closed`. Catch errors at the top level, emit a safe `{op:"error", id, code, message}` frame without environment/credential values, then exit nonzero. Use `importlib.metadata.version()` to verify `deepseek-harness-sdk` and `deepseek-harness-runtime-bin` are present and exactly version-matched before emitting `ready`.

- [ ] **Step 3: Add the unattended Cordis composition**

Base the resource on the runtime-wheel plugins proven by the official JSON-RPC and ACP examples. Include:

- `@deepseek-ai/dsh-sdk-jsonrpc-server`;
- `@deepseek-ai/dsh-llm-deepseek`;
- `@deepseek-ai/dsh-sandbox-local` and `@deepseek-ai/dsh-sandbox-policy`, with `mode: !!js process.env.DSH_PERMISSION_MODE` and workspace root `DSH_CWD`;
- local subprocess plus sandboxed Bash;
- `@deepseek-ai/dsh-user-approval` with `policy: never`;
- the agent spine with background jobs and interactive questions disabled;
- JSONL persistence and checkpoint policy;
- sandboxed filesystem, observation policy, and filesystem tools;
- token meter and bounded compaction.

No console logger, terminal UI, question tool, telemetry exporter, or interactive approval answerer may be present because stdout belongs to JSONL and Archmage execution is unattended.

- [ ] **Step 4: Make the fake SDK assert applied configuration**

The fake package must expose the same constructor surface used above and record provider, model, cwd, cordis, session root, session ID, prompts, and close count to a test-only path supplied by `FAKE_DSH_LOG`. It must also support deterministic exception/oversize/finish-reason modes through non-secret test variables.

- [ ] **Step 5: Run and commit**

Run: `cargo test -p archmage-core --test deepseek_harness_bridge`

Expected: PASS.

Commit: `git add crates/archmage-core/resources crates/archmage-core/tests && git commit -m "feat: add DeepSeek Harness SDK bridge"`

## Task 3: Build the persistent Rust sidecar transport

**Files:**
- Modify: `crates/archmage-core/src/process.rs`
- Replace: `crates/archmage-core/src/provider/deepseek_harness.rs`
- Replace: `crates/archmage-core/tests/deepseek_harness_protocol.rs`
- Modify: `crates/archmage-core/tests/provider_execution.rs`

- [ ] **Step 1: Write failing transport tests with a fake JSONL bridge**

Cover ready handshake, two sequential invocations on one process, invocation-ID matching, stdout line/result limits, timeout, malformed/duplicate frames, early exit, bounded stderr capture, graceful close, and process-group kill. The fake bridge must record its PID so the test can prove a cancelled sidecar is gone.

Run: `cargo test -p archmage-core --test deepseek_harness_protocol --test provider_execution`

Expected: FAIL because the current adapter launches one headless CLI process per call.

- [ ] **Step 2: Add an owned piped-process primitive**

Extend `process.rs` with `spawn_owned_piped(binary, args, cwd, env)` returning a type that owns `ChildStdin`, line-buffered `ChildStdout`, a background bounded-stderr drain, `ProcessIdentity`, `try_wait`, `wait`, and the existing identity-checked TERM/KILL ladder. Keep `spawn_owned` and bounded one-shot capture behavior unchanged for Codex and existing tests.

Do not hold a blocking mutex across async reads/writes. A sidecar is serialized by one Tokio mutex at the provider-pool layer.

- [ ] **Step 3: Replace the CLI adapter with an SDK sidecar**

Define these core types:

```rust
pub struct SdkInspection {
    pub python_binary: PathBuf,
    pub sdk_version: String,
    pub runtime_version: String,
}

pub struct SidecarConfig {
    pub python_binary: PathBuf,
    pub repository: PathBuf,
    pub provider_route: String,
    pub model: String,
    pub session_id: String,
    pub permission_mode: PermissionMode,
    pub session_root: PathBuf,
    pub diagnostics_dir: PathBuf,
    pub deadline: Duration,
    pub max_line_bytes: usize,
    pub max_stream_bytes: usize,
    pub max_result_bytes: usize,
}

pub struct DeepseekHarnessSidecar { /* process, framed IO, config, poisoned */ }
```

`DeepseekHarnessSidecar::start` materializes the embedded Python/YAML resources under `<data_root>/provider-resources/deepseek-harness/<sha256>/` with owner-only permissions, launches `<python> -u bridge.py`, inherits the current environment (including `DSH_HOME` and provider credentials), and overrides only `DSH_PERMISSION_MODE`. It sends `start`, waits for exactly one compatible `ready`, and exposes its process identity.

`invoke(id, prompt, schema)` writes one frame, reads exactly the matching terminal frame under the deadline, validates byte/line bounds, strictly runs `serde_json::from_str::<serde_json::Value>(&text)`, and returns `ProviderOutcome { conversation_id: session_id, result }`. Any timeout/protocol/exit error marks the sidecar poisoned; later calls return the original stable failure without respawning.

- [ ] **Step 4: Simplify the common executor boundary**

Remove `ProviderSession`, `ProviderLifecycle::ConversationPersisted`, and all old DSH CLI invocation construction. Retain the Codex one-shot branch in `executor.rs`; expose a provider-neutral result and process-spawn observer, while DSH calls are supplied an already-owned sidecar by the pool in Task 4.

- [ ] **Step 5: Run and commit**

Run: `cargo test -p archmage-core --test deepseek_harness_protocol --test provider_execution`

Expected: PASS.

Commit: `git add crates/archmage-core/src crates/archmage-core/tests && git commit -m "feat: run DeepSeek Harness through an SDK sidecar"`

## Task 4: Add the `(Run, Capability)` sidecar pool

**Files:**
- Create: `crates/archmage-core/src/provider/pool.rs`
- Modify: `crates/archmage-core/src/provider/mod.rs`
- Modify: `crates/archmage-core/src/runtime/mod.rs`
- Create: `crates/archmage-core/tests/provider_pool.rs`

- [ ] **Step 1: Write failing pool tests**

Use a configurable fake bridge and assert:

- two invocations for `(run-a, coder)` use one PID/session;
- `(run-a, reviewer)` and `(run-b, coder)` use distinct PID/session values;
- a consultation uses a fresh temporary sidecar and does not alter the target's main sidecar;
- a poisoned key never respawns;
- `close_run("run-a")` closes only run-a sidecars;
- `cancel_capability("run-a", "coder")` kills only coder and poisons that key.

Run: `cargo test -p archmage-core --test provider_pool`

Expected: FAIL because no pool exists.

- [ ] **Step 2: Implement pool ownership without global serialization**

Use:

```rust
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct SidecarKey { pub run_id: String, pub capability: String }

pub struct ProviderPool {
    entries: tokio::sync::Mutex<HashMap<SidecarKey, Arc<tokio::sync::Mutex<PoolEntry>>>>,
}
```

The map lock is held only for lookup/insert/remove; each sidecar lock serializes that Capability's turns. `invoke_main` lazily creates a stable `archmage-<uuid>` session and records either `Live(sidecar)` or `Poisoned(error)`. `invoke_temporary` uses `archmage-consult-<uuid>` and always closes/kills in a finally-style path. `close_run` removes entries before awaiting shutdown, so no new caller can acquire a closing entry.

- [ ] **Step 3: Put the pool on Runtime**

Change `Runtime` to hold `Arc<ProviderPool>` alongside `Config`, initialize it in `Runtime::open`, and keep `Clone` sharing the same pool. Add a test-only constructor or bridge override through `Config` rather than global environment mutation so parallel tests remain deterministic.

- [ ] **Step 4: Run and commit**

Run: `cargo test -p archmage-core --test provider_pool`

Expected: PASS.

Commit: `git add crates/archmage-core && git commit -m "feat: retain DSH conversations per run capability"`

## Task 5: Route normal, flow, consultation, cancellation, and recovery paths

**Files:**
- Modify: `crates/archmage-core/src/runtime/dispatch.rs`
- Modify: `crates/archmage-core/src/runtime/flow.rs`
- Modify: `crates/archmage-core/src/runtime/consultation.rs`
- Modify: `crates/archmage-core/src/runtime/recovery.rs`
- Modify: `crates/archmage-core/src/runtime/mod.rs`
- Modify: `crates/archmage-core/tests/runtime.rs`
- Modify: `crates/archmage-core/tests/flow.rs`
- Modify: `crates/archmage-core/tests/consultations.rs`

- [ ] **Step 1: Add failing runtime continuity tests**

Add only the four high-value scenarios:

1. reviewer feedback reopens coder and coder's fake bridge log shows two ordered turns on one session;
2. a flow delivery revisiting the same Capability reuses its sidecar;
3. consultation gets a temporary session and the later main target turn uses its untouched main session;
4. cancel/terminal completion leaves no recorded fake bridge PID alive.

Run: `cargo test -p archmage-core --test runtime --test flow --test consultations`

Expected: FAIL until all runtime paths use the pool and terminal cleanup is centralized.

- [ ] **Step 2: Route DSH executions through the pool**

Replace `main_provider_session(...)` and the old provider-session lease calls. Build a `SidecarConfig` from the frozen binding (`executable_path`, `provider_route`, `model`), workspace, schema, limits, and capability permission:

```rust
let permission = match kind {
    CapabilityKind::Designer | CapabilityKind::Reviewer => PermissionMode::ReadOnly,
    CapabilityKind::Coder | CapabilityKind::Tester => PermissionMode::WorkspaceWrite,
};
```

Normal and flow paths call `pool.invoke_main(SidecarKey { run_id, capability }, ...)`. Consultation calls `pool.invoke_temporary(...)` with read-only permission. Codex continues through the existing executor. Attach the DSH bridge `ProcessIdentity` to the active attempt/consultation record so existing recovery can kill orphan process groups after a crash.

- [ ] **Step 3: Centralize live-run cleanup**

Wrap both legacy and flow terminal drivers so every return path performs `pool.close_run(run_id).await`. On explicit cancellation, request cancellation, call `pool.cancel_run(run_id).await`, kill any database-recorded orphan identity as a fallback, then apply the cancel transition. Cleanup errors are recorded as diagnostics and may turn an otherwise successful provider cleanup into a reported infrastructure failure, but never turn a failed/cancelled Run into success.

`reconcile_interrupted` remains restart-safe: the in-memory pool is empty after restart, so it kills persisted active attempt/consultation identities and marks the Run interrupted. Do not claim or implement DSH session resume across Archmage restarts.

- [ ] **Step 4: Run and commit**

Run: `cargo test -p archmage-core --test runtime --test flow --test consultations`

Expected: PASS.

Commit: `git add crates/archmage-core && git commit -m "feat: route Archmage runs through persistent DSH sidecars"`

## Task 6: Replace discovery and verification with SDK checks

**Files:**
- Modify: `crates/archmage-core/src/agent.rs`
- Modify: `crates/archmage-core/src/provider/deepseek_harness.rs`
- Modify: `crates/archmage-core/tests/agents.rs`

- [ ] **Step 1: Write failing management tests**

Test default Python resolution (`python3`), explicit Python path, import failure, missing runtime distribution, SDK/runtime version mismatch, nonblank route/model validation, and a successful verification that performs one temporary read-only model turn and closes the process.

Run: `cargo test -p archmage-core --test agents`

Expected: FAIL because discovery still inspects the DSH CLI and verification uses headless flags.

- [ ] **Step 2: Implement safe SDK inspection**

Resolve the configured executable or `python3`, then run a bounded probe equivalent to:

```python
import json
from importlib.metadata import version
import deepseek_harness
sdk = version("deepseek-harness-sdk")
runtime = version("deepseek-harness-runtime-bin")
print(json.dumps({"sdkVersion": sdk, "runtimeVersion": runtime}))
```

Require exact version equality. Return a combined display version such as `SDK 0.4.0 / runtime 0.4.0`, and sanitize all errors before persisting them.

- [ ] **Step 3: Verify through the real bridge boundary**

`AgentService::verify` creates a private temporary Git directory, starts a temporary read-only sidecar with the configured route/model, requests a fixed tiny schema such as `{"ok": boolean}`, requires `{"ok": true}`, closes the sidecar, and stores `Ready { version, thread_id: session_id }`. It must not read, rewrite, or display `DSH_HOME` or credentials.

- [ ] **Step 4: Run and commit**

Run: `cargo test -p archmage-core --test agents`

Expected: PASS.

Commit: `git add crates/archmage-core && git commit -m "feat: discover and verify DeepSeek Harness SDK agents"`

## Task 7: Update Agent Management UI and API types

**Files:**
- Modify: `apps/archmage-desktop/src/model.ts`
- Modify: `apps/archmage-desktop/src/AgentManagement.tsx`
- Modify: `apps/archmage-desktop/src/AgentManagement.test.tsx`
- Modify: `apps/archmage-desktop/src/model.test.ts`
- Modify: `apps/archmage-desktop/src/managementLayout.test.ts`

- [ ] **Step 1: Write failing UI/model tests**

Assert a DSH Agent carries `providerRoute`, the create/edit form shows `Python 路径`, `Provider route`, and `Model`, route/model are required for DSH, model remains optional for Codex, and the explanatory copy says credentials/endpoints inherit Archmage's environment while route/model are explicit.

Run: `npm test -- --run AgentManagement.test.tsx model.test.ts managementLayout.test.ts` from `apps/archmage-desktop`.

Expected: FAIL because the current form hides DSH model and describes inherited `DSH_HOME` model selection.

- [ ] **Step 2: Update types and forms**

Add `providerRoute: string | null` to `AgentInstance`, `AgentInstanceInput`, and `FrozenAgentBinding`. For DSH, default Python path to blank (backend resolves `python3`) and submit trimmed nonblank route/model. For Codex, submit `providerRoute: null`. Keep unsupported Provider cards non-creatable.

Display discovered SDK/runtime versions and existing connection state fields without showing environment values or credential material.

- [ ] **Step 3: Run build/tests and commit**

Run from `apps/archmage-desktop`:

```bash
npm test
npm run build
```

Expected: PASS.

Commit: `git add apps/archmage-desktop && git commit -m "feat: configure DSH SDK agents in the desktop UI"`

## Task 8: Replace obsolete docs and run the compact acceptance suite

**Files:**
- Modify: `README.md`
- Modify: `docs/runbooks/desktop-s0.md`
- Modify/Delete: obsolete headless-CLI test fixtures under `crates/archmage-core/tests/`
- Review: `docs/superpowers/specs/2026-10-30-deepseek-harness-provider-design.md`

- [ ] **Step 1: Remove obsolete CLI language and fixtures**

Delete tests/fixtures whose only purpose is asserting `--headless`, `--session-id`, or `--resume-session`. Update the runbook to document:

- install matching `deepseek-harness-sdk` / runtime wheel into the configured Python;
- create a DSH Agent with Python path, route, and model;
- inherited environment/`DSH_HOME` behavior;
- live Run continuity scope and no cross-restart resume;
- read-only versus workspace-write capability permissions;
- one optional real verification/smoke procedure.

State explicitly that no DeepSeek Harness repository patch is required or delivered. Do not delete/reset the separate superseded DSH branch; leave that cleanup for an explicit user decision.

- [ ] **Step 2: Run formatting and the focused full suite**

Run:

```bash
cargo fmt --all -- --check
cargo test -p archmage-core
(cd apps/archmage-desktop && npm test && npm run build)
```

Expected: PASS.

- [ ] **Step 3: Run one optional real SDK smoke**

If the selected Python can import both matching packages and required provider credentials are present, create a temporary Agent/config and run one tiny verification plus a two-turn Run using one existing Capability schema. Otherwise record the precise missing prerequisite and mark only the real smoke as skipped; do not substitute a fake result.

- [ ] **Step 4: Inspect the final diff for scope and secrets**

Run:

```bash
git diff --check
git status --short
git diff --stat HEAD~8..HEAD
rg -n -- "--headless|--resume-session|provider_sessions|DEEPSEEK_API_KEY=|api[_-]?key" crates apps README.md docs
```

Expected: no obsolete runtime dependencies, no credential values, and only intended Archmage files changed.

- [ ] **Step 5: Commit documentation/cleanup**

Commit: `git add README.md docs crates/archmage-core/tests && git commit -m "docs: document the DeepSeek Harness SDK provider"`

## Final acceptance checklist

- [ ] Agent Management can create, discover, verify, edit, and bind a DSH SDK Agent with explicit Python/route/model.
- [ ] Same `Run + Capability` reuses one bridge PID and DSH session across retries, reviewer feedback, and flow revisits.
- [ ] Different Run/Capability keys and consultations are isolated.
- [ ] Cancel, failure, completion, and interruption leave no owned sidecar process group running.
- [ ] DSH final text is accepted only as one valid JSON object satisfying the existing Capability contract.
- [ ] Cordis enforces read-only/workspace-write and automatic approval rejection without interactive UI.
- [ ] Codex behavior and existing Run persistence remain green.
- [ ] No DSH repository change is required for delivery.
