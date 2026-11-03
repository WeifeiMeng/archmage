[简体中文](../../../zh-CN/superpowers/specs/2026-10-28-team-management-layout-polish-design.md) | English

# Team Management Layout Polish Design


**Status:** Approved visual direction; ready for implementation planning.

## Goal

Make Team Management denser, clearer, and more visually cohesive without changing its persistence, validation, canvas, or team-editing behavior. The existing page spends too much vertical space on a presentation-only header and then lists related controls in separate full-width rows. The revised page turns the header into a functional command area and uses the available horizontal space to show team configuration beside the message-flow canvas.

The chosen direction is the refined **Command Hero** layout. It preserves the current dark visual language and green accent while bringing the canvas into the initial viewport on typical desktop windows.

## Information architecture

The page has three levels:

1. A compact command Hero contains the page identity, short guidance, team selection, team status, and global actions.
2. A two-column workspace contains team configuration on the left and the message-flow canvas on the right.
3. A slim footer contains contextual feedback and the primary Save action.

This arrangement groups controls by purpose instead of rendering each group as a separate full-width band.

## Command Hero

The Hero remains the visual anchor, but its content becomes operational:

- The left side shows `TEAM MANAGEMENT`, the `团队管理` title, and one short explanatory sentence.
- The upper-right side contains `创建团队` and icon-only `刷新` actions. The refresh button keeps an accessible label and tooltip.
- The lower row contains the current-team selector and a read-only summary of the selected team, such as capability count and graph mode.
- The selector continues to distinguish `消息流` and `历史咨询` teams.
- The Hero uses a restrained green-tinted gradient and subtle decorative rings. Decoration must not reduce contrast or capture pointer events.

The header no longer uses a full-width divider or a large empty area below the description.

## Workspace

On wide screens, the workspace is a two-column grid of approximately 42% configuration and 58% canvas. Both columns begin at the same vertical position.

### Configuration panel

The left panel contains:

- The team-name field and, for a historical team, the `复制为消息流` action on the same row.
- A `团队能力` heading, concise help text, and a selected-count badge.
- Capability choices in a two-column grid. Each choice is a full clickable card with a clear selected state, rather than a small checkbox floating inside a large empty row.
- Entry and terminal selectors in a compact two-column row for editable flow teams.

For historical teams, disabled fields remain visually legible and the compatibility explanation is integrated into the panel instead of occupying a separate full-width notice. The copy action remains prominent enough to be discovered.

### Canvas panel

The right panel contains:

- A `消息流画布` heading and a compact node/route summary.
- The existing interactive canvas, node dragging, connection creation, edge selection, deletion, and keyboard behavior without semantic changes.
- Connection guidance directly below the canvas, with transient linking or selected-edge controls appearing in the same panel.

The canvas should fill its column while retaining the existing 1000-by-360 internal coordinate system and horizontal overflow behavior when the available width is too small.

## Feedback and save behavior

The existing validation issue or status message appears in a slim footer beneath the workspace. `保存团队` remains the only primary action and stays aligned to the lower right on wide screens.

Busy, disabled, dirty, validation-error, success, and API-error behavior remain unchanged. The redesign must not hide the reason Save is unavailable. Existing confirmation dialogs for switching, creating, refreshing, copying, or leaving with unsaved changes remain unchanged.

## Responsive behavior

- At wide desktop widths, configuration and canvas use the two-column workspace.
- At medium widths, the workspace becomes a single column with configuration above the canvas. The Hero selector row may also wrap, but actions remain grouped.
- On narrow widths, capability cards remain at least two per row when readable and fall back to one per row when needed. Endpoint selectors become one column.
- No control may overflow the page or depend on hover for discovery.

## Accessibility

- Preserve visible focus styles for all inputs, selects, buttons, capability cards, canvas nodes, and connection handles.
- Icon-only Refresh has an accessible name and visible tooltip.
- Capability cards use the existing checkbox input as their semantic control; styling the label must not remove keyboard or screen-reader behavior.
- Disabled historical-team controls maintain sufficient text and border contrast.
- Decorative Hero elements are hidden from assistive technology.

## Implementation boundary

The change is limited to `TeamManagement.tsx`, `teams.css`, and focused frontend tests if the structural markup requires them. It does not alter APIs, models, graph validation, persistence, Runtime behavior, team semantics, or canvas geometry.

Markup may be split into small presentational sections only if that makes the layout easier to understand; no reusable design-system abstraction is required for this page-only change.

## Verification

- Update or add frontend tests for the current-team selector, create/refresh actions, historical-team copy action, capability toggles, endpoint selectors, and Save availability after the markup changes.
- Run the desktop frontend test suite and production build.
- Inspect the page in an editable flow team and a read-only historical team at wide, medium, and narrow viewport widths.
- Verify keyboard focus order, icon-button labeling, canvas drag/link behavior, and horizontal canvas overflow.
- Run `git diff --check` before delivery.
