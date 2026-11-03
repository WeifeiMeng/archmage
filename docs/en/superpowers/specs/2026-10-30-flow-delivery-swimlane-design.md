# Run delivery message swim lane design

[简体中文](../../../zh-CN/superpowers/specs/2026-10-30-flow-delivery-swimlane-design.md) | English

## background

The Run details page currently only renders `flowDeliveries` as a set of routing labels, such as `coder 已交付 → reviewer`. The runtime has actually persisted the capacity output of each round to `output_json` corresponding to Attempt, but RunSnapshot does not project this content into the delivery record, so the front end cannot display "what was delivered."

Also, the Delivery Messages panel suffers from the universal `.panel` minimum height, taking up large chunks of white space even with just a few labels.

## target

- The source, target, sequence, status and content summary of each round of delivery between different capabilities are directly presented on the Run details page.
- Use dynamic capability swim lanes to display the delivery path within the team, and support any combination of Coder, Reviewer, Designer, and Tester.
- After clicking the delivery card, expand the complete structured content in the swim lane grid where the sending capability is located.
- Reuse the persisted historical Attempt output so that existing Runs can display delivery content without re-executing it.
- Let the panel height follow the actual content and no longer leave the entire screen blank.

## data design

### FlowDelivery output projection

Add optional `output` field to RunSnapshot representation of `FlowDelivery`:

```text
output: JSON | null
```

When querying for `flow_deliveries`, `attempts.output_json` is associated with its own `attempt_id` based on that delivery. Here, the currently delivered execution Attempt is used instead of the "latest result" of the target capability, thereby ensuring that each round of content in the historical feedback loop accurately corresponds to its own delivery record.

This adjustment does not add new database columns, nor does it modify Artifact content:

- Delivery Completed: `output` is a structured deliverable for this round of capabilities.
- Delivery pending or in progress: `output` for `null`.
- Failed, interrupted, or canceled delivery: Preserves state and returns content only if the output has been persisted.
- Historical Run: Read directly from existing `attempts.output_json`, no migration or backfill required.

Sensitive diagnostics, provider raw output, and credentials do not enter this projection.

### Delivery direction

One completed `FlowDelivery` indicates the execution ability to hand over the output of this round to the next target:

- Sending capability: `delivery.target`.
- Receiving capability: `delivery.routing.next`.
- The process is completed: `delivery.routing.decision == complete`, and the receiving end displays "Complete".
- Multiple receivers: The same card displays multiple targets and expresses the receiving status in the corresponding target swim lane.
- Initial pending execution record: when there is no routing, the current execution target and queue/running status are displayed, without fictitious sender or summary.

## Interface structure

### Dynamic lane

The swim lane columns are from `team.graph.nodes` for the Run frozen team, in the same order as the frozen team configuration. The narrow serial number column is reserved on the far left, and each `FlowDelivery` occupies one line.

The delivery card is placed in the swim lane grid corresponding to the sending capability. The card is displayed by default:

- Delivery serial number.
- Sending and receiving capabilities.
- Current status.
- A snippet extracted from the structured output.
- Expand/collapse state.

Other capability cells do not copy the deliverable content and only display lightweight status related to the round, such as "Waiting to receive", "Feedback received" or "Process completed".

### Expand within the grid

After clicking to deliver the card, the details will be expanded directly in the swim lane grid where the card is located:

- Expanding only increases the height of the current delivery row.
- Details are not moved across the full width area or to the right of the inspector.
- By default, only one delivery is expanded at the same time to reduce long page noise.
- Card buttons use `aria-expanded` and an explicit accessible name, and support keyboard operation.

Friendly fields are displayed according to the following priority based on output content:

1. `status`, `summary_markdown` and other overall conclusions.
2. `issues`, `suggestions` and other review or feedback items.
3. `changed_paths`, `notes` and other implementation and verification information.
4. Other structured fields are presented as stable key-values.

The original JSON is placed in the secondary `<details>` at the bottom of the details, which is collapsed by default.

### Summary generation

The abstract uses only the currently delivered structured `output`, selected in the following order:

1. The first segment of non-empty `summary_markdown`.
2. Quantity and first issue summary for `issues`.
3. First item of `notes`.
4. `changed_paths` quantity.
5. When there are no readable fields, "Structured delivery has been generated, expand to view details" is displayed.

The abstract performs pure display conversion at the UI layer, without writing back to the database or calling additional models.

## Status and exception display

- `queued`: Displays "Waiting for execution" and does not display the content summary.
- `running`: Displays "Executing", using existing low-disruption loading instructions.
- `completed`: Show summary, routing, and expandable details.
- `failed`, `cancelled`, `interrupted`: Use semantic status colors and text descriptions; if there is no output, the empty details entry will not be displayed.
- `skipped_*`: Show skip reason, do not generate false send arrow.
- Output JSON cannot be parsed: backend maintains existing data error bounds; frontend does not accept partial parsing results.

Color is not the only status signal, all statuses also provide text or icons.

## Responsive design

- Widescreen: Use full dynamic swimlane table.
- Swimlane columns are kept to a minimum readable width, and field text wraps naturally.
- When the window is small or there are many capabilities: it degrades to delivering cards vertically, each card continues to display `发送能力 → 接收能力`, and the expanded content remains within the current card.
- Does not rely on full-page horizontal scrolling; partial content is reflowed first rather than compressed to unreadable width.
- The existing `prefers-reduced-motion` rule continues to be in effect.

## White space correction

Use a more explicit component style than generic `.panel` for the delivery panel, making it `min-height` content-driven. This fix only affects Run delivery messages and does not change the timeline and Node Inspector panel height.

Empty delivery states use compact description text and do not render large empty containers.

## component boundaries

Added a new independent delivery swim lane component, responsible for:

- Dynamic swimlane layout.
- Delivery status, directions and summary display.
- Expand status management.
- Friendly fields and raw JSON presentation.
- Responsive degradation.

`RunWorkbench` is only responsible for incoming frozen team members and `flowDeliveries` does not continue the rendering details of the inline delivery message.

The backend Store is responsible for projecting Attempt output to `FlowDelivery`; runtime scheduling, routing decisions, and persistent transactions remain unchanged.

## Verify

### backend

- Completed delivery returns `output` corresponding to its own Attempt.
- Multiple rounds of Coder/Reviewer loops will not incorrectly reuse the latest results.
- Pending and running deliveries return `output: null`.
- Existing databases can read historical delivery content without migrating.
- Existing tests for Flow and Runtime continue to pass.

### front end

- Lane order comes from frozen team members.
- Coder → Reviewer, Reviewer → Coder and Reviewer → Done all show the correct direction.
- Multi-recipient delivery shows all targets.
- Summary selection follows fixed priority.
- Clicking on the card will only expand it within the current lane grid, and set `aria-expanded` correctly.
- Friendly fields and raw JSON collapse items display correctly.
- Queued, running, failed, skipped and no content status all have clear copy.
- Use vertical card layout for multiple capabilities and narrow windows.
- The height of the panel follows the content and no longer inherits the minimum height of the entire page.

### Closed loop acceptance

Verify at least four rounds of historical deliveries using existing Coder/Reviewer Run:

1. Coder → Reviewer: Implement summary and verification records.
2. Reviewer → Coder: Issue lists and revision requests.
3. Coder → Reviewer: Feedback fix summary.
4. Reviewer → Done: Final approval of the conclusion.

## non-target

- No changes to Flow routing rules, 24-step limits, or capability execution order.
- Does not change the Agent output schema or regenerate the historical summary.
- Deliveries are not written to the new database copy.
- Do not upgrade swim lanes to editable team topology.
- No changes to the node inspector, runtime timeline, or how artifacts are stored.
