# S0 Real Codex Acceptance Runbook

## Purpose

This procedure is the release gate for S0. It must use an installed, authenticated Codex CLI and must not substitute a simulated provider.

## Preconditions

- `archmage doctor` passes.
- A disposable local Git repository exists with at least one commit.
- The repository has a deterministic validation command.
- The requested change is small, observable, and safe, such as adding a health endpoint with a test.
- The source repository has no credentials or production-only data needed by the change.

## Scenario A: complete collaboration flow

1. Record the source repository's branch, `HEAD`, worktree status, and configured remotes.
2. Run:

   ```bash
   archmage run \
     --repo <acceptance-repository> \
     --goal "<small acceptance goal>" \
     --check "<deterministic validation command>"
   ```

3. Confirm four different Codex thread IDs exist for Designer, Coder, Tester, and Reviewer.
4. Confirm every node has a Context Snapshot, rendered prompt, JSONL event stream, validated JSON result, and Markdown summary.
5. Confirm Coder changed the run worktree and that the recorded diff is relative to the persisted baseline.
6. Confirm Tester invoked every required check through the Archmage-owned check runner and that its attempt-scoped evidence matches TestResult.
7. Confirm Reviewer received the design, diff, and TestResult through Context bindings.
8. Confirm the final report links all four node outcomes and the worktree location, and preserves Reviewer `approved` or `changes_requested` as the completion outcome.
9. Recheck the source worktree and remotes; they must be unchanged.

## Scenario B: interruption and resume

1. Start the same workflow with a goal that keeps Tester active long enough to interrupt.
2. Terminate the Archmage runtime process while Tester is running.
3. Restart the CLI and inspect the Run.
4. Confirm the old Tester attempt is reconciled to `interrupted` and its incomplete output is not Context-eligible.
5. Run `archmage resume <run-id>`.
6. Confirm a new Tester attempt and a new real Codex thread are created.
7. Confirm the workflow finishes without re-running already successful upstream nodes.

## Scenario C: offline contract regression (not real-provider acceptance)

1. Run contract unit tests with a malformed final JSON fixture. Test schema compilation failures separately as preflight failures.
2. Confirm that the result-acceptance function rejects invalid output even when given a successful exit status.
3. Test the failed state transition and dependency-readiness rules against the real temporary database. These are component tests, not a synthetic Agent workflow, and do not satisfy Scenarios A or B.

The [implementation plan](../superpowers/plans/2026-09-16-s0-real-codex-vertical-slice.md) also requires real-provider cancellation, dirty-source, failed-check, and concurrent-reader evidence before S0 completion.

## Acceptance record

The release record contains:

- Archmage commit and build profile
- Operating system, Git version, and Codex CLI version
- Run IDs for all scenarios
- CLI exit codes
- Automated test command and result
- Artifact paths for full flow and recovery evidence
- Source repository before/after Git evidence
- Any deviation, which fails the gate until resolved or explicitly changes the approved S0 specification
