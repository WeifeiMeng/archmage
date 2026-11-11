# Live Agent Output and Resizable Run Panels Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show bounded public Agent progress in the selected Capability inspector and add a persistent, accessible horizontal split with the inspector on the left and timeline on the right.

**Architecture:** Normalize provider-specific updates into a shared `ProviderProgress` lifecycle event, persist accepted progress as ordered `agent.progress` Runtime events, and derive the selected node feed from the existing desktop timeline stream. Keep rendering and split behavior in focused React modules; store only the inspector percentage in `localStorage`, clamp it against current geometry, and stack the panels at the existing narrow breakpoint.

**Tech Stack:** Rust 2024, Tokio, SQLite/rusqlite, serde/serde_json, Codex JSONL, Qoder ACP JSON-RPC, React 19, TypeScript, CSS Grid, Vitest.

---

## File structure

- Create `crates/archmage-core/src/provider/progress.rs`: provider-neutral progress types, bounds, and Codex/Qoder allowlist mapping.
- Modify `crates/archmage-core/src/provider/mod.rs`: export the progress module.
- Modify `crates/archmage-core/src/provider/codex.rs`: emit decoded public progress during capture.
- Modify `crates/archmage-core/src/provider/executor.rs`: expose spawn and progress through one lifecycle observer.
- Modify `crates/archmage-core/src/provider/qoder.rs`: map allowlisted `session/update` frames and coalesce final Agent text.
- Modify `crates/archmage-core/src/provider/pool.rs`: forward lifecycle observations from persistent and temporary sessions.
- Modify `crates/archmage-core/src/store/transitions.rs`: persist bounded `agent.progress` events against the owning running Attempt.
- Modify `crates/archmage-core/src/runtime/dispatch.rs`, `flow.rs`, and `consultation.rs`: connect provider lifecycle events to Runtime persistence while preserving process registration.
- Modify `crates/archmage-core/tests/codex_protocol.rs`, `qoder_acp_protocol.rs`, `provider_pool.rs`, and `persistence.rs`: cover allowlists, bounds, forwarding, and persistence identity.
- Create `apps/archmage-desktop/src/agentProgress.ts`: parse and select progress events for a node.
- Create `apps/archmage-desktop/src/agentProgress.test.ts`: cover ordering and malformed/foreign events.
- Create `apps/archmage-desktop/src/AgentLiveOutput.tsx`: render public progress and empty states.
- Create `apps/archmage-desktop/src/AgentLiveOutput.test.tsx`: cover Capability-first labels and progress kinds.
- Create `apps/archmage-desktop/src/RunSplitPane.tsx`: pointer/keyboard split behavior and persistent preference.
- Create `apps/archmage-desktop/src/runSplitPane.ts` and `runSplitPane.test.ts`: pure ratio parsing and clamping logic.
- Modify `apps/archmage-desktop/src/App.tsx`, `model.ts`, `styles.css`, and `inspector.css`: integrate inspector-first composition, progress, and separator styling.

### Task 1: Define and validate provider-neutral progress

**Files:**
- Create: `crates/archmage-core/src/provider/progress.rs`
- Modify: `crates/archmage-core/src/provider/mod.rs`
- Test: `crates/archmage-core/tests/codex_protocol.rs`
- Test: `crates/archmage-core/tests/qoder_acp_protocol.rs`

- [ ] **Step 1: Write failing mapper tests**

Add tests that call the planned API directly:

```rust
use archmage_core::provider::progress::{
    MAX_PROGRESS_TEXT_BYTES, ProviderProgress, codex_progress, qoder_progress,
};

#[test]
fn codex_progress_allows_public_items_and_drops_reasoning() {
    assert_eq!(
        codex_progress(&json!({"type":"item.completed","item":{"type":"agent_message","text":"Working on tests"}})),
        Some(ProviderProgress::message("Agent message", "Working on tests"))
    );
    assert!(codex_progress(&json!({"type":"item.completed","item":{"type":"reasoning","text":"private"}})).is_none());
}

#[test]
fn progress_text_is_utf8_bounded() {
    let mapped = codex_progress(&json!({"type":"item.completed","item":{"type":"agent_message","text":"你".repeat(MAX_PROGRESS_TEXT_BYTES)}})).unwrap();
    assert!(mapped.text.unwrap().len() <= MAX_PROGRESS_TEXT_BYTES);
}

#[test]
fn qoder_progress_drops_agent_thought_and_maps_tool_status() {
    assert!(qoder_progress(&json!({"params":{"update":{"sessionUpdate":"agent_thought_chunk","content":{"type":"text","text":"private"}}}})).is_none());
    let mapped = qoder_progress(&json!({"params":{"update":{"sessionUpdate":"tool_call","toolCallId":"t1","title":"Run tests","status":"pending"}}})).unwrap();
    assert_eq!(mapped.kind, "tool");
    assert_eq!(mapped.status.as_deref(), Some("pending"));
}
```

- [ ] **Step 2: Run focused tests and verify RED**

Run: `cargo test -p archmage-core --test codex_protocol --test qoder_acp_protocol progress -- --nocapture`

Expected: compilation fails because `provider::progress` does not exist.

- [ ] **Step 3: Implement the shared type and allowlist mappers**

Create a serde-serializable type with stable UI fields:

```rust
pub const MAX_PROGRESS_TEXT_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderProgress {
    pub kind: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub metadata: Map<String, Value>,
}
```

Implement constructors and a UTF-8-safe `bounded_text`. `codex_progress` must allowlist `agent_message`, `command_execution`, `file_change`, `mcp_tool_call`, and safe turn usage/status records. `qoder_progress` must allowlist visible message, plan, tool lifecycle, usage/configuration metadata, and notice updates. Both functions must return `None` for reasoning/thought, unknown, or malformed input and must construct metadata field-by-field rather than cloning raw frames.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run: `cargo test -p archmage-core --test codex_protocol --test qoder_acp_protocol progress -- --nocapture`

Expected: all progress mapper tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/archmage-core/src/provider/mod.rs crates/archmage-core/src/provider/progress.rs crates/archmage-core/tests/codex_protocol.rs crates/archmage-core/tests/qoder_acp_protocol.rs
git commit -m "feat: normalize public provider progress"
```

### Task 2: Stream Codex and Qoder progress through provider observers

**Files:**
- Modify: `crates/archmage-core/src/provider/codex.rs`
- Modify: `crates/archmage-core/src/provider/executor.rs`
- Modify: `crates/archmage-core/src/provider/qoder.rs`
- Modify: `crates/archmage-core/src/provider/pool.rs`
- Test: `crates/archmage-core/tests/codex_protocol.rs`
- Test: `crates/archmage-core/tests/provider_pool.rs`

- [ ] **Step 1: Write failing observer tests**

Extend fake-provider tests to collect lifecycle events and assert the spawn observation precedes progress. For Qoder, add public plan/tool/message updates to `tests/fixtures/fake_qoder_acp.py` and assert the pool forwards normalized progress while excluding an `agent_thought_chunk`.

```rust
let observed = Mutex::new(Vec::new());
pool.invoke_main(
    SidecarKey::new("run-q", "coder"), config, "attempt", b"return JSON", &schema,
    |event| { observed.lock().unwrap().push(event); Ok(()) },
).await.unwrap();
let observed = observed.into_inner().unwrap();
assert!(matches!(observed.first(), Some(ProviderLifecycle::Spawned(_))));
assert!(observed.iter().any(|event| matches!(event, ProviderLifecycle::Progress(item) if item.kind == "tool")));
assert!(!observed.iter().any(|event| matches!(event, ProviderLifecycle::Progress(item) if item.text.as_deref() == Some("private"))));
```

- [ ] **Step 2: Run focused tests and verify RED**

Run: `cargo test -p archmage-core --test codex_protocol --test provider_pool observer -- --nocapture`

Expected: tests fail because observers accept only spawn identity and no progress variant exists.

- [ ] **Step 3: Implement a unified lifecycle observer**

Change the public lifecycle enum to:

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum ProviderLifecycle {
    Spawned(ProcessIdentity),
    Progress(ProviderProgress),
}
```

In Codex capture, decode complete events and call the observer for each allowlisted `codex_progress` result after writing the private JSONL. Keep final completion validation unchanged.

Change pool callbacks from `FnOnce(&ProcessIdentity)` to `FnMut(ProviderLifecycle)`. Emit `Spawned` for every Attempt before invocation, then forward Qoder progress emitted while reading `session/update`. Coalesce `agent_message_chunk` into bounded text and emit one visible message at the end of the request; map discrete plan/tool/notice updates immediately. DeepSeek Harness emits only `Spawned`.

- [ ] **Step 4: Update every pool test call site and verify GREEN**

Update closures to match `ProviderLifecycle::Spawned(identity)` and ignore or collect `Progress`. Run:

`cargo test -p archmage-core --test codex_protocol --test provider_pool --test qoder_acp_protocol -- --nocapture`

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/archmage-core/src/provider/codex.rs crates/archmage-core/src/provider/executor.rs crates/archmage-core/src/provider/qoder.rs crates/archmage-core/src/provider/pool.rs crates/archmage-core/tests/codex_protocol.rs crates/archmage-core/tests/provider_pool.rs crates/archmage-core/tests/qoder_acp_protocol.rs crates/archmage-core/tests/fixtures/fake_qoder_acp.py
git commit -m "feat: observe live provider progress"
```

### Task 3: Persist progress as ordered Runtime events

**Files:**
- Modify: `crates/archmage-core/src/store/transitions.rs`
- Modify: `crates/archmage-core/src/runtime/dispatch.rs`
- Modify: `crates/archmage-core/src/runtime/flow.rs`
- Modify: `crates/archmage-core/src/runtime/consultation.rs`
- Test: `crates/archmage-core/tests/persistence.rs`
- Test: `crates/archmage-core/tests/runtime.rs`

- [ ] **Step 1: Write a failing persistence test**

Start a real Store Attempt, attach an owned process, record progress, and assert identity and payload:

```rust
store.record_agent_progress(&attempt.id, generation, &ProviderProgress::message("Agent message", "Checking tests")).unwrap();
let event = store.events(&run, 0).unwrap().into_iter().find(|event| event.event_type == "agent.progress").unwrap();
assert_eq!(event.node_id.as_deref(), Some(node.id.as_str()));
assert_eq!(event.attempt_id.as_deref(), Some(attempt.id.as_str()));
assert_eq!(event.payload["kind"], "message");
assert_eq!(event.payload["text"], "Checking tests");
```

Also assert recording against a non-running Attempt or wrong owner generation returns `Error::Conflict` and creates no event.

- [ ] **Step 2: Run the test and verify RED**

Run: `cargo test -p archmage-core --test persistence agent_progress -- --nocapture`

Expected: compilation fails because `record_agent_progress` does not exist.

- [ ] **Step 3: Implement transactional event persistence**

Add `Store::record_agent_progress(attempt, generation, progress)`. Inside an immediate transaction, query the owning Node and Run only when the Attempt is `running`, the Run is `running`, and both owner generations match. Insert `agent.progress` with `node_id`, `attempt_id`, and `serde_json::to_value(progress)`, then commit.

Update normal dispatch and flow execution callbacks:

```rust
match event {
    ProviderLifecycle::Spawned(identity) => store.attach_attempt_process(..., &identity),
    ProviderLifecycle::Progress(progress) => store.record_agent_progress(attempt_id, generation, &progress),
}
```

Consultation progress is not associated with an executing Node Attempt and must not be persisted as node progress; its callback continues to register process identity and safely ignores public progress until consultation-specific UI is designed.

- [ ] **Step 4: Run persistence and Runtime tests and verify GREEN**

Run: `cargo test -p archmage-core --test persistence --test runtime --test flow -- --nocapture`

Expected: all tests pass and existing final result/Artifact assertions are unchanged.

- [ ] **Step 5: Commit**

```bash
git add crates/archmage-core/src/store/transitions.rs crates/archmage-core/src/runtime/dispatch.rs crates/archmage-core/src/runtime/flow.rs crates/archmage-core/src/runtime/consultation.rs crates/archmage-core/tests/persistence.rs crates/archmage-core/tests/runtime.rs
git commit -m "feat: persist agent progress events"
```

### Task 4: Model and render Capability-first live output

**Files:**
- Create: `apps/archmage-desktop/src/agentProgress.ts`
- Create: `apps/archmage-desktop/src/agentProgress.test.ts`
- Create: `apps/archmage-desktop/src/AgentLiveOutput.tsx`
- Create: `apps/archmage-desktop/src/AgentLiveOutput.test.tsx`
- Modify: `apps/archmage-desktop/src/model.ts`
- Modify: `apps/archmage-desktop/src/inspector.css`

- [ ] **Step 1: Write failing model tests**

Define `AgentProgress` and test `progressForNode(events, nodeId)` filters `agent.progress`, rejects malformed payloads, preserves sequence order, and ignores other nodes.

```ts
expect(progressForNode(events, "node-1")).toEqual([
  { sequence: 12, kind: "tool", label: "Run tests", status: "running", text: null, metadata: {} },
]);
```

- [ ] **Step 2: Run model tests and verify RED**

Run: `npm test -- --run src/agentProgress.test.ts`

Working directory: `apps/archmage-desktop`

Expected: module import fails because `agentProgress.ts` does not exist.

- [ ] **Step 3: Implement strict desktop parsing**

Allow only `message`, `plan`, `tool`, `notice`, `usage`, and `status`. Require string `label`, accept optional string `status`/`text`, and copy only primitive metadata values. Return records sorted by Runtime sequence.

- [ ] **Step 4: Write failing component tests**

Render `AgentLiveOutput` to static markup and assert:

```tsx
const html = renderToStaticMarkup(<AgentLiveOutput
  capability="Coder"
  provider="Codex · codex"
  running
  progress={[{ sequence: 1, kind: "tool", label: "Run tests", status: "running", text: "cargo test", metadata: {} }]}
/>);
expect(html).toContain("Coder 正在运行");
expect(html).toContain("Codex · codex");
expect(html).toContain("Run tests");
expect(html).not.toContain("Codex 正在运行");
```

Cover running-empty, unsupported/history-empty, plan, message, tool, notice, status, and usage rendering.

- [ ] **Step 5: Run component tests and verify RED**

Run: `npm test -- --run src/AgentLiveOutput.test.tsx`

Working directory: `apps/archmage-desktop`

Expected: module import fails because `AgentLiveOutput.tsx` does not exist.

- [ ] **Step 6: Implement the live-output component and styles**

Render Capability status as the primary heading, Provider as secondary text, an `aria-live="polite"` ordered feed, kind/status labels, bounded text, and useful empty copy. Do not render raw payload JSON or unknown records.

- [ ] **Step 7: Run desktop focused tests and verify GREEN**

Run: `npm test -- --run src/agentProgress.test.ts src/AgentLiveOutput.test.tsx`

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add apps/archmage-desktop/src/agentProgress.ts apps/archmage-desktop/src/agentProgress.test.ts apps/archmage-desktop/src/AgentLiveOutput.tsx apps/archmage-desktop/src/AgentLiveOutput.test.tsx apps/archmage-desktop/src/model.ts apps/archmage-desktop/src/inspector.css
git commit -m "feat: render capability live output"
```

### Task 5: Add the persistent accessible split pane

**Files:**
- Create: `apps/archmage-desktop/src/runSplitPane.ts`
- Create: `apps/archmage-desktop/src/runSplitPane.test.ts`
- Create: `apps/archmage-desktop/src/RunSplitPane.tsx`
- Create: `apps/archmage-desktop/src/RunSplitPane.test.tsx`
- Modify: `apps/archmage-desktop/src/styles.css`

- [ ] **Step 1: Write failing ratio tests**

```ts
expect(parseStoredInspectorRatio(null)).toBe(43);
expect(parseStoredInspectorRatio("66")).toBe(66);
expect(parseStoredInspectorRatio("bad")).toBe(43);
expect(clampInspectorRatio(90, 1000, 320, 12)).toBe(66.8);
expect(adjustInspectorRatio(43, "ArrowRight")).toBe(45);
```

- [ ] **Step 2: Run ratio tests and verify RED**

Run: `npm test -- --run src/runSplitPane.test.ts`

Working directory: `apps/archmage-desktop`

Expected: module import fails.

- [ ] **Step 3: Implement pure ratio behavior**

Export `DEFAULT_INSPECTOR_RATIO = 43`, `MIN_PANEL_PX = 320`, `SPLITTER_PX = 12`, `parseStoredInspectorRatio`, `clampInspectorRatio`, and `adjustInspectorRatio`. Clamp against container pixels without overwriting the stored preferred ratio during passive window resizing.

- [ ] **Step 4: Write failing component markup and interaction-boundary tests**

Test that `RunSplitPane` renders inspector first, a semantic separator with `aria-orientation="vertical"`, timeline second, and applies the persisted CSS custom property. Exercise keyboard changes through the exported handler boundary and assert the storage writer receives the updated preferred ratio.

- [ ] **Step 5: Run component tests and verify RED**

Run: `npm test -- --run src/RunSplitPane.test.tsx`

Working directory: `apps/archmage-desktop`

Expected: module import fails.

- [ ] **Step 6: Implement pointer, keyboard, resize, and persistence behavior**

Use pointer capture on the separator. Compute ratio from `clientX` and the container rectangle, clamp it, update immediately, and write `archmage.run.inspectorRatio` on pointer completion and keyboard adjustment. Use `ResizeObserver` to derive an effective clamped ratio. Keep the separator focusable through its native button semantics and set `role="separator"`, `aria-valuemin`, `aria-valuemax`, and `aria-valuenow`.

- [ ] **Step 7: Implement responsive CSS**

Use three grid columns: `minmax(320px, var(--inspector-width)) 12px minmax(320px, 1fr)`. Give the separator a 12-pixel target with a narrow visible bar and clear hover/focus/drag states. At `max-width: 1050px`, use one column, inspector first, timeline second, and hide the separator.

- [ ] **Step 8: Run focused tests and verify GREEN**

Run: `npm test -- --run src/runSplitPane.test.ts src/RunSplitPane.test.tsx`

Expected: all tests pass.

- [ ] **Step 9: Commit**

```bash
git add apps/archmage-desktop/src/runSplitPane.ts apps/archmage-desktop/src/runSplitPane.test.ts apps/archmage-desktop/src/RunSplitPane.tsx apps/archmage-desktop/src/RunSplitPane.test.tsx apps/archmage-desktop/src/styles.css
git commit -m "feat: add persistent run panel splitter"
```

### Task 6: Integrate the Run workbench and verify end to end

**Files:**
- Modify: `apps/archmage-desktop/src/App.tsx`
- Modify: `apps/archmage-desktop/src/styles.css`
- Modify: `apps/archmage-desktop/src/inspector.css`
- Test: `apps/archmage-desktop/src/App.test.tsx` or focused extracted view test

- [ ] **Step 1: Write a failing workbench integration test**

Render the extracted/exported workbench view with a running Coder bound to Codex and one `agent.progress` event. Assert DOM order is inspector, separator, timeline; the page contains `Coder 正在运行`, `Codex · codex`, and the progress text; and it does not contain `Codex 正在运行`.

- [ ] **Step 2: Run the integration test and verify RED**

Run: `npm test -- --run src/App.test.tsx`

Working directory: `apps/archmage-desktop`

Expected: test fails because the old workbench puts timeline first and has no live-output section.

- [ ] **Step 3: Integrate the focused components**

Derive `nodeProgress = progressForNode(timeline.events, node.id)`. Use frozen Capability configuration for the primary display label and `providerLabel(binding.providerKind)` plus the provider token for secondary identity. Compose:

```tsx
<RunSplitPane
  inspector={<NodeInspector node={node} progress={nodeProgress} ... />}
  timeline={<RunTimeline timeline={timeline} />}
/>
```

Move existing result metadata, structured result, consultations, and run records into the inspector content without changing their behavior. Keep full Runtime events in the right-hand timeline.

- [ ] **Step 4: Run integration and complete desktop tests**

Run: `npm test`

Working directory: `apps/archmage-desktop`

Expected: all Vitest tests pass.

- [ ] **Step 5: Run Rust formatting and full core tests**

Run: `cargo fmt --check`

Run: `cargo test -p archmage-core`

Expected: formatting check and all core tests pass. Platform-gated live tests may remain ignored by their existing annotations.

- [ ] **Step 6: Run desktop production build**

Run: `npm run build`

Working directory: `apps/archmage-desktop`

Expected: TypeScript and Vite production build succeed with no errors.

- [ ] **Step 7: Perform visual acceptance**

Launch the desktop development app and verify:

- inspector is left and timeline is right;
- the separator follows pointer movement, has a visible focus state, and responds to keyboard controls;
- a dragged width survives Run changes and application reload;
- `Coder 正在运行` is primary and Provider identity is secondary;
- live records append without exposing thought or raw stderr;
- final structured result remains separate;
- below 1050 pixels the inspector stacks before timeline and the separator disappears.

- [ ] **Step 8: Commit integration**

```bash
git add apps/archmage-desktop/src/App.tsx apps/archmage-desktop/src/styles.css apps/archmage-desktop/src/inspector.css apps/archmage-desktop/src/App.test.tsx
git commit -m "feat: integrate live output run workbench"
```

- [ ] **Step 9: Review branch state**

Run: `git status --short --branch`

Expected: clean worktree on the implementation branch with the design, plan, and feature commits present.
