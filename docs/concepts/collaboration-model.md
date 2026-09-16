# Collaboration Model

## Logical agents over provider instances

Archmage models collaboration through distinct capabilities rather than requiring multiple vendor products or persistent agent processes.

```text
Codex CLI Provider
  ├── Designer Capability Agent
  ├── Coder Capability Agent
  ├── Tester Capability Agent
  └── Reviewer Capability Agent
```

Each Capability Agent has its own instruction, permissions, input contract, output contract, Context Snapshot, and provider session. Sharing a provider does not merge these boundaries.

## Communication

Capability Agents communicate through Handoffs:

```text
workflow input
  → Designer → Design artifacts
  → Coder    → Workspace diff and implementation result
  → Tester   → Test evidence and TestResult
  → Reviewer → ReviewResult
```

The runtime resolves Context References, freezes a Context Snapshot, renders the capability prompt, invokes the provider, validates the result, and publishes artifacts. A downstream node never reads an upstream provider's hidden conversation history.

## Why sessions are independent

- The UI can show exactly what an agent saw.
- A failed attempt can be replayed from the same inputs.
- Provider-specific history does not become an undocumented dependency.
- A capability can later move to another provider without changing its consumers.

The accepted cost is repeated context transfer. Archmage favors reproducibility and provider independence over opaque session continuity.

## Shared workspace rules

Nodes declare `none`, `read`, or `write` workspace access. The scheduler may execute multiple ready nodes concurrently when their dependency and access constraints are compatible. A write node holds an exclusive lease for the run's worktree; `none` nodes can execute alongside it, and multiple readers may execute together against frozen Context Snapshots.
