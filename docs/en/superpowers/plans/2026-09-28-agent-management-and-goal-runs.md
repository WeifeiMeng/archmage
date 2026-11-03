# Agent Management and Goal-Driven Runs Implementation Plan

[简体中文](../../../zh-CN/superpowers/plans/2026-09-28-agent-management-and-goal-runs.md) | English

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a real local Codex Agent management page with durable capability bindings, and let users start a Run with only a repository and goal while Tester autonomously performs validation.

**Architecture:** `archmage-core` owns a persisted Agent-instance registry, global capability bindings, real Codex discovery/probing, and immutable per-Run binding snapshots. Tauri exposes typed commands to React; the workbench adds an Agents destination and removes predeclared checks from new Runs without deleting legacy CheckRunner infrastructure.

**Tech Stack:** Rust 1.95, Tokio, rusqlite/SQLite, real Codex CLI, Tauri 2, React 19, TypeScript, Vite, Vitest.

---

## File structure

```text
crates/archmage-core/
  migrations/0008_agents.sql            # Agent instances and global bindings
  src/agent.rs                          # Agent DTOs, validation, discovery and verification
  src/provider/codex.rs                 # configured executable/model support
  src/runtime/mod.rs                    # goal-only RunRequest and frozen bindings
  src/runtime/dispatch.rs               # per-node frozen Agent dispatch; autonomous Tester
  src/contract.rs                       # Tester v2 and Reviewer evidence rules
  capabilities/v1/tester.md             # autonomous test task prompt
  capabilities/v1/tester.schema.json    # command/outcome evidence contract
  capabilities/v1/reviewer.md           # consume autonomous Tester evidence
  src/store/mod.rs                      # Agent/binding persistence queries
  tests/agents.rs                       # migration, multi-instance and binding tests
  tests/runtime.rs                      # goal-only and frozen binding tests
  tests/contracts.rs                    # Tester/Reviewer contract tests
  tests/codex_live.rs                   # ignored real verification path
apps/archmage-desktop/
  src-tauri/src/lib.rs                  # Agent management Tauri commands
  src/model.ts                          # goal-only Run and Agent DTOs
  src/api.ts                            # typed Agent invokes
  src/AgentManagement.tsx               # connections and capability routing page
  src/App.tsx                           # Runs/Agents navigation and simplified form
  src/styles.css                        # Agent page and navigation styles
  src/model.test.ts                     # request and status presentation tests
docs/guide.md                           # Agent setup and Run usage
docs/runbooks/desktop-s0.md             # real connection verification steps
```

### Task 1: Persist Agent instances and capability bindings

**Files:**
- Create: `crates/archmage-core/migrations/0008_agents.sql`
- Create: `crates/archmage-core/src/agent.rs`
- Create: `crates/archmage-core/tests/agents.rs`
- Modify: `crates/archmage-core/src/lib.rs`
- Modify: `crates/archmage-core/src/store/mod.rs`

- [ ] **Step 1: Write the failing migration and Store test**

```rust
#[test]
fn seeds_one_codex_instance_and_binds_all_builtin_capabilities() {
    let store = fixture_store();
    let snapshot = store.agent_management_snapshot().unwrap();
    assert_eq!(snapshot.instances.len(), 1);
    assert_eq!(snapshot.instances[0].id, "codex-default");
    assert_eq!(snapshot.bindings.len(), 4);
    assert!(snapshot.bindings.iter().all(|binding|
        binding.agent_instance_id == "codex-default"));
}

#[test]
fn stores_multiple_codex_instances_with_stable_ids() {
    let mut store = fixture_store();
    store.save_agent_instance(&codex_input("codex-work", "Codex Work")).unwrap();
    assert_eq!(store.agent_instances().unwrap().len(), 2);
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test -p archmage-core --test agents`

Expected: compilation fails because migration v8 and Agent Store APIs do not exist.

- [ ] **Step 3: Add migration and typed records**

Create `agent_instances` with `id`, `provider_kind`, `display_name`, nullable `executable_path` and `model`, `enabled`, connectivity fields, timestamps, and JSON-safe constraints. Create `capability_bindings` keyed by the four built-in capability keys and referencing `agent_instances(id)`. Seed `codex-default` plus four bindings in the migration.

Define these public types in `agent.rs`:

```rust
pub enum ProviderKind { Codex, Claude, OpenClaw, Hermes }
pub enum ConnectionState { Unverified, Ready, Failed }
pub struct AgentInstance { /* persisted safe fields */ }
pub struct CapabilityBinding { pub capability: String, pub agent_instance_id: String }
pub struct AdapterDescriptor { pub kind: ProviderKind, pub supported: bool }
pub struct AgentManagementSnapshot { pub adapters: Vec<AdapterDescriptor>, pub instances: Vec<AgentInstance>, pub bindings: Vec<CapabilityBinding> }
```

Update Store migration sequencing to require schema version 8 and add CRUD/query methods. Reject binding an unknown, disabled, or unsupported instance. Reject disabling a currently bound instance.

- [ ] **Step 4: Run GREEN**

Run: `cargo test -p archmage-core --test agents`

Expected: both tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/archmage-core/migrations/0008_agents.sql crates/archmage-core/src/agent.rs crates/archmage-core/src/lib.rs crates/archmage-core/src/store/mod.rs crates/archmage-core/tests/agents.rs
git commit -m "feat: persist agent instances and bindings"
```

### Task 2: Configure and verify the real Codex instance

**Files:**
- Modify: `crates/archmage-core/src/provider/codex.rs`
- Modify: `crates/archmage-core/src/agent.rs`
- Modify: `crates/archmage-core/tests/agents.rs`
- Modify: `crates/archmage-core/tests/codex_protocol.rs`
- Modify: `crates/archmage-core/tests/codex_live.rs`

- [ ] **Step 1: Write failing executable-resolution tests**

```rust
#[tokio::test]
async fn discovery_uses_the_instances_explicit_executable() {
    let binary = fixture_codex_binary("codex 1.2.3", valid_exec_help());
    let tools = codex::inspect_at(Some(&binary)).await.unwrap();
    assert_eq!(tools.codex_binary, binary);
    assert_eq!(tools.codex_version, "codex 1.2.3");
}

#[test]
fn changing_execution_config_invalidates_verification() {
    let mut store = fixture_store();
    store.record_agent_ready("codex-default", "codex 1.2.3", "thread-1").unwrap();
    store.save_agent_instance(&changed_path_input()).unwrap();
    assert_eq!(store.agent_instance("codex-default").unwrap().connection_state,
               ConnectionState::Unverified);
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test -p archmage-core --test agents --test codex_protocol`

Expected: failure because configured discovery and persisted verification do not exist.

- [ ] **Step 3: Implement discovery and real verification**

Add `codex::inspect_at(Option<&Path>)`; validate an explicit executable without PATH lookup and keep the existing `inspect()` as the default wrapper. Add optional model to `CodexInvocation` and emit `--model <value>` only when configured.

Implement `AgentService::discover(id)` and `AgentService::verify(id)`. `verify` runs `readonly_probe_logged`, stores ready/failed state with a sanitized message, and returns the refreshed management snapshot. Unsupported Provider kinds return a typed preflight error and never execute a fallback.

- [ ] **Step 4: Run GREEN and the opt-in real probe**

Run: `cargo test -p archmage-core --test agents --test codex_protocol`

Expected: all non-network tests pass.

Run when local Codex authentication is available: `cargo test -p archmage-core --test codex_live real_readonly_probe -- --ignored --nocapture`

Expected: one ignored real probe test passes with a nonblank thread ID.

- [ ] **Step 5: Commit**

```bash
git add crates/archmage-core/src/provider/codex.rs crates/archmage-core/src/agent.rs crates/archmage-core/tests/agents.rs crates/archmage-core/tests/codex_protocol.rs crates/archmage-core/tests/codex_live.rs
git commit -m "feat: verify configured Codex agents"
```

### Task 3: Freeze global capability bindings into goal-only Runs

**Files:**
- Modify: `crates/archmage-core/src/runtime/mod.rs`
- Modify: `crates/archmage-core/src/runtime/dispatch.rs`
- Modify: `crates/archmage-core/tests/runtime.rs`

- [ ] **Step 1: Write failing goal-only and frozen-binding tests**

```rust
#[test]
fn repository_and_goal_are_sufficient_to_define_a_run() {
    let request = RunRequest::new(repo(), "add health endpoint").unwrap();
    let run_id = runtime().create_run_definition(&request).unwrap();
    assert!(runtime().snapshot(&run_id).unwrap().agent_bindings.len() == 4);
}

#[test]
fn changing_global_binding_does_not_change_existing_run() {
    let run_id = runtime().create_run_definition(&RunRequest::new(repo(), "goal").unwrap()).unwrap();
    bind("tester", "codex-work");
    assert_eq!(runtime().snapshot(&run_id).unwrap().agent_bindings["tester"].agent_instance_id,
               "codex-default");
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test -p archmage-core --test runtime repository_and_goal_are_sufficient_to_define_a_run changing_global_binding_does_not_change_existing_run`

Expected: compilation fails because `RunRequest` still requires checks and snapshots lack Agent bindings.

- [ ] **Step 3: Implement the new Run definition**

Remove `checks` from new `RunRequest` and `RunSnapshot`. Replace `FrozenRunDefinition.checks` with `agent_bindings: BTreeMap<String, FrozenAgentBinding>`. Resolve and validate all four bindings before persisting the Run. Keep backward-compatible deserialization for definitions containing legacy `checks` by ignoring that field.

Change dispatch to resolve Codex tools and model from each node's frozen binding rather than calling one global `codex::inspect()` for the whole Run.

- [ ] **Step 4: Run GREEN**

Run: `cargo test -p archmage-core --test runtime`

Expected: all Runtime tests pass with goal-only requests.

- [ ] **Step 5: Commit**

```bash
git add crates/archmage-core/src/runtime/mod.rs crates/archmage-core/src/runtime/dispatch.rs crates/archmage-core/tests/runtime.rs
git commit -m "feat: freeze agent bindings into goal driven runs"
```

### Task 4: Give Tester an autonomous task instead of frozen checks

**Files:**
- Modify: `crates/archmage-core/capabilities/v1/tester.md`
- Modify: `crates/archmage-core/capabilities/v1/tester.schema.json`
- Modify: `crates/archmage-core/capabilities/v1/reviewer.md`
- Modify: `crates/archmage-core/src/contract.rs`
- Modify: `crates/archmage-core/src/runtime/dispatch.rs`
- Modify: `crates/archmage-core/tests/contracts.rs`
- Modify: `crates/archmage-core/tests/runtime.rs`

- [ ] **Step 1: Write failing Tester contract tests**

```rust
#[test]
fn tester_reports_commands_it_selected_without_required_check_ids() {
    let output = json!({
        "schema_version": "1",
        "summary_markdown": "Tests passed",
        "status": "passed",
        "commands": [{"command":"cargo test","exit_code":0,"status":"passed","summary":"12 passed"}],
        "limitations": []
    });
    assert!(validate_output(CapabilityKind::Tester, output.to_string().as_bytes(), &Default::default()).is_ok());
}

#[test]
fn reviewer_cannot_approve_failed_or_blocked_testing() {
    let context = ContractContext { tester_accepted: false };
    assert!(validate_output(CapabilityKind::Reviewer, approved_review(), &context).is_err());
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test -p archmage-core --test contracts tester_reports_commands_it_selected_without_required_check_ids reviewer_cannot_approve_failed_or_blocked_testing`

Expected: Tester schema rejects autonomous command results and `tester_accepted` does not exist.

- [ ] **Step 3: Implement autonomous Tester dispatch**

Update the Tester prompt and schema to require `status`, chosen `commands`, and `limitations`. Remove `invoke_tester`, Unix socket injection, frozen check lookup, and required-check contract assembly from built-in dispatch. Invoke Tester through the normal workspace-write Codex path.

Replace `ContractContext.required_checks/checks_failed` with `tester_accepted` for Reviewer validation. Derive it from the persisted Tester result status. Keep `check_runner.rs`, the internal CLI helper, and migration v5 untouched as dormant lower-level infrastructure.

- [ ] **Step 4: Run GREEN**

Run: `cargo test -p archmage-core --test contracts --test runtime --test checks`

Expected: contracts and Runtime pass; legacy CheckRunner unit tests remain green.

- [ ] **Step 5: Commit**

```bash
git add crates/archmage-core/capabilities/v1/tester.md crates/archmage-core/capabilities/v1/tester.schema.json crates/archmage-core/capabilities/v1/reviewer.md crates/archmage-core/src/contract.rs crates/archmage-core/src/runtime/dispatch.rs crates/archmage-core/tests/contracts.rs crates/archmage-core/tests/runtime.rs
git commit -m "feat: delegate testing to the bound agent"
```

### Task 5: Expose Agent management through Tauri

**Files:**
- Modify: `apps/archmage-desktop/src-tauri/src/lib.rs`
- Modify: `apps/archmage-desktop/src/api.ts`
- Modify: `apps/archmage-desktop/src/model.ts`

- [ ] **Step 1: Add typed frontend contracts before commands**

```ts
export type AgentManagementSnapshot = {
  adapters: AdapterDescriptor[];
  instances: AgentInstance[];
  bindings: CapabilityBinding[];
};

export type RunRequest = { repository: string; goal: string };
```

Run: `npm test -- --run` in `apps/archmage-desktop`.

Expected: existing request serialization test fails because it expects checks.

- [ ] **Step 2: Add Tauri commands**

Expose `get_agent_management`, `save_agent_instance`, `verify_agent_instance`, and `bind_capability`. Commands call Core only, return safe DTOs, and never expose probe stderr. Change `start_run` to accept the goal-only request.

- [ ] **Step 3: Add typed API methods**

```ts
getAgentManagement: () => invoke<AgentManagementSnapshot>("get_agent_management"),
saveAgentInstance: (input) => invoke<AgentManagementSnapshot>("save_agent_instance", { input }),
verifyAgentInstance: (agentInstanceId) => invoke<AgentManagementSnapshot>("verify_agent_instance", { agentInstanceId }),
bindCapability: (capability, agentInstanceId) => invoke<AgentManagementSnapshot>("bind_capability", { capability, agentInstanceId }),
```

- [ ] **Step 4: Compile**

Run: `cargo check -p archmage-desktop`

Expected: Tauri command layer compiles.

- [ ] **Step 5: Commit**

```bash
git add apps/archmage-desktop/src-tauri/src/lib.rs apps/archmage-desktop/src/api.ts apps/archmage-desktop/src/model.ts
git commit -m "feat: expose agent management to desktop"
```

### Task 6: Build the Agent management page and simplify New Run

**Files:**
- Create: `apps/archmage-desktop/src/AgentManagement.tsx`
- Modify: `apps/archmage-desktop/src/App.tsx`
- Modify: `apps/archmage-desktop/src/model.ts`
- Modify: `apps/archmage-desktop/src/model.test.ts`
- Modify: `apps/archmage-desktop/src/styles.css`

- [ ] **Step 1: Write failing model tests**

```ts
it("serializes a run from repository and goal only", () => {
  expect(toRunRequest(" /repo ", " goal ")).toEqual({ repository: "/repo", goal: "goal" });
});

it("does not call an unverified Codex connection ready", () => {
  expect(agentStatusLabel({ connectionState: "unverified" })).toBe("连接未验证");
});
```

- [ ] **Step 2: Run RED**

Run: `npm test -- --run` in `apps/archmage-desktop`.

Expected: request signature and status helper tests fail.

- [ ] **Step 3: Implement pure model behavior**

Remove CheckSpec from frontend Run DTOs. Implement goal-only `toRunRequest`, safe connection labels, and helpers that summarize capability binding readiness.

- [ ] **Step 4: Run GREEN for model tests**

Run: `npm test -- --run` in `apps/archmage-desktop`.

Expected: frontend tests pass.

- [ ] **Step 5: Implement the page**

Add Runs/Agents sidebar navigation. Build `AgentManagement` with the Codex configuration card, online verification action, unsupported Adapter cards, capability binding selectors, and an advanced additional-Codex-instance form. Remove check-command state and controls from New Run. Show frozen Agent identity on each Run node.

- [ ] **Step 6: Build and commit**

Run: `npm run build` in `apps/archmage-desktop`.

Expected: TypeScript and Vite complete with exit code 0.

```bash
git add apps/archmage-desktop/src/AgentManagement.tsx apps/archmage-desktop/src/App.tsx apps/archmage-desktop/src/model.ts apps/archmage-desktop/src/model.test.ts apps/archmage-desktop/src/styles.css
git commit -m "feat: add local Agent management page"
```

### Task 7: Update usage documentation and run final verification

**Files:**
- Modify: `docs/guide.md`
- Modify: `docs/runbooks/desktop-s0.md`
- Modify: `docs/architecture/runtime-and-state.md`

- [ ] **Step 1: Document the real workflow**

Document `./scripts/dev-desktop.sh`, opening Agents, verifying the real Codex connection, reading capability bindings, adding another Codex instance, and starting a Run with repository plus goal. State explicitly that Claude, OpenClaw, and Hermes are not implemented and that Tester chooses its own validation commands.

- [ ] **Step 2: Run focused verification**

Run:

```bash
cargo test -p archmage-core --test agents --test runtime --test contracts --test codex_protocol --test checks
(cd apps/archmage-desktop && npm test -- --run && npm run build)
cargo check -p archmage-desktop
```

Expected: all commands exit 0.

- [ ] **Step 3: Run workspace regression verification**

Run: `cargo test --workspace`

Expected: all non-ignored workspace tests pass.

- [ ] **Step 4: Run real Codex verification when credentials are available**

Run: `cargo test -p archmage-core --test codex_live real_readonly_probe -- --ignored --nocapture`

Expected: the real probe completes with exit code 0 and a nonblank thread ID. If unavailable, report it separately rather than claiming online verification.

- [ ] **Step 5: Self-review against the approved design and commit docs**

Confirm that Run has no check-command input, all four bindings are frozen, unsupported Providers are truthful, no per-Run override exists, and no Fake Agent was added.

```bash
git add docs/guide.md docs/runbooks/desktop-s0.md docs/architecture/runtime-and-state.md docs/superpowers/specs/2026-09-28-agent-management-and-goal-runs-design.md docs/superpowers/plans/2026-09-28-agent-management-and-goal-runs.md
git commit -m "docs: explain Agent management and goal driven runs"
```
