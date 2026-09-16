# S0: Real Codex Vertical Slice

**Status:** Approved for implementation planning

## Goal

Prove the Archmage collaboration spine by connecting a real local Codex CLI provider and coordinating four independent Capability Agents through a fixed workflow in an isolated Git worktree.

## User-visible flow

```bash
archmage doctor
archmage run \
  --repo /path/to/repository \
  --goal "Implement a health check endpoint" \
  --check "<project-specific validation command>"
```

The command streams a Timeline and ends with a Run ID, result, worktree path, branch, and artifact directory. The workflow is:

```text
Designer → Coder → Tester → Reviewer → Completed
```

Each node invokes a new real `codex exec` session. No product code path substitutes a Fake Agent.

## In scope

### Runtime foundation

- Rust workspace with UI-independent core and thin CLI
- Domain identifiers and persisted state for Project, Provider, Capability, Run, Node Run, Agent Run, Workspace, Context Snapshot, Artifact, and Event
- SQLite migrations, WAL, transactional state-and-event writes
- Ready Set scheduler with concurrency and workspace lease compatibility
- Explicit cancellation, failure classification, restart scan, and resume

### Real Codex provider

- Detect Codex executable and record the tested version
- Verify that authentication is usable through a safe provider preflight
- Invoke `codex exec` with explicit working directory and sandbox
- Send the complete rendered prompt through stdin
- Capture JSONL events and the final schema-constrained result
- Record the Codex thread ID for observability without using hidden session history for Handoffs

### Capability Agents

- Designer: read-only repository analysis and design artifacts
- Coder: workspace-write implementation and ImplementationResult
- Tester: workspace-write validation through Archmage-owned required-check commands and TestResult
- Reviewer: read-only review of goal, design, diff, and test evidence

### Context and contracts

- Versioned built-in instructions and JSON Schemas
- Typed Context References and immutable Context Snapshots
- Machine JSON plus extracted human-readable Markdown
- Runtime re-validation and capability-specific invariants

### Workspace

- Source repository validation and persisted baseline `HEAD`
- Per-run `archmage/run-<id>` branch and external worktree
- Dirty source warning without copying uncommitted changes
- Diff and status artifacts relative to baseline
- No automatic commit, merge, push, worktree removal, or branch deletion

### CLI

- `doctor`
- `run`
- `status`
- `logs`
- `resume`
- `cancel`

## Out of scope

- Fake Agent or simulated workflow mode
- Automatic Review-to-Fix loop
- User-authored workflow and capability definitions
- Additional providers
- Desktop application
- Background daemon and automatic resume
- Automatic commit, merge, push, or cleanup
- Cloud services, multi-user features, memory, RAG, and marketplaces

## Functional slices

1. **Bootstrap and diagnostics:** establish the Rust workspace, data root, CLI shell, and Codex/Git preflight.
2. **Persistence and domain:** create migrations, state machines, event transactions, and recovery queries.
3. **Workspace isolation:** create real branches/worktrees and enforce baseline and lease invariants.
4. **Codex protocol:** invoke the local CLI, parse JSONL, capture output, cancel safely, and classify failures.
5. **Capability contracts:** define the four roles, schemas, prompts, artifacts, and Context bindings.
6. **Scheduler and orchestration:** derive the Ready Set, dispatch nodes, commit transitions, and build Handoffs.
7. **Recovery and observability:** support status, logs, interruption detection, resume, and final report generation.
8. **Real acceptance:** run and interrupt the complete workflow against a controlled repository using authenticated Codex.

## Exit criteria

S0 is complete only when all of the following are true:

1. `archmage doctor` confirms Git, Codex CLI, authentication usability, and writable local storage.
2. A real run starts four independent Codex sessions and completes without manual copy and paste.
3. The Coder changes only the isolated worktree; the source worktree and remote are untouched.
4. Every Node Run has a Context Snapshot, prompt, Codex JSONL stream, validated result, summary, and ordered events.
5. Tester evidence comes from the Archmage-owned check runner, includes every required validation command, and cannot report `passed` when a required check failed.
6. Reviewer consumes the persisted design, diff, and TestResult rather than hidden Codex history; `approved` and `changes_requested` remain distinct completion outcomes.
7. Killing the runtime during a node creates an `interrupted` attempt on restart; `resume` creates a new attempt and completes the workflow.
8. Automated domain, persistence, workspace, contract, and protocol parser tests pass.
9. The real Codex acceptance run in [the S0 runbook](../../runbooks/s0-acceptance.md) passes.

## Evidence produced

- CLI transcript and exit status
- SQLite Run, Node Run, Agent Run, Artifact, Context, and Event records
- Four distinct Codex thread IDs
- Design, implementation, test, review, diff, and final-report artifacts
- Recovery evidence showing interrupted and replacement attempts
- Source and run worktree Git status snapshots

## Next stage trigger

S1 starts only after S0 demonstrates that explicit artifacts and Context Snapshots are sufficient for real capability-agent collaboration. S1 then generalizes definitions and adds the bounded Review/Fix loop.
