# Organize and design the operation area of Run details page

[简体中文](../../../zh-CN/superpowers/specs/2026-10-30-run-detail-toolbar-design.md) | English

## target

Reduce the visual noise in the operation area in the upper right corner of the Run details page, and remove the team topology display that overlaps with the ability progress, allowing the page to prioritize task titles, node status, and actual running information.

## Header operating area

- Use grouped toolbars to put "Refresh", "Workspace" and "Cancel" into the same compact container.
- The three buttons maintain the same height, icon size, font size and single line of text to avoid Chinese line breaks.
- "Refresh" and "Workspace" use neutral styles; "Cancel" expresses destructiveness through dividers, dangerous text, and a light dangerous background, but does not use a highly saturated solid button.
- Continue to use the Lucide icon and retain visible text, keyboard focus, and disabled state.
- After the run is completed, the current behavior is retained and "Cancel" is not displayed; the positions of the other buttons remain stable.

## Streamlined page content

- Remove the "Frozen Team Topology" panel and its edge labels from the Run details page.
- Frozen team data in Run snapshots is not modified or deleted.
- The team name and version continue to be displayed in the header meta information.
- Capability progress, delivery messages or review progress, run timelines, and node inspectors maintain existing behavior.

## Responsiveness and visual states

- The operation group does not wrap, and the header content area is allowed to shrink to prevent a long title from squeezing a single button text into two lines.
- Maintain a minimum visual height of 38 pixels and sufficient click area; use border separation between buttons rather than extra margins.
- Hover, active, focus-visible, disabled and dangerous states all need to be distinguishable; do not rely on animation to express the state when reducing dynamic effects.

## Verify

- Component testing confirms that active Run shows three actions, inactive Run hides "Cancel".
- Component testing confirms that Run details no longer render "frozen team topology".
- Desktop testing and production build passed.
- Manually check the long title scene under the light theme to confirm that the button text does not wrap, the level of dangerous operations is moderate, and the content sequence is continuous after the topology is removed.

## non-target

- Does not change the business behavior of refreshing, opening a workspace, or canceling a Run.
- Does not modify team configurations, message flows, freeze snapshots, or run scheduling logic.
- No redesign of capability nodes, delivery messages, timelines, or node inspectors.
