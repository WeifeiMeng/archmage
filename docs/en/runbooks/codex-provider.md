# Codex Provider Runbook

## Purpose

This runbook defines S0 preflight and diagnostic expectations for the real local Codex CLI provider. It is a design-time contract until the `archmage doctor` command is implemented.

## Supported invocation surface

S0 depends on Codex non-interactive mode and verifies the live CLI rather than assuming a fixed installation path:

```bash
codex --version
codex exec --help
```

The required flags are `--cd`, `--sandbox`, `--json`, `--output-schema`, and `--output-last-message`. See the official [Codex non-interactive mode documentation](https://learn.chatgpt.com/docs/non-interactive-mode).

## Preflight checks

`archmage doctor` must report:

1. Resolved Codex executable path and version.
2. Availability of every required `codex exec` flag.
3. Git executable and version.
4. Archmage data-root path and writeability.
5. SQLite database open and migration status.
6. Authentication usability through a bounded, non-code-changing Codex probe.

The probe must not print or persist credential material. A failed probe reports a typed provider error and the sanitized stderr needed for diagnosis.

## Invocation policy

- Pass the complete prompt through stdin using the `-` sentinel.
- Set the worktree explicitly with `--cd`.
- Select `read-only` or `workspace-write` from the Capability permission.
- Enable `--json` and persist stdout as JSONL.
- Pass the versioned Capability schema with `--output-schema`.
- Write the final response to an attempt-owned path with `--output-last-message`.
- Never use `danger-full-access` in S0.
- Do not use `codex exec resume` for normal Handoffs or Archmage recovery.

## Diagnostic classification

| Symptom | Archmage classification | Operator action |
|---|---|---|
| Executable missing | `preflight_failed` | Install Codex CLI and retry `doctor` |
| Required flag missing | `preflight_failed` | Upgrade the local Codex CLI |
| Login or entitlement failure | `auth_failed` | Authenticate with Codex and rerun the bounded probe |
| Non-zero node exit | `cli_failed` | Inspect attempt stderr and JSONL events |
| No valid final JSON | `contract_invalid` | Inspect raw final message, schema, and prompt version |
| Runtime killed mid-run | `interrupted` after reconciliation | Run `archmage resume <run-id>` |

## Version evidence

Every Agent Run records the Codex CLI version. Acceptance reports include the version so protocol regressions can be reproduced.
