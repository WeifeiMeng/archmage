# Desktop Form Pages Layout Polish Design

[简体中文](../../../zh-CN/superpowers/specs/2026-10-28-desktop-form-pages-layout-polish-design.md) | English

**Status:** Approved visual direction; ready for implementation planning.

## Scope

Polish the New Run, Agent Management, and Capability Management pages using the same spatial and visual principles approved for Team Management. The pages should feel like one product without forcing different tasks into identical layouts.

Run Detail is explicitly out of scope. This work does not change Runtime behavior, APIs, persistence, validation, navigation, or business semantics.

The companion Team Management design is documented in `2026-10-28-team-management-layout-polish-design.md`.

## Shared layout language

All four management/form pages use an adaptive command-Hero system:

- The Hero combines page identity with useful page-level controls or state. It is not a presentation-only title block.
- A restrained green-tinted surface, compact title scale, subtle decoration, consistent border radius, and shared spacing establish visual continuity.
- Page-level actions live in the Hero. Object-level actions remain near the object they affect.
- Status is expressed as compact metrics, pills, or summaries rather than long prose.
- The main workspace uses available horizontal space on wide windows and collapses to a clear single-column reading order on narrower windows.
- Existing dark surfaces, green accent, typography, focus treatment, and semantic controls remain the visual foundation.

Shared CSS patterns may be introduced for the command Hero, summary metrics, page panels, section headings, and responsive breakpoints. Page-specific classes remain appropriate when a component has unique behavior. This is a focused consolidation, not a new general-purpose design system.

## New Run

### Hero

The Hero contains `TEAM RUN`, the page title, concise guidance, and a Runtime readiness summary. The summary reflects the existing environment state:

- checking: environment check in progress;
- ready: Runtime and supporting data are available;
- failed: environment check failed;
- preview: browser-only interface preview.

The Hero does not invent new health checks. It projects state already held by the page.

### Task and team workspace

The form becomes a two-column workspace on wide screens:

- The left task panel contains the repository picker and goal field.
- The right team panel contains the team selector, existing `RunTeamPreview`, readiness or binding issues, and the Start Run action.

Repository/environment errors stay near the task or Hero context. Missing teams, failed team loading, and binding issues stay in the team panel. The existing recovery actions remain available.

The Start Run button remains the single primary action and keeps the existing `canStart` logic. The right panel states that a Run freezes the selected Team and Capability configuration.

On narrower screens, task definition precedes team selection and start controls in one column.

## Agent Management

### Hero and health summary

The Hero contains the page identity, `添加 Codex 实例`, and four compact metrics:

- verified and ready;
- enabled but unverified;
- enabled with failed connection;
- total supported instances.

The metrics derive from the existing management snapshot and do not create new Agent states. `添加 Codex 实例` remains the page-level action because it creates a new object.

### Instance workspace

The page moves directly from the Hero into a concise `本地连接` section and the existing responsive instance-card grid. The redundant large section gap is removed.

Each Agent card keeps object-level actions locally: save configuration, inspect CLI, verify connection, and delete. Unsupported adapters retain their distinct read-only cards. The new-instance editor continues to appear in the grid.

The Capability routing handoff becomes a compact next-step panel at the bottom rather than a second large section. It retains the existing `打开 Capability 管理` navigation.

Loading, load error, connection error, dirty state, disabled state, bound-instance restrictions, and delete confirmation behavior remain unchanged.

## Capability Management

### Hero and summary

The Hero contains the page identity, Reload action, and compact totals for:

- fixed Capabilities;
- valid Agent bindings;
- configured Skills;
- configured Tools.

The totals derive from the existing snapshot. The page continues to distinguish browser preview, loading, and load-error states without inventing data.

### Master-detail workspace

On wide screens, the page becomes a stable master-detail layout:

- The left rail lists Designer, Coder, Tester, and Reviewer vertically. Each item shows its Agent connection summary and Skill/Tool counts.
- The right editor shows the selected Capability's identity, description, Agent binding, extra instructions, Skills, Tools, advisory text, feedback, and Save action.

This replaces the current horizontal four-card selector followed by a full-width editor. The left rail remains visible while editing so switching context does not require scrolling back upward.

Skills and Tools use two parallel sections on wide screens and stack on narrower screens. Their existing row editing, limits, labels, removal controls, and guidance text remain intact.

Reload remains a page-level action in the Hero. Save remains an object-level action at the bottom of the editor. Dirty-switch and dirty-refresh confirmations, revision conflict feedback, Agent availability validation, and the statement that changes affect only new Runs remain unchanged.

At medium widths the master-detail workspace becomes one column, with the Capability selector rendered as a compact two-column grid above the editor. At narrow widths it may fall back to one column when needed for readable labels.

## State and data flow

The redesign only changes markup grouping and presentation:

- React state, API calls, effects, confirmation dialogs, and existing derived values remain the sources of truth.
- Shared presentation components may receive already-derived labels and counts; they do not fetch data or own business state.
- Existing child components, including `RunTeamPreview`, Agent cards, and Capability guidance rows, retain their behavior unless a small markup adjustment is required for layout.
- Navigation between Agent and Capability management remains owned by `App`.

## Accessibility

- Preserve semantic headings, labels, inputs, selects, textareas, buttons, checkboxes, and status/alert roles.
- Page-level status summaries must include text, not color alone.
- Maintain visible focus indicators and logical keyboard order after visual reflow.
- Responsive visual order must match DOM and keyboard order.
- Icon-only controls require accessible names and visible tooltips.
- Disabled, failed, and read-only states must retain sufficient contrast.

## Error and transient states

Loading, preview, empty, dirty, busy, failed, success, and validation states must fit the new panels without layout shifts that hide controls. Existing messages and recovery actions remain semantically unchanged. Global errors continue to use the application error banner.

No optimistic state, new toast system, or new error transformation is introduced.

## Implementation boundary

Expected files include:

- `App.tsx` and `styles.css` for New Run;
- `AgentManagement.tsx` and `agents.css` for Agent Management;
- `CapabilityManagement.tsx` and `capabilities.css` for Capability Management;
- `TeamManagement.tsx` and `teams.css` for the previously approved Team Management design;
- focused frontend tests for affected structure and behavior.

Run Workbench and Run Detail markup/styles are not redesigned. Changes to shared selectors must be scoped so Run Detail retains its current layout.

## Verification

- Update or add focused tests for all controls that move: repository selection, team selection, recovery links, Start Run, adding and editing Agents, Agent verification/deletion, Capability switching, Reload, Skill/Tool editing, and Save actions.
- Run the desktop frontend test suite and production build.
- Inspect all four redesigned pages at wide, medium, and narrow widths.
- Check New Run in ready, failed, preview, no-team, load-failure, and binding-issue states.
- Check Agent Management with ready, unverified, failed, unsupported, adding, busy, and load-error states.
- Check Capability Management in preview, loading, error, dirty, unavailable-binding, conflict, success, and normal states.
- Verify keyboard focus order, accessible names, visible focus, and that no control depends only on hover.
- Confirm Run Detail is visually unchanged.
- Run `git diff --check` before delivery.
