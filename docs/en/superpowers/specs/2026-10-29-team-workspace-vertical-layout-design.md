[简体中文](../../../zh-CN/superpowers/specs/2026-10-29-team-workspace-vertical-layout-design.md) | English

# Vertical layout design of team workspace


## target

Change the arrangement of "Team Settings" and "Message Flow Canvas" in the team management page from two columns on the left and right to a single column on the top and bottom, so that both pieces of content use the full page width.

## Layout

- Team Settings sits above, keeping the content at its natural height.
- The Message Flow Canvas is below, taking up the available width.
- The two panels inherit the existing spacing, borders, background, and rounded corners, leaving the rest of the page unchanged.
- The existing horizontal editing area and overflow scrolling capabilities will continue to be retained inside the canvas, and nodes, connections, and keyboard interactions will not be changed.

## range

- Only modify the team management page workspace layout style.
- Does not modify team data, save logic, component order, or message flow behavior.
- The Run details page is not modified.
- The mobile version continues to maintain a single column layout.

## Verify

- Added style regression check to ensure the workspace uses a single column grid.
- Run full test and production builds of the desktop frontend.
- Check the code differences to confirm that they do not exceed the scope of the team management page layout.
