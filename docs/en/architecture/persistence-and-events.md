[简体中文](../../zh-CN/architecture/persistence-and-events.md) | English

# Persistence and Events

## Storage split

SQLite stores transactional state and searchable metadata. The local filesystem stores potentially large or stream-oriented content.

```text
<archmage-data>/
├── archmage.db
├── workspaces/<run-id>/
└── runs/<run-id>/
    ├── workflow.json
    ├── nodes/<node-id>/attempt-<n>/
    │   ├── context.json
    │   ├── prompt.txt
    │   ├── events.jsonl
    │   ├── result.json
    │   └── summary.md
    └── artifacts/
```

The data root defaults to the operating system's application data directory and can be overridden with `ARCHMAGE_HOME`.

## SQLite responsibilities

S0 persists at least Project, Provider, Capability snapshot, Workflow Run, Node Run, Agent Run, Workspace, Context Snapshot metadata, Artifact metadata, and Event.

The database runs with WAL enabled, foreign keys enabled, migrations tracked in a schema table, and bounded busy timeouts. State changes and their Events share one transaction.

## Artifact durability

Artifact bytes are written to a temporary file in the destination directory, flushed, and atomically renamed. Only then is metadata inserted. A file not referenced by committed metadata is an orphan and never participates in Context resolution.

The store verifies content length and hash when reading a referenced artifact. Cleanup is explicit and outside S0's normal run completion path.

## Event model

Events are append-only, ordered within a Run, and contain a stable type, run identity, optional node and attempt identity, timestamp, schema version, and typed payload.

S0 event families include:

- `workflow.created`, `workflow.started`, `workflow.completed`, `workflow.failed`, `workflow.cancelled`
- `workspace.created`, `workspace.changed`
- `node.ready`, `node.started`, `node.succeeded`, `node.failed`, `node.interrupted`, `node.cancelled`
- `agent.started`, `agent.event`, `agent.exited`
- `context.snapshotted`, `artifact.created`

In-process subscribers receive events only after the transaction commits. The persisted Event log is authoritative for restart, CLI status, and the future desktop Timeline.
