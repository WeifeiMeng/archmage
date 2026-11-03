# Message-Flow Teams Design

**Status:** Approved design; implementation in progress. This supersedes the relationship and scheduling semantics in [Selectable Team Capabilities](2026-10-14-selectable-team-capabilities-design.md) for new flow teams only.

## Purpose and scope

A Team is a directed graph of selected Capabilities. Its arrows carry deliverables and feedback, not consultation permissions or prerequisites. A Capability's real Agent chooses one or more connected recipients after each execution. Each delivered message triggers one fresh execution of its recipient; messages are never implicitly joined. Cycles such as Coder → Reviewer → Coder are valid. A Team explicitly designates one entry Capability and one Capability authorized to emit the terminal signal. Users create their own Teams; no Coder–Reviewer Team is seeded.

The first version keeps the four built-in Capability types and one Agent binding per Capability configuration. It does not add arbitrary node types, editable edge conditions, a general workflow expression language, or concurrent writes to one Git worktree.

## Team configuration and canvas

New flow Teams select at least two distinct Capabilities, an entry among them, a terminal emitter among them, and directed edges between selected Capabilities. Self-loops and duplicate directed edges are invalid; opposite directions are independent and may coexist. Every selected Capability must be reachable from the entry and must have a directed path to the terminal emitter. These are topology validity checks, not execution dependencies. A terminal emitter may also have outgoing edges, used only when it decides to continue. The entry receives the Run goal as its first message.

On the canvas, dragging from a node's output handle to another node's input handle creates an edge. A transient arrow follows the pointer; releasing on empty space or pressing Escape cancels. Opposite-direction edges render as distinct curved paths with separate arrowheads. Selecting an edge exposes deletion. Keyboard users can focus a source handle and then a target handle to create the same edge. Node movement remains separate from handle dragging. The member selector remains; the relationship checkbox list is removed. Entry and terminal choices are visible controls drawn from selected members. Saving shows a specific reason when a graph is invalid. Removing a member also removes its incident edges and clears its entry/terminal selection if necessary.

## Durable flow runtime

A flow Run freezes the Team graph, entry/terminal choices, selected Agent bindings and Capability guidance, repository, goal, and a step limit. It creates one initial delivery for the entry. Every delivery has an ID, source attempt (except the initial delivery), target Capability, payload artifact references, and sequence number. The queue and its state are persisted, so routing decisions and deliveries can be audited and crash recovery can identify unfinished work. Each delivery that starts causes exactly one target execution, even when another delivery to that target is already queued. Executions against the shared Git worktree are serialized; choosing several recipients enqueues separate deliveries without merging them.

The real Agent returns a role-specific deliverable plus a structured routing decision in the same execution. `next` is a nonempty, duplicate-free subset of that node's outgoing neighbors when continuing. Only the configured terminal emitter may return `complete`; `complete` has no recipients and completes the Run after its deliverable is committed. A blocked or invalid deliverable fails the attempt under the existing contract. An invalid routing decision (unknown/nonadjacent recipient, duplicate recipient, unauthorized `complete`, or empty continuation) fails the Run explicitly; it is never silently corrected. Role-specific deliverable validation remains strict. Routing metadata is validated separately and persisted atomically with the committed attempt before new deliveries become runnable.

Each execution sees its triggering delivery, goal/repository context, and the committed artifact referenced by that delivery. Revision feedback therefore arrives as an explicit incoming message, not by rewinding an upstream node state. Existing Agent role guidance is adapted so a Coder can implement from the goal or feedback, and a Reviewer can return a revision request or completion signal without inventing absent test evidence. For a Reviewer selected as terminal emitter, `approved` must agree with `complete`; `changes_requested` must continue along at least one configured edge. A terminal emitter of another role uses the same explicit routing signal, subject to its role's normal output contract.

After each committed decision, the runtime consumes the next queued delivery. A successful terminal signal discards no previously committed history; outstanding queued deliveries are marked `skipped_due_terminal`, no further delivery starts, and the Run completes. A fixed, frozen default of 24 execution steps bounds cycles; reaching the limit fails the Run as `flow_step_limit_reached` with its history intact. Cancellation, interruption, and process ownership keep their existing safety behavior.

For Coder → Reviewer → Coder, Coder submits a ChangeSet to Reviewer. Reviewer returns comments and routes to Coder. That delivery triggers a new Coder execution with the comments and prior artifact. Coder sends its revision to Reviewer. When Reviewer approves, it emits `complete` rather than routing back; the Run ends.

## Compatibility and migration

Existing persisted Team graphs and frozen Runs retain their legacy consultation/scheduler semantics. A versioned flow-Team representation distinguishes the new mode without reinterpreting old JSON or historical events. The existing default Team remains legacy. The Team page labels legacy Teams and offers an explicit conversion/copy action into a new flow Team, asking for entry and terminal selections; conversion does not mutate the legacy Team or seed an extra Team automatically. New Team creation uses flow mode. New flow Runs use only the delivery queue; they do not also execute legacy review gates or final-Reviewer rewind logic.

## Verification

Focused tests cover canvas drag/cancel/delete and reciprocal rendering; topology validation including cycles, reachability, and terminal paths; persistence and migration; one delivery per trigger; multiple selected recipients; feedback cycles; terminal authorization and Reviewer status coherence; step-limit failure; legacy Team/Run compatibility. Deterministic tests do not stand in for a live Codex acceptance. A live acceptance uses a user-created Coder ↔ Reviewer Team with real Codex, verifies at least one feedback round and a terminal approval, and inspects persisted deliveries and artifacts.
