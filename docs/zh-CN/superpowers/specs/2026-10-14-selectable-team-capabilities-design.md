# Selectable Team Capabilities Design

**Status:** Approved direction for implementation on `main`.

## Scope

Team creation lets a user choose at least two of the four existing Capabilities, then choose directed relationships between selected members. The user creates their own teams; no additional team is seeded. Existing four-member teams and historical Runs remain valid.

The first release preserves the fixed semantic order Designer → Coder → Tester → Reviewer. Any forward edge between selected members is allowed. A saved graph must be a single connected directed acyclic graph: every selected member participates in an edge, and all members are reachable from one entry. No duplicate/self/backward edge is allowed. Relationships retain the current meaning: downstream reviews the upstream deliverable before its own main task; siblings may consult concurrently. Main tasks remain serialized in semantic order because they share one worktree.

## Runtime

A new Run freezes only the selected Capabilities, their bindings, configurations, and graph. The scheduler derives readiness from incoming graph edges, not from the old four-node spine. A source with outgoing edges must receive approvals before a downstream main task starts. The last selected node determines completion. If Reviewer is present, its status is the Run outcome. Otherwise successful completion is `approved`; any blocked/failed node fails the Run.

Prompt inputs depend on which upstream roles are present. Designer uses goal and repository view. Coder uses design if present, otherwise goal and repository view. Tester uses implementation and ChangeSet if Coder is present, otherwise goal and repository view; design is included when present. Reviewer receives available design, implementation/ChangeSet, and test result. Without Tester, Reviewer may approve based on available evidence; it must not claim tests occurred. Built-in role prompts explain optional inputs while output JSON schemas stay unchanged.

Review feedback from outgoing consultations reruns the producing Capability, bounded to three revisions. A final Reviewer `changes_requested` reruns the nearest upstream non-Reviewer Capability and every later member in semantic order, again bounded to three revisions. For `Coder → Reviewer`, this means Coder then Reviewer; for the original four-member team it remains Coder, Tester, Reviewer. Existing Run definitions without a Team keep legacy behavior.

## Desktop and validation

Team creation starts from an empty graph. The user selects Capabilities, then toggles forward relationships between selected members and arranges nodes on the canvas. Removing a member also removes its edges. The page explains the valid graph rules, disabled controls, save conflicts, and the difference between consultations and main tasks. Existing teams can be edited under the same rules. A browser preview never invents persisted teams.

Backend validation is authoritative. Invalid graphs are rejected before persistence and Run creation; unsupported or disabled Agent bindings for unselected Capabilities do not block a Run. Tests cover graph selection and rejection, persisted round trips, dynamic Run nodes, scheduler, Coder→Reviewer context and outcome, and four-node compatibility. Live Codex acceptance is separate from deterministic tests.
