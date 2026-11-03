# Run Team Selection Design

## Goal

Creating a Run should require a Git repository, a goal, and one Team selection. The selected Team is the only collaboration configuration shown on the form.

## Form

Keep the Team selector immediately above one compact, read-only preview. Show its member Capabilities and directed edges. For a message-flow Team, also identify its entry and terminal Capabilities. For a legacy Team, label its historical consultation semantics. Changing the selection updates the preview without changing the Team.

Remove the persistent Tester, all-Capability binding, and ready-Runtime cards. Do not promise that every Run includes all four built-in Capabilities. Show a small actionable message only when no Team exists, a selected Team member has an unusable binding, or the local Runtime cannot be used. Link Team problems to Team Management and binding problems to Capability Management. Keep backend preflight as the final authority; do not add a new execution option.

## Start behavior

Only selected Team members participate in the form's binding check. A valid selected Team with a valid repository and goal can start even when an unused Capability is not configured. Do not treat connection state alone as a launch blocker; the existing backend preflight remains responsible for live validation. A new Run continues to freeze its Team graph and Capability settings at creation.

When returning from Team Management, reload the Team list and Agent bindings before enabling Start. Hide cached Team details while reloading; a failed Team-list request must disable Start and offer retry rather than look like an empty Team list. Browser-only preview must not claim that Teams are still loading.

## Verification

Test Team preview details and selected-member binding filtering with pure TypeScript tests. Run the desktop test suite and production build. Inspect the final form for selection, empty Team, and blocked states.
