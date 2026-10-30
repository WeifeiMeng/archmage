# DeepSeek Harness Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a real DeepSeek Harness Provider whose Archmage capabilities can be configured, verified, executed, cancelled, and resumed in one DSH conversation per Run and Capability.

**Architecture:** Extend `dsh --profile headless` with explicit create/resume session arguments while retaining one process per task. In Archmage, add a `deepseek_harness` adapter behind a provider-neutral invocation API and persist one guarded DSH session mapping per Run and Capability; consultations use temporary sessions. Existing contracts, worktrees, artifacts, and process-group cancellation remain authoritative.

**Tech Stack:** TypeScript/Vitest/Cordis in DeepSeek Harness; Rust/Tokio/Rusqlite in Archmage Core; React/TypeScript/Vitest/Tauri in Archmage Desktop.

---

## File structure

DeepSeek Harness repository (`/Users/mengweifei/Desktop/code/github/deepseek-harness`):

- `packages/bundle/headless/src/startup.ts`: parse mutually exclusive create/resume session options.
- `packages/bundle/headless/src/index.ts`: create or resume the requested Agent and drive one owned turn.
- `packages/bundle/headless/tests/{startup,headless}.spec.ts`: focused session-mode behavior.
- `packages/bundle/headless/README.md`, `README.zh.md`: public CLI and persistence semantics.
- `.agents/notes/implemented/feature/2026-10-30-headless-session-resume.{md,zh.md}` plus `.i18n.yaml`: required decision record.

Archmage repository (`/Users/mengweifei/Desktop/code/github/archmage`):

- `crates/archmage-core/migrations/0016_deepseek_harness.sql`: Provider constraint, seeded instance, and guarded Run/Capability session table.
- `crates/archmage-core/src/provider/{mod.rs,deepseek_harness.rs,executor.rs}`: DSH protocol plus provider-neutral dispatch.
- `crates/archmage-core/src/{agent.rs,store/mod.rs,runtime/{mod.rs,dispatch.rs,consultation.rs,flow.rs}}`: management, frozen bindings, session ownership, and all runtime routes.
- `crates/archmage-core/tests/{agents.rs,deepseek_harness_protocol.rs,runtime.rs,consultations.rs,flow.rs}`: narrow persistence and routing coverage.
- `apps/archmage-desktop/src/{model.ts,AgentManagement.tsx,CapabilityManagement.tsx,managementLayout.ts}` and focused tests: supported Provider UI.
- `docs/runbooks/desktop-s0.md`: current Provider behavior.

### Task 1: Add explicit headless session creation and resume to DeepSeek Harness

**Files:**
- Modify: `/Users/mengweifei/Desktop/code/github/deepseek-harness/packages/bundle/headless/src/startup.ts`
- Modify: `/Users/mengweifei/Desktop/code/github/deepseek-harness/packages/bundle/headless/src/index.ts`
- Modify: `/Users/mengweifei/Desktop/code/github/deepseek-harness/packages/bundle/headless/cordis.patch.yml`
- Test: `/Users/mengweifei/Desktop/code/github/deepseek-harness/packages/bundle/headless/tests/startup.spec.ts`
- Test: `/Users/mengweifei/Desktop/code/github/deepseek-harness/packages/bundle/headless/tests/headless.spec.ts`

- [ ] **Step 1: Write failing startup parser tests**

Assert these resolved values and errors:

```ts
expect((await bootStartup(['--session-id', 'run-designer', 'first task'])).task)
  .toEqual({ task: 'first task', session: { mode: 'create', id: 'run-designer' } })
expect((await bootStartup(['--resume-session-id', 'run-designer', 'next task'])).task)
  .toEqual({ task: 'next task', session: { mode: 'resume', id: 'run-designer' } })
expect((await bootStartup(['--session-id', 'a', '--resume-session-id', 'b', 'task'])).observed.out)
  .toContain('mutually exclusive')
```

- [ ] **Step 2: Run the parser test and confirm it fails**

Run: `pnpm exec vitest run packages/bundle/headless/tests/startup.spec.ts`

Expected: FAIL because `HeadlessStartupValues` has no session mode.

- [ ] **Step 3: Parse and publish the session mode**

Use one shared type in `startup.ts`:

```ts
export type HeadlessSession =
  | { mode: 'fresh' }
  | { mode: 'create'; id: SessionId }
  | { mode: 'resume'; id: SessionId }

export interface HeadlessStartupValues {
  task: string
  session: HeadlessSession
}
```

Add Commander options `--session-id <id>` and `--resume-session-id <id>`, reject blank IDs and simultaneous options, and retain `{ mode: 'fresh' }` when neither is supplied. Forward both fields through `cordis.patch.yml` into runner config.

- [ ] **Step 4: Write failing runner tests for create and resume**

Extend the scripted factory to record calls and assert:

```ts
expect(calls).toEqual([{ method: 'create', sessionId: 'run-designer' }])
expect(calls).toEqual([{ method: 'resume', resumeSessionId: 'run-designer' }])
```

Also assert the existing no-option path still creates `session-<uuid>`.

- [ ] **Step 5: Implement runner selection**

Change runner config to `{ task, session }`. Resolve the handle as follows, using the same model-selection setup for both methods:

```ts
const handle = config.session.mode === 'resume'
  ? await agents.resume({ resumeSessionId: config.session.id, agentOptions, setup })
  : await agents.create({
      sessionId: config.session.mode === 'create' ? config.session.id : SessionId(`session-${randomUUID()}`),
      meta: { cwd: process.cwd() }, agentOptions, setup,
    })
```

Keep the existing idle interval, flush-before-output, exit codes, and final-text selection unchanged.

- [ ] **Step 6: Run focused DeepSeek Harness tests**

Run: `pnpm exec vitest run packages/bundle/headless/tests/startup.spec.ts packages/bundle/headless/tests/headless.spec.ts`

Expected: PASS.

- [ ] **Step 7: Commit the executable behavior**

```bash
git -C /Users/mengweifei/Desktop/code/github/deepseek-harness add packages/bundle/headless
git -C /Users/mengweifei/Desktop/code/github/deepseek-harness commit -m "feat(headless): support persistent session turns"
```

### Task 2: Document the DeepSeek Harness session contract

**Files:**
- Modify: `/Users/mengweifei/Desktop/code/github/deepseek-harness/packages/bundle/headless/README.md`
- Modify: `/Users/mengweifei/Desktop/code/github/deepseek-harness/packages/bundle/headless/README.zh.md`
- Create: `/Users/mengweifei/Desktop/code/github/deepseek-harness/.agents/notes/implemented/feature/2026-10-30-headless-session-resume.md`
- Create: `/Users/mengweifei/Desktop/code/github/deepseek-harness/.agents/notes/implemented/feature/2026-10-30-headless-session-resume.zh.md`
- Create: `/Users/mengweifei/Desktop/code/github/deepseek-harness/.agents/notes/implemented/feature/2026-10-30-headless-session-resume.i18n.yaml`

- [ ] **Step 1: Update the bilingual public contract**

Document exactly:

```text
--session-id <id>         create only; fail when the persisted ID exists
--resume-session-id <id>  resume only; fail when the persisted ID is absent or invalid
no option                 preserve the current fresh random-session behavior
```

State that every invocation submits one turn, flushes it, prints only that interval's final assistant text, and exits.

- [ ] **Step 2: Add the required implemented Agent Note triplet**

Use the required sections `Problem`, `Decision`, `Alternatives considered`, and `Consequences`. Record that explicit create/resume beat create-or-resume because callers must not silently replace lost history; record ACP/JSON-RPC as the heavier rejected alternative.

- [ ] **Step 3: Run documentation gates relevant to the changed files**

Run: `pnpm run verify-agent-note-format && pnpm run verify-translation-pairing`

Expected: PASS.

- [ ] **Step 4: Commit the documentation**

```bash
git -C /Users/mengweifei/Desktop/code/github/deepseek-harness add packages/bundle/headless/README.md packages/bundle/headless/README.zh.md .agents/notes/implemented/feature/2026-10-30-headless-session-resume.*
git -C /Users/mengweifei/Desktop/code/github/deepseek-harness commit -m "docs(headless): define session resume semantics"
```

### Task 3: Add DeepSeek Harness Provider data and session ownership to Archmage

**Files:**
- Create: `crates/archmage-core/migrations/0016_deepseek_harness.sql`
- Modify: `crates/archmage-core/src/store/mod.rs`
- Modify: `crates/archmage-core/src/agent.rs`
- Test: `crates/archmage-core/tests/agents.rs`

- [ ] **Step 1: Write failing migration and session-state tests**

Cover the seeded unbound instance, unchanged Codex bindings, and this transition:

```rust
let lease = store.begin_provider_session(run_id, "coder", ProviderKind::DeepseekHarness)?;
assert!(matches!(lease.mode, ProviderSessionMode::Create));
store.mark_provider_session_ready(run_id, "coder", &lease.session_id)?;
assert!(matches!(store.begin_provider_session(run_id, "coder", ProviderKind::DeepseekHarness)?.mode,
                 ProviderSessionMode::Resume));
```

Also prove an interrupted create remains `uncertain` and cannot begin again.

- [ ] **Step 2: Run the test and confirm it fails**

Run: `cargo test -p archmage-core --test agents`

Expected: FAIL because schema 16 and `DeepseekHarness` do not exist.

- [ ] **Step 3: Add schema 16**

Rebuild `agent_instances` and `capability_bindings` inside the migration so the Provider check includes `deepseek_harness`, preserving every existing row and foreign key. Insert:

```sql
INSERT INTO agent_instances(id,provider_kind,display_name,model)
VALUES('deepseek-harness-default','deepseek_harness','DeepSeek Harness',NULL);

CREATE TABLE provider_sessions(
 run_id TEXT NOT NULL REFERENCES runs(id),
 capability TEXT NOT NULL CHECK(capability IN ('designer','coder','tester','reviewer')),
 provider_kind TEXT NOT NULL CHECK(provider_kind='deepseek_harness'),
 session_id TEXT NOT NULL UNIQUE,
 state TEXT NOT NULL CHECK(state IN ('reserved','ready','uncertain')),
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 PRIMARY KEY(run_id,capability)
);
```

Register migration 16 in both fresh and upgrade paths and update the expected schema version.

- [ ] **Step 4: Add Provider and guarded Store methods**

Add `ProviderKind::DeepseekHarness`, string form `deepseek_harness`, and `supported() == true`. Reject a non-null model for this Provider. Implement one immediate transaction that inserts `reserved`, changes `reserved -> uncertain` before returning `Create`, returns `Resume` for `ready`, and rejects `uncertain`; `mark_provider_session_ready` changes only the matching `uncertain` row to `ready`.

- [ ] **Step 5: Run the Store tests**

Run: `cargo test -p archmage-core --test agents`

Expected: PASS.

- [ ] **Step 6: Commit persistence and domain changes**

```bash
git add crates/archmage-core/migrations/0016_deepseek_harness.sql crates/archmage-core/src/store/mod.rs crates/archmage-core/src/agent.rs crates/archmage-core/tests/agents.rs
git commit -m "feat: persist DeepSeek Harness agents and sessions"
```

### Task 4: Implement the DeepSeek Harness adapter and provider-neutral executor

**Files:**
- Create: `crates/archmage-core/src/provider/deepseek_harness.rs`
- Create: `crates/archmage-core/src/provider/executor.rs`
- Modify: `crates/archmage-core/src/provider/mod.rs`
- Test: `crates/archmage-core/tests/deepseek_harness_protocol.rs`

- [ ] **Step 1: Write protocol tests with an executable fake `dsh`**

Assert discovery calls `--version` and `--profile headless --help`; create and resume argv are respectively:

```rust
["--profile", "headless", "--session-id", session_id, prompt]
["--profile", "headless", "--resume-session-id", session_id, prompt]
```

Assert code 0 plus one JSON object succeeds, while prose-wrapped JSON, nonzero exit, and oversized output fail.

- [ ] **Step 2: Run the protocol test and confirm it fails**

Run: `cargo test -p archmage-core --test deepseek_harness_protocol`

Expected: FAIL because the modules do not exist.

- [ ] **Step 3: Implement inspection and invocation**

Use the existing bounded process capture and process-group identity callback. Set `DSH_PERMISSION_MODE` to `read-only` or `workspace-write` for the child while inheriting all other environment, including `DSH_HOME`. Persist stdout, stderr, argv, version, exit and capture error under the invocation diagnostics directory with private permissions.

Expose provider-neutral types:

```rust
pub enum ProviderSession { None, Create(String), Resume(String) }
pub struct ProviderRequest<'a> { /* binding, cwd, prompt, schema, sandbox, limits, session */ }
pub struct ProviderOutcome { pub conversation_id: String, pub result: serde_json::Value }
pub enum ProviderLifecycle<'a> { Spawned(&'a ProcessIdentity), ConversationPersisted }
```

The DSH prompt appends the exact schema plus the instruction to return one JSON object without Markdown. The executor accepts one `FnMut(ProviderLifecycle)`: emit `Spawned` after process creation and emit `ConversationPersisted` after exit code 0 but before JSON parsing. This lets Runtime mark the session resumable even when the completed assistant turn violates Archmage's JSON contract. Parse the complete trimmed stdout with `serde_json::from_slice`; do not scan for braces or code fences.

- [ ] **Step 4: Preserve Codex through the same executor**

Move the current Codex invocation construction into `executor.rs`. Return the Codex thread ID as `conversation_id`; return the supplied DSH session ID for DSH. Keep Codex model override behavior and reject a DSH model override.

- [ ] **Step 5: Run focused protocol tests**

Run: `cargo test -p archmage-core --test codex_protocol --test deepseek_harness_protocol`

Expected: PASS.

- [ ] **Step 6: Commit the adapters**

```bash
git add crates/archmage-core/src/provider crates/archmage-core/tests/deepseek_harness_protocol.rs
git commit -m "feat: execute DeepSeek Harness providers"
```

### Task 5: Route management and every Runtime path through the new executor

**Files:**
- Modify: `crates/archmage-core/src/agent.rs`
- Modify: `crates/archmage-core/src/runtime/mod.rs`
- Modify: `crates/archmage-core/src/runtime/dispatch.rs`
- Modify: `crates/archmage-core/src/runtime/consultation.rs`
- Modify: `crates/archmage-core/src/runtime/flow.rs`
- Test: `crates/archmage-core/tests/runtime.rs`
- Test: `crates/archmage-core/tests/consultations.rs`
- Test: `crates/archmage-core/tests/flow.rs`

- [ ] **Step 1: Write narrow failing routing tests**

Use fake executables to prove: a DSH-bound Capability reaches DSH; two successful attempts for the same Run/Capability use create then resume; another Capability gets a different ID; consultation gets a temporary create ID and does not touch `provider_sessions`; flow delivery reuse matches ordinary dispatch.

- [ ] **Step 2: Run the focused tests and confirm they fail**

Run: `cargo test -p archmage-core --test runtime --test consultations --test flow`

Expected: FAIL on unsupported Provider or missing session routing.

- [ ] **Step 3: Add provider-aware discovery and verification**

Switch `AgentService::discover` and `verify` on Provider kind. DSH verification uses a temporary Git directory, a fresh random headless session, read-only permission, and a fixed object schema. Record the real DSH version and the probe session ID as the verification execution identity; keep user-facing failure summaries credential-free.

- [ ] **Step 4: Resolve frozen DSH bindings at Run creation**

In `Runtime::start_run`, inspect either Codex or DSH and freeze the resolved executable. Continue rejecting Claude, OpenClaw, and Hermes.

- [ ] **Step 5: Replace direct Codex calls in all three paths**

For ordinary and flow dispatch, call `begin_provider_session` only for DSH and pass create/resume mode to the executor. Handle `Spawned` by attaching the process and `ConversationPersisted` by changing the matching session to `ready`; leave it `uncertain` on timeout, cancellation, or nonzero exit. For consultations, generate a temporary create ID, ignore `ConversationPersisted`, and never insert a main-session row. Preserve existing result validation, artifact publication, Changeset publication, and state transitions.

- [ ] **Step 6: Run focused Runtime tests**

Run: `cargo test -p archmage-core --test agents --test runtime --test consultations --test flow`

Expected: PASS.

- [ ] **Step 7: Commit Runtime integration**

```bash
git add crates/archmage-core/src/agent.rs crates/archmage-core/src/runtime crates/archmage-core/tests
git commit -m "feat: route capabilities through DeepSeek Harness"
```

### Task 6: Expose DeepSeek Harness in the desktop management UI

**Files:**
- Modify: `apps/archmage-desktop/src/model.ts`
- Modify: `apps/archmage-desktop/src/AgentManagement.tsx`
- Modify: `apps/archmage-desktop/src/CapabilityManagement.tsx`
- Modify: `apps/archmage-desktop/src/managementLayout.ts`
- Modify: `apps/archmage-desktop/src/agents.css`
- Test: `apps/archmage-desktop/src/AgentManagement.test.tsx`
- Test: `apps/archmage-desktop/src/managementLayout.test.ts`
- Test: `apps/archmage-desktop/src/model.test.ts`

- [ ] **Step 1: Write failing UI/model tests**

Add `deepseek_harness` to `ProviderKind`; assert metrics count Codex plus DSH and exclude unsupported Providers; assert Capability metrics accept either supported kind; render a DSH card/new-instance form and check the inherited `DSH_HOME`/default-model message.

- [ ] **Step 2: Run the focused tests and confirm they fail**

Run: `npm test -- AgentManagement.test.tsx managementLayout.test.ts model.test.ts`

Workdir: `apps/archmage-desktop`

Expected: FAIL because filters and forms are Codex-only.

- [ ] **Step 3: Implement Provider-aware forms and labels**

Define helpers rather than scattering comparisons:

```ts
export const isSupportedProvider = (kind: ProviderKind) =>
  kind === "codex" || kind === "deepseek_harness";
export const providerLabel = (kind: ProviderKind) =>
  kind === "deepseek_harness" ? "DeepSeek Harness" : kind === "codex" ? "Codex" : kind;
```

Let “添加实例” choose Codex or DeepSeek Harness. Reuse the card fields but show `dsh CLI 路径`, hide model override, and explain inherited `DSH_HOME` for DSH. Save the actual `providerKind`. Capability options and metrics use the helpers and display the Provider label.

- [ ] **Step 4: Run frontend tests and build**

Run: `npm test -- AgentManagement.test.tsx managementLayout.test.ts model.test.ts && npm run build`

Workdir: `apps/archmage-desktop`

Expected: PASS.

- [ ] **Step 5: Commit the desktop UI**

```bash
git add apps/archmage-desktop/src
git commit -m "feat: manage DeepSeek Harness agents"
```

### Task 7: Minimal end-to-end verification and current-state docs

**Files:**
- Modify: `docs/runbooks/desktop-s0.md`

- [ ] **Step 1: Update the runbook**

Replace the statement that only Codex is supported. Document inherited `DSH_HOME`, real verification cost, per-Run/Capability conversation reuse, and consultation isolation. Keep Hermes, Claude, and OpenClaw marked unsupported.

- [ ] **Step 2: Run the minimal repository checks**

DeepSeek Harness:

```bash
pnpm exec vitest run packages/bundle/headless/tests/startup.spec.ts packages/bundle/headless/tests/headless.spec.ts
pnpm run typecheck
```

Archmage:

```bash
cargo test -p archmage-core --test agents --test codex_protocol --test deepseek_harness_protocol --test runtime --test consultations --test flow
cargo fmt --all -- --check
(cd apps/archmage-desktop && npm test -- AgentManagement.test.tsx managementLayout.test.ts model.test.ts && npm run build)
git diff --check
```

Expected: all commands PASS.

- [ ] **Step 3: Run one real smoke Run**

Using the inherited default `DSH_HOME`, create a small temporary Git repository, bind one Capability to `deepseek-harness-default`, and run a minimal Team through Archmage. Confirm the stored provider session is `ready`, the result passes its Capability contract, and a second turn for the same Capability uses `--resume-session-id`.

If credentials or the required built `dsh` are unavailable, record the exact missing prerequisite and leave this acceptance item explicitly incomplete; do not substitute the fake CLI result.

- [ ] **Step 4: Commit the runbook**

```bash
git add docs/runbooks/desktop-s0.md
git commit -m "docs: verify DeepSeek Harness provider flow"
```

