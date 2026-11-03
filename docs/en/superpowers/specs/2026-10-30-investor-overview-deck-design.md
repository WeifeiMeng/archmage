# Archmage Investor Overview Deck Design

## Objective

Create a five-slide Chinese presentation that explains Archmage to investors. The deck should make the product value understandable before introducing the technical architecture.

## Audience and message

The audience is investors evaluating the problem, product thesis, differentiation, and scalability of the system. The central message is:

> Archmage turns isolated local coding agents into a traceable collaboration system around one real code workspace.

The deck must not claim market size, revenue, adoption, or performance numbers that the repository does not support.

## Narrative

1. **Product introduction** — Archmage is a local-first collaboration runtime for software development. It connects existing agent providers and organizes capability-oriented agents around one repository.
2. **Collaboration pain points** — Today, users manually move context between tools, agent decisions remain opaque, multiple code-changing tasks can conflict, and execution evidence is fragmented.
3. **Product thesis** — The design is capability-first, uses explicit context snapshots and contract-driven handoffs, treats the Git workspace as shared reality, and records observable decisions.
4. **Solution flow** — A user provides a repository and goal. Archmage coordinates design, coding, testing, review, and repair without manual copying, while keeping the run inspectable and recoverable.
5. **Architecture and defensibility** — A thin desktop and CLI layer sit above a Rust collaboration runtime. Provider adapters, context and contract services, workspace isolation, and local persistence create provider independence and reproducible execution.

## Visual direction

- 16:9 layout with a warm off-white background.
- Near-black text plus one muted green accent sampled from the current Archmage visual language.
- One idea per slide, large type, generous whitespace, and no decorative card grids.
- Cover contains only the Archmage mark, name, and one-line positioning.
- Diagrams use the fewest necessary nodes, thin strokes, and short labels.
- No gradients, textures, stock imagery, ornamental icons, or dense UI screenshots.
- Minimum body size is 18 pt; slide titles use at least 32 pt.

## Content constraints

- Use only claims supported by the repository's product and architecture documentation.
- Present the current executable path accurately: Codex provider, capability agents, explicit handoffs, isolated Git worktree, local persistence, CLI, and Tauri desktop workbench.
- Describe multi-provider support as an architectural direction rather than a completed current capability.
- Keep all text and diagrams editable in PowerPoint.

## Deliverable and verification

The final deliverable is one five-slide `.pptx` file. Verification covers slide count, text fit, visual overlap, consistency with the minimal design, editability, and successful presentation-file validation.
