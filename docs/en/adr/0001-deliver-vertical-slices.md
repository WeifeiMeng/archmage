# ADR 0001: Deliver Vertical Slices

**Status:** Accepted

**Date:** 2026-09-16

## Context

The original brief orders horizontal foundations before real agent execution and product validation. That sequence can produce generalized abstractions before the collaboration contract is tested against a real provider and repository.

## Decision

Deliver S0–S4 as runnable vertical stages. S0 crosses the original M0–M5 layers to prove one real Codex collaboration path. Later stages generalize only after evidence from earlier stages.

## Consequences

- S0 must implement thin versions of persistence, workspace, capability, context, contract, and workflow concerns together.
- Some S0 definitions are built in and deliberately not user-editable.
- The roadmap maintains explicit traceability to M0–M7 and P0–P2 so resequencing does not drop requirements.
