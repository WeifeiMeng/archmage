# Capability Topology and Management Design

**Status:** Awaiting written-spec review

## Goal and scope

Make the four built-in capabilities visible as a collaboration topology and configurable independently of the local Agent instances that execute them. This release displays the existing, fixed topology; it does not edit edges or change scheduling. Team management with an editable canvas is a subsequent, separate stage.

This follows the existing Capability-first boundary: a Capability describes a logical responsibility, an Agent instance supplies execution, and a future Team will own a reusable topology of capabilities.

## Product decisions

- Add a `Capabilities` destination beside `Runs` and `Agents`. `Agents` continues to manage local Codex instances and connection checks; move capability binding controls from `Agents` to `Capabilities` so each setting has one home.
- Show four nodes in a fixed, left-to-right graph: Designer, Coder, Tester, Reviewer. Selecting a node opens its configuration panel. Nodes and edges cannot be added, removed, dragged, or rewired in this release.
- Distinguish **execution dependency** from **context handoff**. Solid arrows show the scheduler chain `Designer → Coder → Tester → Reviewer`. Dashed arrows show additional data handoffs `Designer → Tester`, `Designer → Reviewer`, and `Coder → Reviewer`. Adjacent nodes also hand off context; the solid arrow represents both relations there. A legend explains this and the UI never presents a dashed arrow as an independent execution path.
- Keep the stable keys `designer`, `coder`, `tester`, and `reviewer` immutable. Each capability has an editable display name, description, supplemental instructions, and Agent-instance binding. The display name and description are UI metadata; supplemental instructions affect future invocations. The base prompt, output schema, required inputs, and topology are not editable.
- A Run freezes both the selected Agent binding and effective capability configuration at creation. Editing a capability cannot change an active or historical Run.
- The topology is the built-in default workflow, not a user-created Team. The later Team canvas may reuse its node/edge visual language but will have its own persisted model and design review.

## Approaches considered

1. **Recommended: fixed graph plus node configuration.** The graph truthfully visualizes today's runtime and adds a narrow configuration surface. It leaves workflow editing for Team management.
2. A fully editable graph now would imply new scheduler, validation, persistence, and Run-snapshot semantics. It would make the first Capability page substantially larger and blur the requested later Team stage.
3. A plain configuration table would be cheaper but would not answer the user's topology question or reveal the difference between dependency and context handoff.

## Data and API design

Core owns one `CapabilityConfiguration` per stable key:

```text
key                 immutable built-in capability ID
display_name        nonblank, bounded UI label
description         bounded UI explanation
extra_instructions  bounded text appended as a distinct prompt field
agent_instance_id   existing global binding to an enabled supported instance
```

The current `capability_bindings` table remains the authority for Agent routing. A new migration adds capability metadata/instructions keyed by the same four IDs and seeds defaults without replacing existing bindings. Unknown keys are rejected; all four built-in rows are required. Saving one configuration validates it atomically, including the referenced Agent instance. Unsupported or disabled instances remain unbindable under the existing Agent rules. No arbitrary prompt-file or schema editing is exposed.

The Core service returns a `CapabilityManagementSnapshot` containing the four configurations, the supported Agent choices and their connection states, and topology metadata. The topology metadata is defined once in Core alongside scheduler/context contracts rather than duplicated as an unrelated frontend truth. Tauri exposes typed read/save commands; React does not access SQLite or launch Agent processes directly.

At Run creation, the frozen definition stores for each capability its key, display name, description, extra instructions, built-in prompt version, and Agent binding. Historical definitions lacking these fields deserialize with the original built-in defaults. Dispatch renders the immutable base prompt plus the frozen extra instructions as separate manifest fields, so context hashes and persisted prompt snapshots reflect the exact settings used. The output JSON schema, required context inputs, and contract validation remain fixed. Existing Runs continue to render from their frozen definition, never from current global configuration.

## Desktop experience

The page has a topology area and a selected-node inspector:

- Each node shows its stable role, editable display name, bound Agent name/provider, and connection state. Shared use of one Codex instance by multiple capabilities is shown naturally; the graph represents capabilities, not separate processes.
- Solid/dashed arrow styles, labels, and a legend make the two edge meanings clear. A keyboard-accessible node list mirrors graph selection for non-pointer use. The graph remains legible at narrower window widths via horizontal scrolling rather than collapsing the edges into an ambiguous stack.
- The inspector edits display name, description, supplemental instructions, and Agent binding. It states that base instructions and output contract are built in, and that changes affect only future Runs. It shows an explicit Save action and unsaved-change state.
- Errors preserve the user's edits and name the failing field or binding. A failed read offers Retry. Browser-only preview may show the static topology, but saves and live Agent status require the Tauri Runtime; it must not display invented healthy connections.
- The `Agents` page links to `Capabilities` for routing. The Run creation page continues to validate all four bindings and can link to the new management page when a binding is missing.

## Failure and consistency rules

- A blank display name, over-limit field, unknown key, or unavailable Agent binding is rejected before persistence. Limits are 80 characters for display name, 500 for description, and 8,000 for extra instructions.
- If a configuration save fails, no partial metadata/binding update is visible. Concurrent reads see the previous or the fully saved configuration.
- A stale editor save must not silently overwrite a newer save: update carries a revision and reports a conflict requiring refresh.
- Deleting or disabling a bound Agent remains blocked until its capability bindings are changed.
- Changing supplemental instructions cannot bypass the fixed output schema or required-input validation. Runtime includes the instructions in the frozen prompt snapshot and diagnostic provenance for the new Run.

## Verification

- Core migration tests: upgrade an existing schema-8 database without losing Agent bindings; seed four defaults on a fresh database; reject unknown capability keys.
- Core service tests: validation, atomic save, revision conflict, and supported/enabled Agent binding checks.
- Runtime tests: new Runs freeze configuration and Agent selection; later edits do not affect them; old definitions deserialize and use original defaults; rendered context/hash includes frozen extra instructions.
- Frontend tests: graph metadata maps to four nodes and the documented edge styles; node selection, save/error/conflict behavior, and Run binding warnings.
- Keep the final verification focused on relevant Rust tests and frontend tests/build. Real Codex connectivity is already covered by Agent management and does not need a new live probe solely for this page.

## Next stage: Team canvas

Team management is a separate design and implementation stage. A Team will own a named, persisted topology on an editable canvas, including its capability nodes, edges, and validation rules; a Run will select and freeze a Team version. That stage must define whether edges are scheduling dependencies, data handoffs, or both, and how cycles, missing inputs, and multiple instances are handled. None of those editing semantics are implied by this fixed graph.
