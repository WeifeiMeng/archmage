[简体中文](../../zh-CN/product/vision.md) | English

# Product Vision

## Purpose

Archmage is a local-first collaboration runtime for software development. It connects agent providers already available on a user's machine and organizes capability-oriented agents around a shared repository without requiring the user to copy output between tools.

The first product proof is one command that turns a goal into a traceable sequence of design, implementation, validation, and review inside an isolated Git worktree.

## Product model

```text
Provider          Executes agent sessions, for example Codex CLI
Capability Agent  Performs one professional role with its own instruction and contract
Workflow          Coordinates capability agents and their dependencies
Handoff           Transfers explicit artifacts and context between agents
Workspace         Provides the shared Git reality for code-changing work
```

Multiple collaborating agents do not require multiple vendor products or long-lived processes. Designer, Coder, Tester, and Reviewer are distinct logical agents even when all four sessions are executed by the same local Codex provider.

## Principles

1. **Local-first.** Source access, agent execution, Git operations, logs, artifacts, and state remain local by default.
2. **Capability-first.** Workflows depend on capabilities, not vendor-specific agent names.
3. **Explicit context.** Every node receives a reproducible Context Snapshot instead of inheriting an opaque conversation history.
4. **Contract-driven handoff.** Human-readable artifacts and machine-validated results travel together.
5. **Workspace as shared reality.** Code-changing agents coordinate through a real repository, baseline commit, branch, worktree, and diff.
6. **Observable decisions.** The runtime records who ran, what they saw, what they produced, and why the workflow advanced or stopped.

## Product non-goals for the MVP

- Building a new foundation model or hosted agent service
- Cloud SaaS, organizations, or multi-user collaboration
- Automatic provider selection or agent marketplaces
- Semantic memory, RAG, or implicit cross-run memory
- A free-form DAG canvas
- Automatic merge or push to a user's branch
