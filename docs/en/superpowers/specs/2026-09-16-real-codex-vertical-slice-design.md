# Real Codex Vertical Slice Design

[简体中文](../../../zh-CN/superpowers/specs/2026-09-16-real-codex-vertical-slice-design.md) | English

**Status:** Approved

**Date:** 2026-09-16

## Summary

S0 connects one real local Codex CLI provider and coordinates four logical Capability Agents—Designer, Coder, Tester, and Reviewer—through explicit Handoffs in a per-run Git worktree. Each node runs in a new Codex session, receives a frozen Context Snapshot, and must return a schema-valid result. SQLite and the local artifact store make every state transition, input, output, and recovery decision durable.

This slice intentionally crosses the original M0–M5 layers. It produces a runnable collaboration proof before generalizing workflow definitions or building a desktop UI.

## Decisions already approved

1. Deliver a vertical slice rather than a domain-only foundation.
2. Use a real Git worktree from the first stage.
3. Call real Codex for every Capability Agent; product execution has no Fake Agent mode.
4. Ship Runtime plus CLI before Tauri and React.
5. Treat Provider as the physical execution backend and Capability Agent as the collaboration actor.
6. Allow concurrent ready nodes subject to dependency, provider capacity, and workspace-access constraints.
7. Use independent Codex sessions and explicit Handoffs instead of cross-node session history.
8. Leave worktrees and branches intact; do not automatically commit, merge, push, or delete.

## Architecture

The Rust runtime owns workflow state, scheduling, context resolution, contracts, workspace lifecycle, persistence, artifacts, and events. A provider-neutral interface separates runtime policy from Codex process details. The CLI is a presentation and control layer over runtime APIs.

```text
CLI → Runtime → Capability Agent invocation → Codex Provider → codex exec
        │                 │                       │
        │                 └→ Context + Contract  └→ JSONL + final JSON
        ├→ Workspace Manager → Git worktree
        └→ Store → SQLite + local artifacts
```

Although S0 has only one provider implementation, runtime types use provider-neutral identities and outcomes. Provider-specific JSONL types remain inside the Codex adapter.

## Collaboration flow

### Designer

Receives the goal, baseline repository metadata, and a bounded repository view. It runs read-only and returns a versioned design JSON plus `summary_markdown`.

### Coder

Receives the goal and validated design artifacts. It runs with workspace-write access in the run worktree and returns an ImplementationResult. The runtime independently captures the resulting status and diff against baseline.

### Tester

Receives the goal, design, implementation result, workspace, and explicit validation policy. Each required check has an ID and must be invoked by the Tester through an Archmage-owned check runner. That runner executes the configured command in the real worktree and durably records command, exit status, stdout, stderr, and duration before returning control to Codex. The Tester interprets this evidence and returns TestResult. Runtime invariants prevent `passed` when required evidence is missing or contains a non-zero result.

### Reviewer

Receives the goal, design, implementation result, baseline diff, and TestResult. It runs read-only and returns ReviewResult. S0 records `approved` or `changes_requested` but treats either as a terminal Reviewer outcome; S1 adds conditional routing to Fixer. Both outcomes complete the workflow lifecycle, while a separate completion outcome preserves the review decision. The CLI returns success for `approved` and a distinct non-zero outcome code for `changes_requested`.

## Provider protocol

The adapter invokes the installed Codex CLI using the non-interactive interface:

```text
codex exec
  --cd <run-worktree>
  --sandbox <read-only|workspace-write>
  --json
  --output-schema <capability-schema>
  --output-last-message <result-path>
  -
```

The rendered prompt is written to stdin. JSONL on stdout is parsed incrementally and persisted; stderr is captured separately. The adapter records the `thread.started` identifier, process identity, version, timestamps, exit status, usage when available, and termination path.

Normal Handoffs never call `codex exec resume`. On Archmage resume, an interrupted node starts a new Codex session with a new attempt and the same persisted logical inputs. If an interrupted writer left partial workspace changes, the new attempt also receives a new recovery Context Snapshot describing the current workspace and its delta from the previous snapshot. Previous snapshots remain immutable. This keeps recovery independent of provider-local conversation files without pretending that code changes disappeared after interruption.

## Context and Handoff

Each attempt resolves typed Context References into an immutable snapshot before it starts. The snapshot records origin, hash, serialization version, prompt version, provider binding, and workspace identity. Downstream nodes can read only committed upstream artifacts.

Handoffs are explicit records linking producer artifacts to consumer Context bindings. They do not expose hidden provider reasoning or conversation history.

## Contracts

Each built-in Capability has a versioned JSON Schema. Codex is asked to conform through `--output-schema`, and the runtime validates the final file again. Capability-specific invariants validate relationships the schema alone cannot express.

Every valid result contains `summary_markdown`. The runtime stores validated JSON for machines, extracted Markdown for people, and the raw provider stream for diagnosis.

## Scheduler and concurrency

The scheduler derives a Ready Set from persisted dependencies and Context availability. S0's built-in workflow is linear, but the engine allows compatible nodes to run concurrently.

Nodes declare workspace access:

- `none`: compatible with all other access modes
- `read`: compatible with `none` and other readers
- `write`: compatible only with `none` and holds an exclusive worktree lease

Dispatch also respects `run.max_parallelism` and provider capacity. State transition plus Event insertion is atomic.

## Workspace lifecycle

Run creation freezes the source repository's current `HEAD` as baseline and creates `archmage/run-<run-id>` in `<archmage-data>/workspaces/<run-id>`. Dirty source changes are excluded and reported as a warning.

Worktree identity, baseline, branch, `HEAD`, and status are checked before and after every write-capable node. Capability instructions forbid commits, and a changed `HEAD` fails the workspace invariant. Diff artifacts are generated independently of agent claims. S0 retains the worktree and branch after all terminal outcomes.

## Persistence and recovery

SQLite uses migrations, WAL, foreign keys, transactions, and bounded busy handling. Artifact content uses temporary-file write, flush, and atomic rename before metadata insertion.

On startup, owned attempts left in `running` are reconciled. If their process cannot be safely confirmed, the attempt becomes `interrupted`. `archmage resume` preserves successful nodes, discards incomplete outputs from Context eligibility, and creates a new attempt for retryable work.

Reconciliation requires exclusive scheduler ownership of the Run. An interrupted classification does not establish that an old child process stopped: replacement writers must wait until owned Codex/check processes are confirmed exited or safely terminated. Uncertain process identity blocks execution instead of authorizing a signal to a possibly reused PID. Read-only status inspection does not take over an active Run.

## Error model

S0 distinguishes at least:

- `preflight_failed`
- `spawn_failed`
- `auth_failed`
- `timed_out`
- `cancelled`
- `cli_failed`
- `contract_invalid`
- `workspace_invariant_failed`
- `storage_failed`
- `interrupted`

Every failure retains available diagnostics, emits a typed event, and leaves the worktree available for inspection. Free-form prose cannot override a failed exit status or invariant.

## CLI

S0 exposes:

```text
archmage doctor
archmage run --repo <path> --goal <text> --check <command> [--check ...]
archmage status <run-id> [--json]
archmage logs <run-id> [--follow]
archmage resume <run-id>
archmage cancel <run-id>
```

`run` executes in the foreground and streams committed Events. Ctrl-C requests cancellation. Commands share the runtime crate rather than duplicating workflow behavior.

Workflow command exit codes are:

| Code | Meaning |
|---:|---|
| `0` | Workflow completed with Reviewer approval |
| `10` | Workflow completed with `changes_requested` |
| `20` | Preflight or infrastructure failure prevented reliable execution |
| `21` | A Capability Agent, contract, or workspace invariant failed |
| `22` | The Run was cancelled |

CLI argument errors retain the parser's standard non-zero behavior and never create a Run.

## Testing

Automated tests cover domain transitions, Ready Set and lease compatibility, migrations, transaction rollback, recovery queries, artifact atomicity, real temporary Git repositories and worktrees, schema/invariant validation, Context hashing, and Codex JSONL parsing from fixed protocol samples.

Protocol samples are parser fixtures, not an executable provider and not a product Fake Agent path. A complete happy-path run invokes real authenticated Codex for all four roles. The separate interruption/resume scenario necessarily creates additional real invocations.

## Security

- Use explicit `read-only` or `workspace-write` sandbox modes; never `danger-full-access` in S0.
- Do not serialize credentials or environment secrets into prompts, events, or artifacts.
- Treat repository content and agent output as untrusted data.
- Pass process arguments without shell interpolation.
- Restrict filesystem writes to the run worktree and Archmage data root.
- Do not mutate the source repository's checked-out branch or any remote.

## Scope exclusions

S0 excludes the Fix loop, human gates, external workflow editing, other providers, desktop UI, daemonization, automatic cleanup, remote execution, memory, RAG, marketplaces, and cloud/multi-user features.

## Acceptance

The authoritative exit criteria are in [the S0 stage document](../../roadmap/stages/s0-real-codex-slice.md) and the executable manual procedure is in [the S0 acceptance runbook](../../runbooks/s0-acceptance.md).

The [S0 implementation plan](../plans/2026-09-16-s0-real-codex-vertical-slice.md) specifies file ownership, task dependencies, test gates, and the implementation details above. No product implementation or real provider acceptance is claimed by this document.
