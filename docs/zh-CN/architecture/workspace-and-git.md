# Workspace and Git

## Run baseline

At run creation, Archmage resolves the source repository's canonical path and current `HEAD`. That commit becomes the immutable baseline for the run.

Uncommitted changes in the source worktree are not copied into the run workspace. Their presence is recorded as a warning so users understand that the run starts from `HEAD`, not from their dirty working tree.

## Worktree lifecycle

Each Workflow Run receives:

```text
branch:    archmage/run-<run-id>
worktree:  <archmage-data>/workspaces/<run-id>
baseline:  <source-repository HEAD at creation>
```

Creation is idempotent against the persisted workspace record. Branch or directory collisions fail closed unless the existing resources can be proven to belong to the same Run.

S0 leaves the branch and worktree intact after success, failure, or cancellation and prints their locations. It never automatically commits, merges, pushes, removes the worktree, or deletes the branch.

## Access modes and lease

Every node declares `none`, `read`, or `write` access. The scheduler grants an exclusive workspace lease to a writer. Readers use frozen Context Snapshots and cannot overlap a live writer on the same worktree.

S0 mappings are:

| Capability Agent | Access | Codex sandbox |
|---|---|---|
| Designer | `read` | `read-only` |
| Coder | `write` | `workspace-write` |
| Tester | `write` | `workspace-write` |
| Reviewer | `read` | `read-only` |

Tester is a writer because builds and tests commonly create local files even when source code should remain unchanged. A post-test diff invariant records and classifies any source changes made during validation.

## Safety invariants

- Provider processes run with the worktree as their working root.
- S0 never uses `danger-full-access`.
- The source repository's checked-out branch is not changed.
- Capability instructions forbid Git commits, and the run worktree's `HEAD` must remain equal to the persisted baseline throughout S0.
- Diff artifacts are computed against the persisted baseline.
- The runtime records branch, worktree path, baseline, and current commit before every write-capable node.
- Credentials are inherited through the provider's supported authentication mechanism and are never copied into prompts, events, or artifacts.
