# Agent management is designed by Provider grouping

[简体中文](../../../zh-CN/superpowers/specs/2026-10-30-agent-provider-grouping-design.md) | English

## target

Let the adding entry of the Agent management page establish a direct ownership relationship with the Provider, reduce the visual weight of the top operations, and maintain parity between the instance management capabilities of Codex and DeepSeek Harness.

## Page structure

- Top Hero removes the `LOCAL AGENT CONNECTIONS` header and the global "Add Agent Instance" action.
- Hero retains page titles, descriptions, and instance status statistics.
- The local connection area permanently displays two supported Provider groups: Codex and DeepSeek Harness; the group does not disappear when the instance is empty.
- Each group contains a name, a short description, a low-emphasis add button, and a list of instances of that Provider.
- Unsupported providers continue to appear after the supported group and remain "Not yet supported".

## Add interaction

- The Codex group provides an "Add Codex" button, and the DeepSeek Harness group provides an "Add DeepSeek Harness" button.
- After clicking the button, the new instance form is displayed in the corresponding group, the Provider type is determined by the entrance, and the Provider drop-down box is no longer displayed.
- At most one new instance form can be expanded at the same time; clicking the add button of another Provider will switch the form, and clicking the current button or cancel will close the form.
- After successful creation, close the form and refresh the instance list; existing behaviors such as saving, verification, and deletion remain unchanged.

## visual hierarchy

- Provider adds buttons using the normal secondary button style and not the Hero accent color style.
- The Provider group title assumes the main classification level of the page, and the instance card maintains the existing editing and status display structure.
- Empty Provider groups display a brief empty state to ensure users can still find the entry point to add.

## Testing and Acceptance

- The page no longer renders the `LOCAL AGENT CONNECTIONS` or Hero global add button.
- Codex and DeepSeek Harness groupings and their respective addition entries are always present.
- Both portals open new instance forms bound to the correct Provider.
- Only one new instance form is displayed at the same time. Successful cancellation and creation will close the form.
- Existing desktop tests and production builds remain passing.

## non-target

- No modifications are made to the Agent data model, Rust services, Provider execution logic, or database structures.
- Do not change the available status of unsupported providers.
- Existing Agent instance edit forms are not redesigned.
