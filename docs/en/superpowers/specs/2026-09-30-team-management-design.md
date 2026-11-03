# Team Management and Review-Gated Topology Design

**Status:** Approved direction, implementation scope for the first runnable Team release

## Product boundary

Agent management owns local provider instances and connectivity. Capability management owns individual logical workers: Designer, Coder, Tester, Reviewer, each with a built-in persona/prompt and one Agent binding. It is a list/detail page, not a topology view. A later release may add a per-Capability skill scope; this release does not expose or enforce skill scopes.

Team management owns named, persisted arrangements of those Capabilities. A Team is selected when starting a Run and frozen into that Run. The Team canvas is not merely illustrative: every saved edge causes a real downstream consultation, and approvals gate the next execution step.

## First-release topology constraints

- Exactly one of each built-in Capability is present. Designer is the sole entry; Coder, Tester, and Reviewer keep their current execution contracts.
- The required execution/consultation spine is `Designer → Coder → Tester → Reviewer`. These three edges cannot be removed. This is necessary because the current Coder contract requires the design, Tester requires the implementation and changeset, and Reviewer requires the test result.
- The canvas may add or remove the three forward consultation edges `Designer → Tester`, `Designer → Reviewer`, and `Coder → Reviewer`. No reverse edge, duplicate edge, self-edge, cycle, arbitrary node type, or second copy of a Capability is allowed. Nodes may be repositioned for readability; positions have no scheduling semantics.
- Every edge means the downstream Capability reviews the current upstream deliverable before that downstream's own execution. The required spine also provides the execution dependency. Optional edges add consultations but do not remove the required execution order.
- Seed one built-in default Team with all six edges, matching the topology that was previously shown on the Capability page. Users may create and rename Teams, edit their optional edges and node positions, and select a Team for a Run. Deletion is out of scope in this release.

## Review-gated execution

1. Designer receives the goal and produces a design using its existing persona and output contract.
2. For each completed upstream node, the Runtime starts consultations with **all of that node's outgoing neighbors concurrently**. A consultation uses the downstream Capability's frozen Agent binding and persona, but a separate strict review contract (`approved` or `changes_requested`, summary, nonblank suggestions). It is read-only and never executes downstream's main task or writes to the worktree.
3. All outgoing consultations for one producer attempt must approve before that producer's handoff is accepted. An optional cross-edge is not skipped just because another path reaches the same downstream node.
4. If one or more reviewers request changes, aggregate their suggestions, persist the review records and events, and return the suggestions to the upstream Capability. The upstream reruns in a new Attempt with its prior deliverable and the feedback. Its previous successful Attempt and artifacts remain auditable, but the new successful Attempt becomes the current handoff source. Consultations then rerun for every outgoing edge against that new Attempt.
5. A producer may make at most three revision attempts after its initial output for a handoff. If any outgoing consultation still rejects the fourth output, the Run fails with a specific `review_round_limit` reason and retains every suggestion. Provider/contract errors remain ordinary execution failures, not a synthetic rejection.
6. Once approvals are durable, the next Capability on the required spine may execute. The main node executions remain serialized in this release because Coder and Tester use one mutable Git worktree; only read-only consultations fan out in parallel.
7. The final Reviewer main result retains its existing approved/changes-requested outcome. A `changes_requested` result is treated as feedback to Coder: Coder and Tester rerun, then Reviewer reruns, subject to the same three-revision ceiling. The feedback is persisted. Reviewer approval completes the Run.

The UI distinguishes a downstream **consultation** from that Capability's main execution. A consultation does not count as the downstream node having run. No silent auto-approval, Fake Agent, or prewritten tester acceptance is permitted.

## Persistence and compatibility

SQLite stores Teams with stable IDs, names, revision numbers, ordered node positions, and edge sets. Saving validates the full graph transactionally and compares revisions; a stale editor receives a conflict rather than overwriting another save. The default Team is seeded idempotently. A Run request carries a Team ID; if omitted by an existing CLI caller, the default Team is used. The frozen Run definition stores the Team ID/name/revision and full validated topology alongside the existing frozen Agent bindings and Capability configurations. Editing a Team or binding never mutates an active or historical Run.

Consultations are separate persisted records keyed by Run, source node, target node, source Attempt, and review round, with decision, suggestions, result artifact/identity, and timestamps. They produce ordered events for timeline inspection. The scheduler derives progress from the frozen topology plus durable node/consultation states; a process restart does not invent approvals or discard feedback. A forced process exit follows the existing interrupted-Run policy. Existing schema-9 databases migrate without changing old Runs. Historical Run definitions without a Team continue using their frozen legacy four-node behavior and snapshot format.

## Desktop experience

- Sidebar has separate Agent, Capability, and Team destinations. Capability management becomes four individual cards/list entries with bound Agent status and one binding editor; no graph or edge legend appears there.
- Team management has a Team selector/create control and a canvas with the four Capability nodes. Dragging changes only positions. Users can toggle the three optional forward edges through visible controls; the three required edges are visually distinct and locked. A nonvisual edge list offers the same editing actions by keyboard. Save/dirty/conflict states are explicit.
- New Run requires a Team selection (default preselected). Run detail shows the frozen Team and each consultation's status, reviewer, suggestions, and revision round. The topology view in Run detail is read-only and sourced from the frozen Run definition, not the current Team.
- A browser-only Vite preview must not invent persisted Teams, Agent health, or Run progress. Real management and execution require the Tauri Runtime.

## Failure and validation

- Reject invalid graphs, blank/duplicate Team names, stale revisions, disabled/unsupported Agent bindings, or missing Capability configuration before creating a Run. A failed save leaves the old Team intact.
- Consultation JSON must satisfy its strict schema. `changes_requested` requires at least one actionable, nonblank suggestion; `approved` cannot carry blocking suggestions. Model output and feedback remain provenance-linked to the producing Attempt.
- Every revision or final-review rejection is bounded. After a rejection, no downstream main task starts with the rejected producer Attempt. If a prior downstream task must be rerun after final Reviewer feedback, its old result is retained only as history and cannot satisfy the new round's dependencies.
- Cancellation terminates in-flight consultation processes under the existing Run owner generation and records cancellation; concurrent consultations must not publish results after owner loss.

## Verification

- Migration and Team service tests cover seeding, CRUD update, graph validation, revision conflict, and preserving schema-9 data.
- Scheduler tests cover mandatory spine, optional fan-out, all-approved gate, mixed rejection and feedback aggregation, re-review of a revised Attempt, three-revision limit, stale review exclusion, and legacy Runs.
- Runtime tests use the real Codex adapter boundary with a test-controlled invocation layer for deterministic results; live authenticated Codex acceptance remains an explicitly separate manual check. No Fake Agent product adapter is introduced.
- Frontend tests cover edge toggles, dirty/save/conflict behavior, Team selection serialization, and consultation timeline projection. Run Rust workspace tests, frontend tests/build, and `git diff --check` before delivery.
