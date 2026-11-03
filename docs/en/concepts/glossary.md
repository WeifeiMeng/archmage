[简体中文](../../zh-CN/concepts/glossary.md) | English

# Glossary

## Provider

A physical execution backend that can start and control agent sessions. Codex CLI is the only S0 provider.

## Capability

A versioned definition of professional work. It declares instructions, required context, output contract, permissions, and default execution configuration.

## Capability Agent

A logical collaboration role formed by binding a Capability to a Provider. Designer, Coder, Tester, and Reviewer are separate Capability Agents even when they share the same Codex CLI installation.

## Workflow Definition

A versioned graph of nodes, dependencies, context bindings, conditions, retry policy, and loop limits. S0 ships one built-in, fixed definition; S1 externalizes definitions.

## Workflow Run

An execution snapshot of a Workflow Definition, user input, baseline commit, configuration, and workspace reference.

## Node Run

The runtime state of one workflow node. A Node Run can have multiple attempts across failure and resume.

## Agent Run

One concrete provider invocation for a Node Run attempt. In S0 this is one `codex exec` process and its JSONL event stream.

## Context Reference

A typed declaration of where a node input comes from, such as workflow input, a prior node artifact, or a workspace snapshot.

## Context Snapshot

The immutable, content-addressed input view actually supplied to one Node Run attempt. It records values, source references, hashes, and rendering metadata.

## Artifact

A durable output such as Markdown, validated JSON, a Git diff, command evidence, or an event log. Artifact content lives on the filesystem; searchable metadata lives in SQLite.

## Handoff

The explicit transfer of selected artifacts and Context References from one Capability Agent to another. A Handoff is not a shared chat history.

## Workspace

The isolated Git worktree and branch assigned to one Workflow Run.

## Event

An ordered, immutable fact emitted by the runtime, such as `node.started`, `artifact.created`, or `node.failed`.

## Provider session

A provider-specific conversation or execution identity. S0 starts a fresh Codex session for each node and records its thread ID for observability only.
