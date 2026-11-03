# Agent Management and Goal-Driven Runs Design

**Status:** Approved for implementation planning

## Goal

Make Archmage goal-driven at the product boundary and make local Agent routing visible. A user creates a Run with a repository and a goal, while a dedicated Agent management page shows which local Agent instances are usable and which instance executes each logical capability.

## Product decisions

- A Run form asks only for a Git repository and a nonblank goal.
- The user does not predeclare test commands or an acceptance plan.
- The Tester capability receives the testing task and independently decides how to inspect and test the work.
- Capability routing is global. There is no per-Run Agent override.
- Run creation freezes the current capability-to-Agent bindings so history remains truthful after settings change.
- The storage model supports multiple instances of the same Provider kind from the start.
- The first UI creates one default instance per supported Provider kind and keeps additional instances behind an advanced action.
- S1 implements a real Codex Adapter only. Claude, OpenClaw, and Hermes appear as known but unsupported Adapter kinds and never report a fake healthy state.

## Domain model

Archmage keeps the existing separation between physical Providers and logical capabilities.

```text
Provider Adapter       Agent Instance                 Capability Binding
----------------       -----------------------------  ------------------
codex (supported)  ->  codex-default                <- designer
claude (unsupported)   executable/model/config      <- coder
openclaw (unsupported) connection status            <- tester
hermes (unsupported)   last verification            <- reviewer
```

An Agent instance is a persisted local configuration:

- stable ID;
- Provider kind;
- user-facing name;
- optional executable override;
- optional model override;
- enabled flag;
- last connectivity state, time, version, and safe error summary.

The Adapter registry is compile-time product metadata. It distinguishes `supported`, `not_configured`, and `unsupported` without creating fake Provider implementations.

Each capability binding points to an Agent instance ID, not merely to a Provider kind. The initial migration creates `codex-default` and binds `designer`, `coder`, `tester`, and `reviewer` to it.

## Connectivity semantics

The management page separates local discovery from a real connectivity check:

- discovery verifies the configured executable, version, required `codex exec` flags, and Git availability;
- “Verify connection” runs the existing bounded read-only real Codex probe;
- a discovered CLI that has not completed the online probe is shown as “CLI available, connection unverified”;
- a successful probe records `ready`, the checked version, timestamp, and Codex thread ID;
- a failed probe records `failed` and a sanitized diagnostic summary;
- full diagnostic files remain private under the Archmage data root and are not placed in model prompts or UI payloads.

Changing executable or model configuration invalidates the previous online verification and returns the instance to `unverified`.

## Run binding snapshot

`RunRequest` contains only:

```text
repository: absolute path
goal: nonblank string
```

Before provisioning a worktree, Runtime resolves all built-in capabilities to enabled supported Agent instances. It rejects missing, disabled, unsupported, or locally undiscoverable bindings with a capability-specific preflight error.

The frozen Run definition stores, for every node:

- capability key;
- Agent instance ID and display name;
- Provider kind;
- executable path used at Run creation;
- model override, if any.

Node dispatch uses this frozen snapshot. Later edits in Agent management do not alter active or historical Runs.

## Tester semantics

Tester receives the goal, design, implementation summary, canonical ChangeSet, and workspace. It may inspect the project and run relevant commands through the normal Codex workspace sandbox. Runtime does not generate, freeze, or inject check commands.

Tester returns a strict structured result containing:

- overall `passed`, `failed`, or `blocked` status;
- human-readable summary;
- commands it chose to run, with exit codes and concise outcomes;
- limitations or unverified areas.

The structured result and private Codex diagnostics form the S1 evidence. Existing `CheckRunner` and `check_runs` storage remain available as lower-level infrastructure but are no longer part of the built-in Run path. This avoids a destructive migration while removing the predeclared-check product constraint.

Reviewer receives the Tester result. A failed or blocked Tester result cannot be approved.

## Desktop experience

The sidebar gains two stable destinations:

- `Runs`: new Run form and Run history;
- `Agents`: local connections and capability routing.

The Agents page contains:

1. A summary showing how many supported instances are ready, unverified, or failed.
2. A Codex instance card with name, executable, optional model, discovered version, connection state, last checked time, save action, and real “Verify connection” action.
3. Unsupported Claude, OpenClaw, and Hermes cards clearly labeled “Not supported yet”.
4. A capability routing table for Designer, Coder, Tester, and Reviewer. Each row selects a persisted compatible instance and saves globally.
5. An advanced “Add Codex instance” action. The initial page still starts with only `codex-default`.

The new Run form removes all acceptance-command controls and enables Start when repository, goal, environment, and global capability bindings are valid.

Run inspection displays the frozen Agent name and Provider kind for every node so users can answer which Agent performed historical work.

## API boundary

Core adds an Agent service used by both Tauri and future CLI surfaces:

- `agent_management_snapshot()`
- `save_agent_instance(input)`
- `verify_agent_instance(id)`
- `bind_capability(capability, instance_id)`

Tauri exposes typed commands with the same responsibilities. React never invokes Codex directly or reads SQLite.

## Failure behavior

- Unsupported Adapter kinds cannot be saved as runnable instances or bound to capabilities.
- Deleting or disabling a bound instance is rejected until its bindings are changed.
- A failed verification is durable and visible but does not fabricate authentication details.
- Run creation reports the exact missing or unusable capability binding.
- Connectivity or invocation failure after Run creation remains a persisted node/Run failure with private diagnostics.
- Existing historical Run definitions containing checks remain readable through backward-compatible deserialization, but new Runs never require or create them.

## Testing strategy

- migration and Store tests cover default Codex instance creation, multiple same-kind instances, and binding integrity;
- Agent service tests cover discovery status, connectivity result persistence, and binding validation;
- Runtime tests prove repository plus goal is sufficient and bindings are frozen per Run;
- contract tests prove autonomous Tester output and Reviewer rejection after failed/blocked testing;
- frontend tests cover goal-only request serialization and Agent state presentation;
- the existing ignored real-Codex tests are extended with one explicit Agent verification test;
- final verification uses focused Rust tests, frontend tests/build, workspace tests, and one opt-in real Codex probe when credentials are available.

## Non-goals

- real Claude, OpenClaw, or Hermes invocation;
- automatic Provider selection or fallback routing;
- per-Run capability overrides;
- secrets or login flows inside Archmage;
- workflow editing;
- removing the legacy CheckRunner tables and APIs;
- pretending an offline executable check proves authentication.
