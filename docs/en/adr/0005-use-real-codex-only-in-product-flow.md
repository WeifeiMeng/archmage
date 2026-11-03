[简体中文](../../zh-CN/adr/0005-use-real-codex-only-in-product-flow.md) | English

# ADR 0005: Use Real Codex Only in the Product Flow

**Status:** Accepted

**Date:** 2026-09-16

## Context

S0 exists to validate real local-agent collaboration. A simulated provider could make the orchestration appear complete without proving Codex invocation, permissions, events, contracts, or workspace behavior.

## Decision

Every S0 Capability Agent invocation uses the installed, authenticated Codex CLI. The product has no Fake Agent, dry-run workflow, or simulated provider mode.

Fixed JSONL documents may be used as parser fixtures in automated tests. They cannot execute workflows or satisfy stage acceptance.

## Consequences

- Real acceptance requires Codex authentication, usage allowance, and a controlled repository.
- Most runtime logic remains testable without network calls through focused unit and integration tests.
- Release evidence must distinguish fixture-based parser tests from the real Codex end-to-end run.
