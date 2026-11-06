# Agent Installation Guidance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show safe, platform-aware Codex and DeepSeek Harness installation instructions when Archmage cannot detect the corresponding local provider.

**Architecture:** A Tauri-side resolver owns the provider/platform support matrix and returns fixed guidance records plus current detection state. React renders one reusable inline guide from Provider headings or failed instance cards; it can copy the fixed command, open official documentation, and trigger the existing discovery path, but never executes installation commands.

**Tech Stack:** Rust/Tauri 2, React 19, TypeScript, Vitest, Cargo tests, `@tauri-apps/plugin-opener`.

---

### Task 1: Resolve platform-aware guidance in Tauri

**Files:**
- Create: `apps/archmage-desktop/src-tauri/src/agent_install.rs`
- Modify: `apps/archmage-desktop/src-tauri/src/lib.rs`

- [x] **Step 1: Write failing resolver tests**

Add unit tests for these records:

```rust
assert_eq!(resolve(ProviderKind::Codex, "macos", "aarch64", Some(14), false).command.as_deref(), Some("curl -fsSL https://chatgpt.com/codex/install.sh | sh"));
assert_eq!(resolve(ProviderKind::Codex, "windows", "x86_64", None, false).command.as_deref(), Some("powershell -ExecutionPolicy ByPass -c \"irm https://chatgpt.com/codex/install.ps1 | iex\""));
assert!(resolve(ProviderKind::DeepseekHarness, "macos", "aarch64", Some(14), false).supported);
assert!(!resolve(ProviderKind::DeepseekHarness, "windows", "x86_64", None, false).supported);
assert!(!resolve(ProviderKind::DeepseekHarness, "macos", "x86_64", Some(14), false).supported);
```

- [x] **Step 2: Run the focused tests and verify RED**

Run: `cargo test -p archmage-desktop agent_install`

Expected: FAIL because the module and resolver do not exist.

- [x] **Step 3: Implement the resolver and read-only command**

Define a camelCase-serialized `AgentInstallGuidance` with:

```rust
pub struct AgentInstallGuidance {
    pub provider_kind: ProviderKind,
    pub platform: String,
    pub architecture: String,
    pub supported: bool,
    pub detected: bool,
    pub prerequisite: Option<String>,
    pub command: Option<String>,
    pub documentation_url: String,
    pub unsupported_reason: Option<String>,
}
```

Implement a pure `resolve(...)` function, a macOS-major-version reader using `/usr/bin/sw_vers -productVersion`, and an async `list_agent_install_guidance` Tauri command. Detect Codex with `codex::find_executable("codex")`; detect DSH with `deepseek_harness::inspect_at(None).await`. Return guidance for Codex and DeepSeek Harness only. Register the command in `generate_handler!`.

- [x] **Step 4: Run the focused tests and verify GREEN**

Run: `cargo test -p archmage-desktop agent_install`

Expected: all resolver tests PASS.

### Task 2: Add the frontend data contract

**Files:**
- Modify: `apps/archmage-desktop/src/model.ts`
- Modify: `apps/archmage-desktop/src/api.ts`
- Modify: `apps/archmage-desktop/src/model.test.ts`

- [x] **Step 1: Write the failing model test**

Add a small lookup test for a new helper:

```ts
expect(installGuidanceFor(guidance, "codex")?.platform).toBe("macos");
expect(installGuidanceFor(guidance, "hermes")).toBeUndefined();
```

- [x] **Step 2: Run the focused test and verify RED**

Run: `npm test -- model.test.ts`

Expected: FAIL because `AgentInstallGuidance` and `installGuidanceFor` do not exist.

- [x] **Step 3: Add types, lookup, and API call**

Add the TypeScript mirror of the Rust record, implement `installGuidanceFor`, and expose:

```ts
listAgentInstallGuidance: () => invoke<AgentInstallGuidance[]>("list_agent_install_guidance")
```

- [x] **Step 4: Run the focused test and verify GREEN**

Run: `npm test -- model.test.ts`

Expected: PASS.

### Task 3: Render and operate the installation guide

**Files:**
- Modify: `apps/archmage-desktop/src/AgentManagement.test.tsx`
- Modify: `apps/archmage-desktop/src/AgentManagement.tsx`
- Modify: `apps/archmage-desktop/src/agents.css`
- Modify: `apps/archmage-desktop/src-tauri/capabilities/default.json`

- [x] **Step 1: Write failing rendering tests**

Render `AgentManagementContent` with undetected Codex and unsupported Windows DSH guidance. Assert that Provider headings expose “安装指引”, the open guide contains the fixed command and “复制命令”, and unsupported DSH contains its explanation without a copy action. Add a failed, versionless instance and assert its card exposes “查看安装指引”.

- [x] **Step 2: Run the focused test and verify RED**

Run: `npm test -- AgentManagement.test.tsx`

Expected: FAIL because guidance props and UI do not exist.

- [x] **Step 3: Implement guide state and actions**

Load snapshot and guidance together in `AgentManagement`. Track the active guide as `{ provider, instanceId? }`. Implement one `AgentInstallGuide` component with local copy/open/recheck feedback. Use `navigator.clipboard.writeText`, `openUrl` from `@tauri-apps/plugin-opener`, and add `opener:allow-open-url` to the desktop capability.

Provider headings show the guide button only when `detected === false`. Failed instance cards show it only when `connectionState === "failed" && !discoveredVersion`. Provider recheck reloads guidance; instance recheck first invokes existing discovery, then reloads guidance. No action executes the displayed command.

- [x] **Step 4: Add accessible responsive styles**

Style the guide as an inline card spanning the Provider grid. Keep command text selectable, make buttons at least 40px high with visible focus states, and stack guide actions below 780px.

- [x] **Step 5: Run the focused test and verify GREEN**

Run: `npm test -- AgentManagement.test.tsx`

Expected: PASS.

### Task 4: Verify the full feature

**Files:**
- Modify only if verification exposes a defect in files listed above.

- [x] **Step 1: Run Rust tests**

Run: `cargo test -p archmage-desktop`

Expected: all tests PASS.

- [x] **Step 2: Run desktop frontend tests**

Run: `npm test`

Expected: all tests PASS.

- [x] **Step 3: Build the desktop frontend**

Run: `npm run build`

Expected: TypeScript and Vite build successfully.

- [x] **Step 4: Review and commit**

Run: `git diff --check && git status --short`

Expected: no whitespace errors and only planned files plus the user's pre-existing untracked files. Commit with `feat: add agent installation guidance`.
