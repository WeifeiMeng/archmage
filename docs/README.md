# Archmage Documentation

This directory is the maintained source of truth for product scope, concepts, architecture, delivery stages, decisions, and operating procedures. The original [product brief](../target.md) is preserved as an input, not treated as an executable specification.

## Recommended reading order

Latest development increment: [Online Doctor guide (中文)](guide-online-doctor.md).

**Want to run the current code?** Start with the [current usage guide (中文)](guide.md). It distinguishes working commands from planned capabilities.

1. [Product vision](product/vision.md)
2. [MVP definition](product/mvp-definition.md)
3. [Glossary](concepts/glossary.md)
4. [Collaboration model](concepts/collaboration-model.md)
5. [Architecture overview](architecture/overview.md)
6. [Delivery roadmap](roadmap/README.md)
7. [S0: Real Codex vertical slice](roadmap/stages/s0-real-codex-slice.md)
8. [Approved S0 design](superpowers/specs/2026-09-16-real-codex-vertical-slice-design.md)
9. [S0 implementation plan](superpowers/plans/2026-09-16-s0-real-codex-vertical-slice.md) — 14 tasks, four delivery batches, real Codex acceptance gates

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
