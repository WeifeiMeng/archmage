# Qoder ACP Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Qoder as a real local Archmage Provider using an isolated ACP v1 session for every Run and Capability.

**Architecture:** Add a reusable ACP client boundary and a Qoder-specific process adapter, then route Qoder through the existing provider session pool while keeping Context Snapshot, contract validation, Artifact, ChangeSet, and routing inside Archmage. Persist explicit Qoder configuration in agent instances and frozen Run bindings, expose it in the desktop management UI, and validate the integration with a deterministic fake ACP process plus an ignored live smoke test.

**Tech Stack:** Rust 2024, Tokio, official `agent-client-protocol` Rust crate, SQLite/rusqlite, Tauri 2, React 19, TypeScript, Vitest.

---

### Task 1: Persist and expose Qoder agent instances

**Files:**
- Create: `crates/archmage-core/migrations/0017_qoder.sql`
- Modify: `crates/archmage-core/src/store/mod.rs`
- Modify: `crates/archmage-core/src/agent.rs`
- Modify: `crates/archmage-core/src/runtime/mod.rs`
- Modify: `crates/archmage-core/tests/agents.rs`
- Modify: `crates/archmage-core/tests/persistence.rs`

- [ ] **Step 1: Write failing provider and migration tests**

Add tests that parse `qoder`, report it as supported, save and reload a Qoder instance, and migrate a schema-16 database without losing bindings:

```rust
assert_eq!("qoder".parse::<ProviderKind>()?, ProviderKind::Qoder);
assert!(ProviderKind::Qoder.supported());

let input = AgentInstanceInput {
    id: "qoder-local".into(),
    provider_kind: ProviderKind::Qoder,
    display_name: "Qoder Local".into(),
    executable_path: Some("/opt/qoder/bin/qoder".into()),
    provider_route: None,
    model: Some("performance".into()),
    agent_name: Some("archmage-coder".into()),
    provider_config: serde_json::json!({"mcpServers": []}),
    enabled: true,
};
store.save_agent_instance(&input)?;
assert_eq!(store.agent_instance("qoder-local")?.agent_name.as_deref(), Some("archmage-coder"));
```

- [ ] **Step 2: Run the focused tests and verify they fail**

Run: `cargo test -p archmage-core --test agents qoder -- --nocapture && cargo test -p archmage-core --test persistence schema_16_upgrades_to_qoder`

Expected: compilation or assertion failure because `ProviderKind::Qoder`, `agent_name`, `provider_config`, and migration 17 do not exist.

- [ ] **Step 3: Add migration 17 and model fields**

Create schema 17 by rebuilding `agent_instances` with `qoder` in the check constraint and the new columns:

```sql
CREATE TABLE agent_instances_v17(
 id TEXT PRIMARY KEY,
 provider_kind TEXT NOT NULL CHECK(provider_kind IN ('codex','deepseek_harness','qoder','claude','openclaw','hermes')),
 display_name TEXT NOT NULL CHECK(length(trim(display_name))>0),
 executable_path TEXT,
 provider_route TEXT,
 model TEXT,
 agent_name TEXT,
 provider_config_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(provider_config_json) AND json_type(provider_config_json)='object'),
 enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0,1)),
 connection_state TEXT NOT NULL DEFAULT 'unverified' CHECK(connection_state IN ('unverified','ready','failed')),
 discovered_version TEXT,
 last_verified_at TEXT,
 last_error TEXT,
 verification_thread_id TEXT,
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
```

Add `ProviderKind::Qoder`, `agent_name: Option<String>`, and `provider_config: serde_json::Value` to input, persisted, and frozen types. Validate that Qoder never uses `provider_route`, `agent_name` is not blank when present, and provider config is an object.

- [ ] **Step 4: Update Store SQL and schema bootstrap**

Extend every agent SELECT/INSERT/UPDATE mapper with `agent_name` and `provider_config_json`, parse JSON on reads, invalidate verification when either changes, execute migration 17 after migration 16, and require schema version 17.

- [ ] **Step 5: Run provider and persistence tests**

Run: `cargo test -p archmage-core --test agents --test persistence`

Expected: PASS.

- [ ] **Step 6: Commit the model slice**

```bash
git add crates/archmage-core/migrations/0017_qoder.sql crates/archmage-core/src/agent.rs crates/archmage-core/src/runtime/mod.rs crates/archmage-core/src/store/mod.rs crates/archmage-core/tests/agents.rs crates/archmage-core/tests/persistence.rs
git commit -m "feat: persist Qoder provider instances"
```

### Task 2: Add an ACP-safe workspace client

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/archmage-core/Cargo.toml`
- Create: `crates/archmage-core/src/provider/acp.rs`
- Modify: `crates/archmage-core/src/provider/mod.rs`
- Create: `crates/archmage-core/tests/acp_workspace.rs`

- [ ] **Step 1: Add failing workspace-boundary tests**

Cover canonicalized reads, Coder writes, Reviewer write rejection, symlink escape rejection, terminal cwd escape rejection, and sensitive command rejection:

```rust
let policy = AcpWorkspacePolicy::new(root.path(), AcpAccess::ReadOnly)?;
assert!(policy.read_text(&root.path().join("src/lib.rs"), None, None).await.is_ok());
assert!(policy.write_text(&root.path().join("src/lib.rs"), "changed").await.is_err());
assert!(policy.validate_terminal(root.path(), "cargo", &["test".into()]).is_ok());
assert!(policy.validate_terminal(root.path(), "sudo", &["rm".into(), "-rf".into(), "/".into()]).is_err());
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run: `cargo test -p archmage-core --test acp_workspace`

Expected: compilation failure because the ACP workspace module does not exist.

- [ ] **Step 3: Add the official ACP dependency and focused policy types**

Add the current ACP v1 Rust crate version resolved by Cargo and implement:

```rust
pub enum AcpAccess { ReadOnly, WorkspaceWrite }

pub struct AcpWorkspacePolicy {
    root: PathBuf,
    canonical_root: PathBuf,
    access: AcpAccess,
}

impl AcpWorkspacePolicy {
    pub fn resolve(&self, path: &Path, for_write: bool) -> Result<PathBuf>;
    pub async fn read_text(&self, path: &Path, line: Option<u32>, limit: Option<u32>) -> Result<String>;
    pub async fn write_text(&self, path: &Path, content: &str) -> Result<()>;
    pub fn validate_terminal(&self, cwd: &Path, command: &str, args: &[String]) -> Result<()>;
}
```

Reject paths outside the canonical root, writes in read-only mode, empty commands, privilege escalation commands, and destructive targets that are the worktree, a parent, HOME, root, or an unresolved variable/glob.

- [ ] **Step 4: Run the workspace policy tests**

Run: `cargo test -p archmage-core --test acp_workspace`

Expected: PASS.

- [ ] **Step 5: Commit the ACP policy boundary**

```bash
git add Cargo.toml Cargo.lock crates/archmage-core/Cargo.toml crates/archmage-core/src/provider/acp.rs crates/archmage-core/src/provider/mod.rs crates/archmage-core/tests/acp_workspace.rs
git commit -m "feat: add ACP workspace policy"
```

### Task 3: Implement the Qoder ACP process and protocol lifecycle

**Files:**
- Create: `crates/archmage-core/src/provider/qoder.rs`
- Create: `crates/archmage-core/tests/fixtures/fake_qoder_acp.py`
- Create: `crates/archmage-core/tests/qoder_acp_protocol.rs`
- Modify: `crates/archmage-core/src/provider/mod.rs`
- Modify: `crates/archmage-core/src/process.rs`

- [ ] **Step 1: Write failing fake-process protocol tests**

The fixture accepts `--scenario` and implements ACP initialize, session/new, session/prompt, session/cancel, fs calls, permission requests, and deterministic updates. Add tests for a successful strict JSON turn, invalid protocol version, wrong response ID, oversized final text, cancellation, and unexpected exit:

```rust
let mut session = QoderSession::start(config_for("success")).await?;
let outcome = session.prompt("attempt-1", b"return JSON").await?;
assert_eq!(outcome.result["status"], "implemented");
assert!(!outcome.conversation_id.is_empty());
session.close().await?;
```

- [ ] **Step 2: Run the protocol test and verify it fails**

Run: `cargo test -p archmage-core --test qoder_acp_protocol`

Expected: compilation failure because `QoderSession` does not exist.

- [ ] **Step 3: Implement Qoder inspection and configuration**

Define focused public types:

```rust
pub struct QoderTools { pub binary: PathBuf, pub version: String }

pub struct QoderSessionConfig {
    pub binary: PathBuf,
    pub repository: PathBuf,
    pub model: Option<String>,
    pub agent_name: Option<String>,
    pub provider_config: serde_json::Value,
    pub access: AcpAccess,
    pub diagnostics_dir: PathBuf,
    pub deadline: Duration,
    pub max_line_bytes: usize,
    pub max_stream_bytes: usize,
    pub max_result_bytes: usize,
}

pub async fn inspect_at(path: Option<&Path>) -> Result<QoderTools>;
```

Launch `qoder --acp` with explicit model/Agent flags when configured, `--strict-mcp-config`, and only frozen explicit MCP configuration. Do not pass a bypass-permissions flag.

- [ ] **Step 4: Implement ACP v1 lifecycle using the official crate**

Implement `QoderSession::start`, `prompt`, `cancel`, and `close`. The client must initialize once, create one session, serve permission/fs/terminal callbacks through `AcpWorkspacePolicy`, collect bounded `agent_message_chunk` content, validate exactly one JSON object, map ACP stop reasons, and track child process identity.

- [ ] **Step 5: Run protocol tests**

Run: `cargo test -p archmage-core --test qoder_acp_protocol -- --nocapture`

Expected: PASS for success, malformed protocol, limits, cancel, and process exit scenarios.

- [ ] **Step 6: Commit the Qoder ACP adapter**

```bash
git add crates/archmage-core/src/provider/qoder.rs crates/archmage-core/src/provider/mod.rs crates/archmage-core/src/process.rs crates/archmage-core/tests/fixtures/fake_qoder_acp.py crates/archmage-core/tests/qoder_acp_protocol.rs
git commit -m "feat: add Qoder ACP adapter"
```

### Task 4: Route Qoder sessions through Runtime

**Files:**
- Modify: `crates/archmage-core/src/provider/pool.rs`
- Modify: `crates/archmage-core/src/runtime/dispatch.rs`
- Modify: `crates/archmage-core/src/runtime/flow.rs`
- Modify: `crates/archmage-core/src/runtime/consultation.rs`
- Modify: `crates/archmage-core/src/runtime/mod.rs`
- Modify: `crates/archmage-core/src/store/recovery.rs`
- Modify: `crates/archmage-core/tests/provider_pool.rs`
- Modify: `crates/archmage-core/tests/provider_execution.rs`
- Modify: `crates/archmage-core/tests/flow.rs`
- Modify: `crates/archmage-core/tests/runtime.rs`

- [ ] **Step 1: Write failing pool and Runtime tests**

Add tests proving that the same Run/Capability reuses a Qoder session, different capabilities use different sessions, consultation is temporary, cancellation reaches Qoder, and a Qoder binding freezes agent name/provider config.

- [ ] **Step 2: Run focused Runtime tests and verify failure**

Run: `cargo test -p archmage-core --test provider_pool --test provider_execution --test flow qoder -- --nocapture`

Expected: compilation or assertion failures because the pool and Runtime do not route Qoder.

- [ ] **Step 3: Generalize the provider pool entry**

Replace the DSH-only live entry with an enum:

```rust
enum PersistentProviderSession {
    DeepseekHarness(DeepseekHarnessSidecar),
    Qoder(QoderSession),
}

enum PoolEntry {
    Live(PersistentProviderSession),
    Poisoned(String),
    Closing,
}
```

Expose provider-specific start configurations behind one `invoke_main`, `invoke_temporary`, `cancel_capability`, and `close_run` lifecycle without changing existing DSH semantics.

- [ ] **Step 4: Route all execution modes to Qoder**

Build `QoderSessionConfig` from the frozen binding and Capability sandbox. Route normal nodes, flow nodes, and consultation through the pool. Persist Qoder child identity on the Attempt and keep all existing contract, Artifact, ChangeSet, and delivery code unchanged after `ProviderOutcome` is returned.

- [ ] **Step 5: Implement explicit retry recovery semantics**

Persist the ACP session ID and worktree baseline/hash with the Attempt result metadata. Recovery continues to mark a turn that was running during process loss as `interrupted`. On an explicit retry, call `session/load` only when the Qoder initialize response advertises `loadSession` and the recorded worktree baseline/hash still matches; otherwise start a new ACP session and record that continuity was not restored.

- [ ] **Step 6: Run Runtime tests**

Run: `cargo test -p archmage-core --test provider_pool --test provider_execution --test flow --test consultations --test runtime`

Expected: PASS for Qoder and existing DSH/Codex cases.

- [ ] **Step 7: Commit Runtime integration**

```bash
git add crates/archmage-core/src/provider/pool.rs crates/archmage-core/src/runtime/dispatch.rs crates/archmage-core/src/runtime/flow.rs crates/archmage-core/src/runtime/consultation.rs crates/archmage-core/src/runtime/mod.rs crates/archmage-core/src/store/recovery.rs crates/archmage-core/tests/provider_pool.rs crates/archmage-core/tests/provider_execution.rs crates/archmage-core/tests/flow.rs crates/archmage-core/tests/runtime.rs
git commit -m "feat: run capabilities through Qoder ACP"
```

### Task 5: Add Qoder discovery and real connection verification

**Files:**
- Modify: `crates/archmage-core/src/agent.rs`
- Create: `crates/archmage-core/tests/qoder_live.rs`
- Modify: `crates/archmage-core/tests/agents.rs`

- [ ] **Step 1: Write failing discovery and verification tests**

Use the fake ACP executable to assert discovered version, ready state, verification session ID, safe authentication failure, and invalidation when model/Agent/config changes.

- [ ] **Step 2: Run focused tests and verify failure**

Run: `cargo test -p archmage-core --test agents qoder -- --nocapture`

Expected: failure because AgentService has no Qoder branch.

- [ ] **Step 3: Implement Qoder discovery and verification**

Dispatch Qoder discovery to `qoder::inspect_at`. Verification creates a temporary read-only Git directory, starts ACP with frozen explicit settings, asks for a fixed JSON object, validates it, closes the process, and records ready/version/session ID. Return safe stable errors without environment or credential content.

- [ ] **Step 4: Add ignored live smoke test**

```rust
#[tokio::test]
#[ignore = "requires installed and authenticated Qoder CLI"]
async fn real_qoder_acp_completes_a_strict_json_turn() -> Result<()> {
    let tools = qoder::inspect_at(None).await?;
    let outcome = qoder::readonly_probe(&tools).await?;
    assert!(outcome.result.is_object());
    Ok(())
}
```

- [ ] **Step 5: Run agent tests and compile the ignored live test**

Run: `cargo test -p archmage-core --test agents && cargo test -p archmage-core --test qoder_live --no-run`

Expected: PASS and successful compilation.

- [ ] **Step 6: Commit lifecycle verification**

```bash
git add crates/archmage-core/src/agent.rs crates/archmage-core/tests/agents.rs crates/archmage-core/tests/qoder_live.rs
git commit -m "feat: discover and verify Qoder ACP"
```

### Task 6: Expose Qoder in desktop Agent management

**Files:**
- Modify: `apps/archmage-desktop/src/model.ts`
- Modify: `apps/archmage-desktop/src/model.test.ts`
- Modify: `apps/archmage-desktop/src/AgentManagement.tsx`
- Modify: `apps/archmage-desktop/src/AgentManagement.test.tsx`
- Modify: `apps/archmage-desktop/src/managementLayout.test.ts`
- Modify: `apps/archmage-desktop/src-tauri/src/agent_install.rs`

- [ ] **Step 1: Write failing frontend and install-guidance tests**

Assert Qoder is supported/labeled, its Provider section and form appear, the form submits `agentName` and object `providerConfig`, and installation guidance points to official Qoder CLI instructions.

- [ ] **Step 2: Run focused UI tests and verify failure**

Run: `npm test -- --run src/model.test.ts src/AgentManagement.test.tsx src/managementLayout.test.ts`

Working directory: `apps/archmage-desktop`

Expected: FAIL because Qoder is absent from TypeScript models and sections.

- [ ] **Step 3: Extend TypeScript models and Qoder form**

Add `"qoder"` to `ProviderKind`, include `agentName` and `providerConfig` in instance/input/frozen binding types, treat Qoder as supported, and add the section:

```ts
{ kind: "qoder", description: "本地 ACP Agent 连接", addLabel: "添加 Qoder" }
```

For Qoder cards show executable path, optional model, optional Agent name, and an explicit provider-config JSON textarea defaulting to `{ "mcpServers": [] }`. Parse only a JSON object; disable save and show an inline validation message for invalid input.

- [ ] **Step 4: Add Qoder install guidance**

Resolve Qoder for macOS, Linux, and Windows using the official installation command/documentation, and detect it with `qoder::find_executable` or `qoder::inspect_at` without a model call.

- [ ] **Step 5: Run UI and Tauri unit tests**

Run: `npm test -- --run src/model.test.ts src/AgentManagement.test.tsx src/managementLayout.test.ts && cargo test -p archmage-desktop agent_install`

Expected: PASS.

- [ ] **Step 6: Commit desktop support**

```bash
git add apps/archmage-desktop/src/model.ts apps/archmage-desktop/src/model.test.ts apps/archmage-desktop/src/AgentManagement.tsx apps/archmage-desktop/src/AgentManagement.test.tsx apps/archmage-desktop/src/managementLayout.test.ts apps/archmage-desktop/src-tauri/src/agent_install.rs
git commit -m "feat: manage Qoder provider instances"
```

### Task 7: Document Qoder provider operation

**Files:**
- Create: `docs/en/runbooks/qoder-provider.md`
- Create: `docs/zh-CN/runbooks/qoder-provider.md`
- Modify: `docs/en/architecture/cross-provider-handoff.md`
- Modify: `docs/zh-CN/architecture/cross-provider-handoff.md`
- Modify: `README.en.md`
- Modify: `README.md`

- [ ] **Step 1: Write the bilingual runbook**

Document installation, CLI login/PAT boundary, instance configuration, ACP v1 requirements, strict JSON output, `Run + Capability` session isolation, permission behavior, verification, cancellation, recovery, diagnostics, and the ignored live smoke command.

- [ ] **Step 2: Update architecture and README links**

Add Qoder ACP to the provider transport table and link the new runbook from both README variants without changing Codex or DSH instructions.

- [ ] **Step 3: Check bilingual link symmetry**

Run: `rg -n "qoder-provider|Qoder ACP" README.md README.en.md docs/en docs/zh-CN`

Expected: matching English and Chinese entry points and no broken relative path in the changed Markdown.

- [ ] **Step 4: Commit documentation**

```bash
git add README.md README.en.md docs/en/runbooks/qoder-provider.md docs/zh-CN/runbooks/qoder-provider.md docs/en/architecture/cross-provider-handoff.md docs/zh-CN/architecture/cross-provider-handoff.md
git commit -m "docs: add Qoder provider runbook"
```

### Task 8: Complete regression and live verification

**Files:**
- Modify only files required to fix regressions introduced by Tasks 1-7.

- [ ] **Step 1: Format and lint**

Run: `cargo fmt --all --check`

Expected: PASS. If it fails, run `cargo fmt --all`, inspect the diff, and re-run the check.

Run: `cargo clippy --workspace --all-targets -- -D warnings`

Expected: PASS.

- [ ] **Step 2: Run the full Rust test suite**

Run: `cargo test --workspace`

Expected: PASS with live tests reported ignored.

- [ ] **Step 3: Run the desktop tests and build**

Working directory: `apps/archmage-desktop`

Run: `npm test -- --run && npm run build`

Expected: all Vitest tests pass and the Vite/TypeScript production build succeeds.

- [ ] **Step 4: Run the real Qoder smoke test when authentication is available**

Run: `cargo test -p archmage-core --test qoder_live -- --ignored --nocapture`

Expected: PASS on the installed authenticated Qoder CLI. If Qoder authentication, network, or quota is unavailable, record the exact missing prerequisite and do not replace this result with the fake ACP test.

- [ ] **Step 5: Review the final diff and commit regression fixes**

Run: `git diff --check && git status --short && git log --oneline --decorate -10`

Expected: no whitespace errors and only intended Qoder changes. Commit any verification-only fixes with:

```bash
git add <specific-files>
git commit -m "fix: complete Qoder provider verification"
```
