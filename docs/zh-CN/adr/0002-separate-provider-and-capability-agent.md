# ADR 0002: Separate Provider and Capability Agent

**Status:** Accepted

**Date:** 2026-09-16

## Context

The product must support multi-agent collaboration even when a user initially has only one local agent product installed.

## Decision

A Provider is the physical execution backend. A Capability Agent is a logical role created by binding a Capability to a Provider. S0 uses one Codex CLI Provider for four Capability Agents: Designer, Coder, Tester, and Reviewer.

## Consequences

- Multi-agent semantics do not require multiple vendor products or persistent instances.
- Provider configuration and capability contracts evolve independently.
- S2 can move a capability to another provider without changing Handoff consumers.
- UI and documentation must not use “Agent” ambiguously when Provider or Capability Agent is intended.
