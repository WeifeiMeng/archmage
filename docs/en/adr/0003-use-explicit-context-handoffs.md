[简体中文](../../zh-CN/adr/0003-use-explicit-context-handoffs.md) | English

# ADR 0003: Use Explicit Context Handoffs

**Status:** Accepted

**Date:** 2026-09-16

## Context

Sharing provider conversation history makes it difficult to explain what a downstream agent saw, reproduce an attempt, or move a capability to another provider.

## Decision

Every Node Run receives an immutable Context Snapshot built from typed references and committed artifacts. Every S0 node starts a new Codex session. Provider session history is not a Handoff channel.

## Consequences

- Context is auditable, hashable, and replayable.
- Handoffs remain provider-neutral.
- Inputs may be transferred repeatedly and consume more tokens.
- Prompt rendering and Artifact design become first-class runtime responsibilities.
