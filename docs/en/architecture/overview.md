# Architecture Overview

## Boundary

Archmage starts as a Rust workspace with a UI-independent runtime and a thin CLI. Tauri and React are introduced only after the collaboration runtime is proven.

```text
archmage CLI
    │
    ▼
collaboration runtime
    ├── scheduler and state machine
    ├── capability and context resolver
    ├── contract validation
    ├── provider adapters
    ├── workspace manager
    ├── artifact store
    └── event and persistence store
          │                  │
          ▼                  ▼
     Codex CLI          Git + SQLite + files
```

The CLI depends on public runtime interfaces. The runtime has no dependency on Tauri, React, terminal presentation, or provider-specific UI concepts.

## S0 component boundaries

| Component | Responsibility | Must not own |
|---|---|---|
| CLI | Parse commands, stream events, render status, map exit codes | Workflow policy or persistence SQL |
| Runtime | Coordinate lifecycle, scheduling, state transitions, and recovery | Codex event details or shell presentation |
| Capability | Define role instructions, input/output contracts, permissions | Provider process management |
| Context Resolver | Resolve typed references and freeze immutable snapshots | Hidden provider history |
| Codex Provider | Detect CLI, invoke `codex exec`, parse JSONL, cancel processes | Workflow routing decisions |
| Workspace Manager | Create and inspect branches/worktrees, enforce leases | Capability prompts |
| Store | Apply migrations, persist state/events, manage artifact metadata | Business scheduling policy |

## S0 execution path

1. Validate the repository, Codex CLI availability, goal, schemas, and validation policy.
2. Persist the Run and its built-in workflow snapshot.
3. Create a run branch and isolated Git worktree from the source repository's current `HEAD`.
4. Build the Ready Set and dispatch compatible nodes.
5. For each node, freeze context, render the capability instruction, and invoke a new real Codex session.
6. Persist Codex JSONL events, validate the final JSON contract, create artifacts, and transition the node atomically.
7. Continue through Designer, Coder, Tester, and Reviewer, then write a final run report.

## Evolution

S1 externalizes workflow and capability definitions and introduces conditions, retries, Fixer loops, and human gates. S2 adds providers behind the same adapter boundary. S3 uses the runtime from Tauri commands and streams runtime events to React.
