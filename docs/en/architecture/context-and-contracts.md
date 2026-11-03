[简体中文](../../zh-CN/architecture/context-and-contracts.md) | English

# Context and Contracts

## Context binding

A Capability declares what it needs. A workflow node declares where each input comes from.

Examples:

```text
goal        ← workflow.input.goal
design      ← nodes.design.artifacts.design_json
workspace   ← workflow.workspace.snapshot
test_result ← nodes.test.output
diff        ← workflow.workspace.diff_from_baseline
```

The Context Resolver dereferences these bindings, validates their types, records content hashes, and writes an immutable Context Snapshot before provider invocation.

## Snapshot contents

Each snapshot records:

- Run, node, and attempt identity
- Capability ID and version
- Prompt template version
- Provider binding and execution settings
- Resolved scalar inputs
- Artifact IDs, relative paths, media types, and content hashes
- Repository baseline and workspace state identifiers
- Creation timestamp and canonical serialization version

Large content remains in artifacts and is referenced by hash. The rendered provider prompt and its hash are also retained so the effective input can be audited.

## Output contracts

S0 uses one JSON Schema per Capability. Every schema is versioned and rejects undeclared fields where practical. The Codex adapter passes the schema through `codex exec --output-schema`, then the runtime validates the returned JSON again before accepting it.

The result includes a `summary_markdown` field for human-readable output. The runtime extracts it into a Markdown artifact while preserving the validated JSON as the machine contract.

## S0 handoffs

| Producer | Required outputs | Consumer |
|---|---|---|
| Designer | Design JSON, design Markdown | Coder, Tester, Reviewer |
| Coder | ImplementationResult, workspace diff | Tester, Reviewer |
| Tester | TestResult, command evidence | Reviewer |
| Reviewer | ReviewResult | Final run report |

The Reviewer result is terminal in S0. S1 adds an `approved` or `changes_requested` condition and the Fixer loop.

The built-in Tester receives the testing task rather than a predeclared command list. Its strict result records every command it relied on, exit codes, concise outcomes, and limitations. A `passed` result requires at least one successful command and cannot contradict its command outcomes; failed or blocked testing prevents Reviewer approval. The older Archmage-owned CheckRunner remains available as lower-level infrastructure but is not used by this goal-driven built-in workflow.

## Contract failure

Valid process exit is necessary but insufficient. Parse failure, schema mismatch, missing artifacts, contradictory status, or a workspace invariant violation marks the attempt `contract_invalid` or `workspace_invariant_failed`. The runtime stores the raw final message and validation diagnostics but does not infer success from prose.
