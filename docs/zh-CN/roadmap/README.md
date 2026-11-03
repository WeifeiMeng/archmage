# Delivery Roadmap

## Strategy

The original brief separates the system into horizontal milestones M0–M7. Archmage will instead deliver vertical stages S0–S4 so every stage produces a runnable, testable increment. This changes sequencing, not product intent.

## Stages

| Stage | Runnable outcome | Depends on | Exit criterion |
|---|---|---|---|
| [S0](stages/s0-real-codex-slice.md) | Real Codex provider coordinates Designer, Coder, Tester, and Reviewer in a fixed workflow | None | One command completes and recovers a real repository run with auditable handoffs |
| S1: Closed-loop workflow | Definitions become externalized; condition, retry, Fixer loop, and human gate work | S0 | Review can approve or route through bounded Fix/Test/Review iterations |
| S2: Provider expansion | Capabilities can bind to Claude Code, Qoder, or Codex through adapters | S1 | One workflow demonstrably uses at least two providers without changing capability contracts |
| S3: Desktop MVP | Tauri and React expose project, run, node context, timeline, workspace, and provider management | S2 | The S1 workflow can be started, observed, interrupted, and resumed from the desktop |
| S4: Alpha hardening | Failure recovery, permissions, packaging, diagnostics, and observability meet alpha quality | S3 | The defined failure matrix and release checklist pass on supported platforms |

## Original milestone traceability

| Original brief | Primary delivery | Notes |
|---|---|---|
| M0 Domain & Runtime Foundation | S0, S1 | S0 implements only the model needed by the real vertical slice; S1 generalizes definitions and control flow |
| M1 Agent Execution Layer | S0, S2 | S0 proves Codex; S2 proves the adapter architecture with additional providers |
| M2 Workspace Runtime | S0 | Real per-run Git worktrees are required in the first slice |
| M3 Capability & Contract | S0, S1 | S0 ships four built-in capabilities; S1 externalizes versioned definitions |
| M4 Context + Workflow Engine | S0, S1 | S0 provides explicit context and scheduling; S1 adds conditions, loops, retry, and human gates |
| M5 Coding Collaboration Loop | S0, S1 | S0 proves Design/Code/Test/Review; S1 adds the Fix loop |
| M6 Desktop Product MVP | S3 | Runtime remains independent of Tauri |
| M7 Stability & Alpha | S0, S4 | S0 establishes crash-safe invariants; S4 expands the failure matrix and release quality |

## Priority traceability

| Original priority | Delivery treatment |
|---|---|
| P0 | Collaboration spine in S0; closed loop in S1; runtime views in S3 |
| P1 | Human gate and permissions begin in S1; provider discovery and usage arrive in S2; IDE actions arrive in S3 |
| P2 | Remains outside S0–S4 unless a later approved roadmap revision promotes it |

## Stage document rule

Only the current or next stage is expanded into a detailed spec and implementation plan. Later stages keep a goal, dependency set, scope boundary, and exit criterion until earlier evidence makes detailed design responsible.
