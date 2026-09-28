# Archmage Documentation

This directory is the maintained source of truth for product scope, concepts, architecture, delivery stages, decisions, and operating procedures. The original [product brief](../target.md) is preserved as an input, not treated as an executable specification.

## Recommended reading order

Latest development increment: [Desktop runtime vertical slice](superpowers/specs/2026-09-28-desktop-runtime-vertical-slice-design.md).

**Want to run the current code?** Use the [CLI command reference (中文)](cli-reference.md) or [desktop client guide (中文)](runbooks/desktop-s0.md). The [current-version guide (中文)](guide.md) adds contributor tests and architecture details.

1. [CLI command reference](cli-reference.md)
2. [Desktop client guide](runbooks/desktop-s0.md)
3. [Product vision](product/vision.md)
4. [MVP definition](product/mvp-definition.md)
5. [Glossary](concepts/glossary.md)
6. [Collaboration model](concepts/collaboration-model.md)
7. [Architecture overview](architecture/overview.md)
8. [Delivery roadmap](roadmap/README.md)
9. [S0: Real Codex vertical slice](roadmap/stages/s0-real-codex-slice.md)
10. [Desktop runtime implementation plan](superpowers/plans/2026-09-28-desktop-runtime-vertical-slice.md)

## Documentation layers

| Layer | Purpose | Change policy |
|---|---|---|
| `product/` | Why the product exists and what counts as success | Update when product scope changes |
| `concepts/` | Stable vocabulary and collaboration semantics | Update before introducing conflicting terms |
| `architecture/` | Current system boundaries and invariants | Keep synchronized with approved designs and code |
| `roadmap/` | Delivery order, scope, dependencies, and exit criteria | Update when sequencing changes |
| `adr/` | Durable records of consequential decisions | Add a new ADR to supersede an old decision |
| `runbooks/` | Repeatable setup, diagnosis, and acceptance procedures | Verify against the live CLI before releases |
| `superpowers/specs/` | Approved stage or feature designs | Review before an implementation plan is written |
| `superpowers/plans/` | File-level implementation plans | Create only after the corresponding spec is approved |

## Document status

Documents use explicit status language in their opening section where it matters. `Approved` means the design decision is accepted; it does not claim that the software is implemented. Roadmap stages distinguish planned scope from completed scope.
