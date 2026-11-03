[简体中文](../../zh-CN/adr/0004-use-per-run-git-worktrees.md) | English

# ADR 0004: Use Per-Run Git Worktrees

**Status:** Accepted

**Date:** 2026-09-16

## Context

Capability Agents need a shared coding reality, while concurrent Workflow Runs must not overwrite one another or the user's active checkout.

## Decision

Create one branch and Git worktree per Workflow Run from the source repository's persisted `HEAD`. All nodes in that Run share the worktree subject to read/write leases.

## Consequences

- Concurrent runs are isolated.
- Dirty source changes are excluded and must be reported clearly.
- S0 retains worktrees and branches for inspection and never merges or pushes automatically.
- Cleanup and branch integration remain explicit later features.
