# Runtime and State

## Definition and execution separation

A Workflow Definition describes nodes and dependencies. A Workflow Run snapshots the definition version, inputs, baseline commit, provider binding, and execution policy. Editing a later definition never changes an existing run.

A Node definition names one Capability and its Context bindings. A Node Run records execution state. Every retry or resume creates a new attempt; attempts are never overwritten.

## Node states

```text
pending → ready → running → succeeded
                     ├────→ failed
                     ├────→ interrupted
                     └────→ cancelled
```

`succeeded` requires all of the following:

- The provider process completed successfully.
- The final output parsed as JSON.
- JSON Schema validation passed.
- Capability-specific invariants passed.
- Required artifacts were durably registered.

## Scheduler

The scheduler derives a Ready Set from persisted state. A node becomes ready only when all required predecessors have succeeded and its Context References can resolve.

The Run does not have a global single-node restriction. Dispatch is limited by:

- `run.max_parallelism`
- dependency readiness
- provider capacity
- workspace access compatibility

Workspace access compatibility is:

| Running access | Candidate `none` | Candidate `read` | Candidate `write` |
|---|---:|---:|---:|
| `none` | allow | allow | allow |
| `read` | allow | allow | wait |
| `write` | allow | wait | wait |

The S0 workflow is linear because of its data dependencies, but the scheduler and lease model are not hard-coded to serial execution.

## Atomic transitions

Every state transition and its corresponding Event are committed in one SQLite transaction. Consumers must reconstruct runtime state from persisted entities, not from in-memory events alone.

## Recovery

At startup, the runtime scans persisted `running` attempts. If the recorded process cannot be confirmed as an owned live process, the attempt becomes `interrupted` and an event records the recovery decision.

`archmage resume <run-id>` rebuilds the Ready Set from persisted state. It preserves successful nodes, ignores incomplete artifacts, and creates a new attempt for interrupted or retryable failed nodes. S0 recovery is explicit; there is no daemon that resumes work automatically.

## Cancellation

Cancellation targets the owned provider process group, waits for a bounded grace period, then forces termination if necessary. The runtime records the signal path and marks the attempt and downstream run state consistently. A cancelled attempt cannot publish a successful result after cancellation wins the state transition.
