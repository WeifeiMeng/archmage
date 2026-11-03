[简体中文](../../zh-CN/product/mvp-definition.md) | English

# MVP Definition

## Outcome

The MVP proves that Archmage can connect local agent providers and coordinate multiple capability agents around one repository through explicit, structured handoffs.

## MVP completion statement

Given a local Git repository and a development goal, Archmage can create an isolated workspace and coordinate Designer, Coder, Tester, Reviewer, and—after S1—Fixer capability agents. The system preserves their inputs, context, outputs, events, and workspace changes, and it completes the collaboration loop without manual copy and paste.

## Delivery interpretation

The MVP is delivered incrementally:

- **S0 proves the collaboration spine:** one real Codex provider, four capability agents, a fixed linear workflow, real Git isolation, contracts, persistence, and recovery.
- **S1 closes the loop:** externalized definitions, conditions, retry, Review-to-Fix looping, and human intervention.
- **S2 proves provider independence:** additional local providers and configurable capability bindings.
- **S3 productizes the runtime:** a Tauri desktop application exposes runs, context, timeline, artifacts, and workspaces.
- **S4 makes it usable as an alpha:** reliability, safety, packaging, and failure recovery are hardened.

## Success criteria

The MVP is complete when a user can:

1. Select a local Git repository and provide a goal.
2. Run capability agents without manually transferring their outputs.
3. Inspect the exact Context Snapshot, artifacts, execution log, and decision for every node.
4. Recover a persisted workflow after a process or desktop restart.
5. Keep concurrent workflow runs isolated in separate worktrees.
6. Stop at an explicit human decision when retry or loop limits are exhausted.

## Scope control

The original P0, P1, and P2 requirements are mapped to delivery stages in the [roadmap](../roadmap/README.md). A feature is not part of a stage merely because its supporting abstraction exists; it must be named in that stage's scope and exit criteria.
