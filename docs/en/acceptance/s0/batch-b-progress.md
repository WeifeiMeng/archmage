[简体中文](../../../zh-CN/acceptance/s0/batch-b-progress.md) | English

# Batch B: Storage foundation checkpoint

2026-09-17, `feat/s0-codex`. This checkpoint is not a claim that Batch B or S0 is complete.

## Implemented

- SQLite schema v3, transactional migrations, WAL, foreign keys, FULL synchronous mode, and a five-second busy timeout.
- Separate Run, NodeRun, and attempt records; Runtime-generated UUIDs; one unique active attempt.
- Revision/state CAS; state and events committed in the same transaction.
- Monotonic per-Run event sequence and cursor queries; active-attempt queries are read-only.
- Private Artifact files, publish after sync, SHA-256/length verification, and metadata plus `artifact.created` in one transaction.
- Only Artifacts from successful attempts are readable as committed input; unregistered files cannot be referenced.
- Online and offline Doctor open `archmage.db` and return real database inspection results.
- Per-Run Workspace provisioning intent; failed Git creation preserves intent and does not delete colliding paths.
- Real branch/worktree creation from a committed baseline; dirty source changes do not enter the Run worktree.
- Content-level Snapshot covers index diff, working diff, tracked/untracked/binary files, and symlink targets.
- Provisioning reconciliation after restart accepts a registered worktree only when path, branch, baseline, and common-dir all match.
- ChangeSet publishes tracked binary patches, each untracked file's contents, and a canonical manifest as attempt-owned Artifacts.
- Unix attempts use a separate session/process group; cancellation validates PID/PGID/start identity and performs TERM, grace, KILL, and wait confirmation.
- One OS owner lock per Run excludes competing schedulers; database owner-generation fencing rejects stale owners registering processes or committing terminal state.

## Verification scope

Tests use real temporary SQLite rather than simulated storage. They cover reopen, repeated migrations, invalid schema, concurrent claim, repeated completion, foreign-key failure, rollback after event-insertion failure, Artifact tampering, missing files, out-of-range IDs, symlink roots, and orphan files after registration failure. Real temporary Git repository tests also cover dirty-source isolation, paths with spaces, two Runs, unborn HEAD, colliding intent, partial-provision recovery, and ChangeSet Artifacts. Ordinary shell tests cover dual-stream output, limits, timeout, wrong-identity rejection, TERM grace, and killing the whole process tree after TERM is ignored.

## Explicitly not implemented

- Complete persistence models for Project/Provider/Capability/Context/Handoff/check/control.
- Atomic commit of terminal state, Contract-validated output, and Handoff; `finish_attempt` was only an internal ledger API whose caller had to validate the Contract first, not a user-facing Workflow entry point.
- Cross-node scheduling and Runtime recovery orchestration.
- Process-crash injection matrix around atomic file publication and adversarial-filesystem TOCTOU hardening.
- Source before/after invariants and a manual diagnostic entry point for non-exact partial provisioning.

Opening the database only performs SQL queries and persistence; it does not automatically recover or start Agents. CLI business wiring for `run`, `status`, `logs`, `resume`, and `cancel` was still absent. Nothing was automatically merged to main or pushed.
