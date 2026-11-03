[简体中文](../../../zh-CN/superpowers/specs/2026-11-03-cross-provider-handoff-guide-design.md) | English

# Cross-Provider Handoff Guide Design


**Status:** Approved for documentation implementation.

## Purpose

Add maintained, code-backed English and Simplified Chinese guides that explain how Archmage hands work from one Capability to another when the two Capabilities use different Agent providers, especially Codex and DeepSeek Harness (DSH). Both guides must answer three questions without requiring readers to inspect the Runtime:

1. What does Capability A actually deliver?
2. What context does Capability B actually receive?
3. Which parts of the protocol are provider-independent, and which parts belong only to the Codex or DSH adapter?

The guides document current behavior. They do not propose a new Runtime protocol, change Provider execution, or introduce MCP, ACP, shared provider history, or arbitrary Capability schemas.

## Location and documentation updates

Create two first-class technical guides:

- `docs/architecture/cross-provider-handoff.md` — English;
- `docs/architecture/cross-provider-handoff.zh-CN.md` — Simplified Chinese.

Architecture is the appropriate layer because the documents define current boundaries and invariants rather than an operating procedure or a future feature. Each guide will start with a reciprocal language link. The two versions will use the same heading structure, JSON examples, source links, and normative meaning; translation must not introduce different protocol requirements.

Link the guides from:

- `docs/README.md`, in the recommended reading order, with both language links on the same entry;
- `docs/superpowers/specs/2026-10-14-message-flow-teams-design.md`, beside the durable flow description;
- `docs/architecture/context-and-contracts.md`, as the detailed cross-provider explanation.

Also make two small consistency corrections:

- change the Message-Flow Teams design status from “implementation in progress” to an implemented status that points readers to the implementation plan for verification history;
- replace the desktop guide's statement that every delivery executes “real Codex” with provider-neutral wording describing the Agent bound to the target Capability, and link that Chinese guide to the Simplified Chinese protocol document.

## Guide structure

Both language versions will contain these sections:

1. **Protocol boundary** — Archmage, not either provider, is the coordinator. Capability Agents do not establish a direct Codex-to-DSH session.
2. **Common language** — UTF-8 JSON constrained by versioned JSON Schema is the machine contract. Markdown and natural language are allowed only inside declared human-readable fields.
3. **Delivery envelope** — document `deliverable` and `routing`, including `continue`, `complete`, and `next` constraints.
4. **Persisted delivery record** — explain the FlowDelivery identity, source Attempt, target Capability, sequence, payload Artifact, state, executing Attempt, and routing decision.
5. **Receiver context** — show `goal`, `repo_view`, `incoming`, optional `changeset`, and optional `previous_result`. Explain the Artifact wrapper containing `artifact_id`, `sha256`, and `content`.
6. **End-to-end example** — trace Coder/Codex → Reviewer/DSH and a Reviewer/DSH → Coder/Codex feedback cycle with representative JSON.
7. **Provider adapters** — separate the common business contract from Codex CLI stdin/schema/result handling and the DSH newline-delimited JSON sidecar protocol.
8. **Conversation and workspace boundaries** — state that downstream Capabilities do not receive upstream hidden conversation history. Explain independent Codex executions, per-Run/per-Capability persistent DSH conversations, the shared isolated Git worktree, and canonical Coder ChangeSets.
9. **Validation and failure semantics** — cover malformed JSON, schema violations, illegal recipients, duplicate or empty routes, unauthorized completion, role/routing contradictions, blocked output, and the flow step limit.
10. **Compatibility and non-goals** — distinguish current message-flow Teams from legacy consultation Teams and state that the protocol is not MCP, ACP, or an OpenAI SDK handoff primitive.
11. **Sources of truth** — link to the role schemas, contract validation, context rendering, flow Runtime, delivery store, Codex adapter, and DSH adapter.

## Normative example

The guides will use a Coder handoff as the main example because it exercises both the structured result Artifact and the separate canonical ChangeSet:

```json
{
  "deliverable": {
    "schema_version": "1",
    "summary_markdown": "Implemented the requested change.",
    "status": "implemented",
    "changed_paths": ["src/example.rs"],
    "notes": []
  },
  "routing": {
    "decision": "continue",
    "next": ["reviewer"]
  }
}
```

The receiver example will show that the complete validated envelope is placed in the `incoming` Artifact content rather than reducing the handoff to prose or `summary_markdown`. It will separately show the Coder ChangeSet and clearly label IDs and hashes as illustrative. The JSON field names and enum tokens remain identical in both language versions; explanatory prose and human-readable example values may be translated.

## Accuracy rules

- Describe behavior from current code, not from provider marketing or hidden implementation assumptions.
- Use “must” only for constraints enforced by schema or Runtime validation.
- Distinguish the Capability-to-Capability contract from adapter transport details.
- Do not imply that `changed_paths` is canonical; the Runtime-created ChangeSet is authoritative.
- Do not imply that a persistent DSH conversation is shared across Capabilities or survives an Archmage restart.
- Do not imply that either provider can route outside the frozen Team graph.
- Do not expose credentials, endpoints, local data values, or real Artifact identifiers.
- Keep English and Simplified Chinese claims semantically aligned. When implementation details change, update both documents in the same change.

## Verification

Because this is documentation-only work, verification will include:

- inspect every referenced implementation symbol and line before final wording;
- scan the new and updated documents for stale claims, placeholders, and contradictions;
- compare the English and Simplified Chinese headings, examples, constraints, and source links for semantic parity;
- verify that all relative Markdown links resolve;
- run `git diff --check`;
- review the final diff to confirm that no Runtime or schema files changed.
