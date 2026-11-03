[简体中文](../../../zh-CN/acceptance/s0/batch-c-progress.md) | English

# Batch C: Capability Contracts and Check Runner

2026-09-17, `codex/s0-runtime-fencing`. This record covers Tasks 08–10 and does not claim that Batch C or S0 is complete.

## Implemented

- Four v1 prompts and strict JSON Schemas for Designer, Coder, Tester, and Reviewer.
- Rust rejects malformed JSON, missing fields, extra fields, invalid enums, and invalid schema versions.
- Designer design, criteria, and steps must be nonempty.
- Tester check IDs, evidence IDs, item status, and overall status must exactly match Runtime canonical evidence.
- Reviewer cannot approve failed checks, and issue paths cannot be absolute or escape the Workspace.
- Coder `blocked` is a valid Contract status; the canonical ChangeSet does not trust model-provided `changed_paths`.
- ContextRef allows only WorkflowInput, CommittedNodeOutput, Artifact, and WorkspaceSnapshot.
- Fixed role bindings and recursively canonical JSON; prompt version and the actual transmitted bytes enter the Snapshot hash.
- Schema v5 persists Context, Handoff, and attempt-scoped check evidence; each consumer attempt binds a specific `context_id`.
- Artifact resolution revalidates successful attempt, length, and SHA-256; tampered or uncommitted input cannot be consumed.
- Hidden `archmage internal check` sends only endpoint, attempt ID, and frozen check ID; it accepts neither arbitrary commands nor arbitrary data directories.
- Runtime validates the active attempt and owner generation, then uses `/bin/sh -c` to run the user-authorized check in the real worktree. stdin is closed, environment variables use an allowlist, and Provider authentication variables are not forwarded.
- Codex permission profile allows the check to write the worktree, exposes other paths as read-only, and denies access to the Codex credential directory. Tests verify sibling-source writes are rejected.
- Canonical evidence contains command/cwd/timeout, exit/signal, timestamps and duration, stdout/stderr hash and Artifact, and before/after Workspace Snapshots. Nonzero exit and timeout are evidence, not disguised transport failures.
- Unknown, expired, and other-attempt requests are rejected. Repeated completion requests return the same evidence without re-executing.
- Capability Schema `const`/`enum` nodes declare explicit types required by real Codex structured-output validation.

## Verification

```bash
cargo test -p archmage-core --test contracts
cargo test -p archmage-core --test context --test artifacts
cargo test -p archmage-core --test checks
cargo build -p archmage-cli
cargo test -p archmage-core --test checks_live \
  real_tester_requests_success_and_failure_evidence \
  -- --ignored --nocapture
```

The first three groups do not call a model; the last invokes a real Codex Tester. In the 2026-09-17 acceptance, the real Tester requested one exit-0 and one exit-7 check through the exact Unix-socket allowlist and produced a valid TestResult with `status=failed`. Context Snapshot represents only explicit Archmage input and Prompt; it does not claim deterministic replay of model output and does not include Codex's implicit system Prompt. The complete four-role chain and Ready Set scheduler were still unimplemented.
