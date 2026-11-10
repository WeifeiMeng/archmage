[简体中文](../../zh-CN/architecture/cross-provider-handoff.md) | English

# Cross-provider Capability handoff

This guide describes the handoff contract when adjacent Capabilities use Codex, DeepSeek Harness (DSH), or Qoder ACP. Future Provider adapters use the same boundary.

## Short answer

Capability A and Capability B do not open a direct chat. Archmage mediates every handoff:

```text
Capability A → Provider A returns JSON → Archmage validates and commits an artifact
             → Archmage queues a delivery → Context Resolver builds a snapshot
             → Provider B executes Capability B and returns JSON
```

The common machine language is **UTF-8 JSON constrained by JSON Schema**. Human-readable explanations may appear in fields such as `summary_markdown`, but prose is not the control protocol. Provider transports remain behind adapters.

## Three protocol layers

| Layer | Owner | Contract |
|---|---|---|
| Collaboration | Archmage Runtime | Deliverable, routing, artifacts, delivery ledger, Context Snapshot |
| Capability | Archmage validator | Versioned JSON Schema for Designer, Coder, Tester, Reviewer, or consultation |
| Provider transport | Provider adapter | Codex CLI JSONL/output file, DSH bridge JSONL/session frames, or Qoder ACP v1 JSON-RPC |

A Capability consumes the same logical context and satisfies the same output schema regardless of its Provider.

## Output envelope

Every successful message-flow execution returns exactly one envelope:

```json
{
  "deliverable": {
    "schema_version": "1",
    "summary_markdown": "Implemented the requested change.",
    "status": "implemented",
    "changed_paths": ["src/example.rs"],
    "notes": []
  },
  "routing": {"decision": "continue", "next": ["reviewer"]}
}
```

`deliverable` must match the executing Capability's schema. `routing` is validated separately:

- `continue` requires one or more unique direct downstream recipients;
- `complete` is allowed only for the configured terminal Capability and requires an empty `next`;
- Reviewer approval must agree with `complete`; a revision request must continue along a configured edge;
- blocked Coder or Tester output cannot be routed, and failed testing cannot complete a flow;
- malformed JSON, undeclared fields, contradictory states, or invalid recipients fail as `contract_invalid`.

The executable source of truth is [`contract.rs`](../../../crates/archmage-core/src/contract.rs).

## What Archmage commits and forwards

After validation, Archmage publishes the complete envelope as an immutable `application/json` artifact. It commits the Attempt, routing, artifact references, and downstream deliveries in one transaction. Each recipient gets an independent delivery pointing to the same committed result artifact; deliveries are not merged.

The target receives a bounded Context Snapshot that can include:

- `goal` and repository view;
- `incoming`, referencing the exact artifact carried by this delivery;
- `changeset`, when the source Coder published a canonical ChangeSet manifest;
- `previous_result`, when the target has an earlier committed result in this Run;
- frozen Agent binding, prompt version, Capability guidance, Skill/Tool hints, neighbors, terminal Capability, and delivery sequence.

Large content stays in the artifact store and is referenced by artifact ID and SHA-256 hash. The rendered snapshot and hash are persisted before invocation. See [`context.rs`](../../../crates/archmage-core/src/context.rs), [`runtime/flow.rs`](../../../crates/archmage-core/src/runtime/flow.rs), and [`store/flow.rs`](../../../crates/archmage-core/src/store/flow.rs).

## Codex to DSH example

Assume `coder` uses Codex and `reviewer` uses DSH:

1. Archmage sends Codex the rendered Coder context and flow schema.
2. Codex reads stdin, emits CLI JSONL lifecycle events, and writes the JSON selected by `--output-schema`.
3. Archmage validates it, publishes the result and Coder ChangeSet, and queues a delivery to `reviewer`.
4. The Reviewer snapshot contains the exact committed Coder result and ChangeSet reference, not an inferred summary of hidden Codex history.
5. Archmage sends the rendered task and the same logical schema through the DSH bridge.
6. DSH returns final assistant text through newline-delimited JSON frames. The adapter parses one JSON object; Archmage runs normal Capability and routing validation again.
7. If Reviewer routes a revision to `coder`, a new delivery carries that explicit Reviewer artifact as the next Coder invocation's `incoming`.

Reversing Provider bindings changes only the adapter transport, not the artifact or delivery contract.

## Conversation semantics

- Codex executions are independent. Continuity comes from artifacts and Context Snapshots, not shared conversation history.
- DSH keeps one live named conversation per `Run + Capability`; repeated calls to that Capability reuse its sidecar and session.
- Different Capabilities never share a DSH session, even when bound to the same Agent instance.
- An unexpectedly exited DSH sidecar is not silently replaced, because a new session would falsely appear continuous.
- Live DSH sessions are not restored after an Archmage restart; affected legacy processes converge to `interrupted`.
- Consultation uses a temporary isolated DSH session, outside the Capability's main conversation.
- Qoder likewise keeps one ACP process and session per `Run + Capability`; its consultation session is temporary and its file/terminal callbacks remain worktree-bound.

These Provider-specific session rules never replace the explicit handoff artifact.

## Failure ownership

| Failure | Result |
|---|---|
| Executable, SDK, credentials, timeout, or process failure | Provider adapter fails the Attempt and stores bounded diagnostics |
| Non-JSON or schema-invalid output | Contract layer rejects `contract_invalid` |
| Duplicate, unknown, or nonadjacent recipient | Routing validation rejects the Attempt |
| Artifact or ChangeSet publication failure | No downstream delivery is committed |
| Unexpected DSH exit | Sidecar is poisoned; continuity is not fabricated |
| Step limit reached | Runtime stops; message-flow Runs currently cap completed steps at 24 |

Never infer success from an exit code or prose. A handoff exists only after the validated artifact and delivery ledger entry are committed.

## Adding another Provider

A new adapter does not define a new agent-to-agent language. It must accept the Context Snapshot and JSON Schema, enforce read/write permissions, return one bounded JSON value with a nonblank execution identity, expose lifecycle and bounded diagnostics, and leave validation, artifacts, routing, and persistence to Archmage.

If a Provider cannot reliably return schema-constrained JSON, it is not compatible with this boundary.

## Related documents

- [Context and contracts](context-and-contracts.md)
- [Collaboration model](../concepts/collaboration-model.md)
- [Message-flow Team design](../superpowers/specs/2026-10-14-message-flow-teams-design.md)
- [DeepSeek Harness Provider design](../superpowers/specs/2026-10-30-deepseek-harness-provider-design.md)
- [Qoder ACP Provider runbook](../runbooks/qoder-provider.md)
