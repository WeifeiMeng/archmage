# Live Agent Output and Resizable Run Panels Design

## Goal

Improve the Run workbench so a user can inspect a Capability while it is still running and can give either the inspector or the timeline more horizontal space. The inspector becomes the left panel, the timeline becomes the right panel, and their split ratio persists across Runs.

## Product behavior

### Capability-first node identity

The selected node's Capability is the primary identity in the inspector. While a node is active, the inspector uses copy such as `Coder 正在运行`, `Designer 正在运行`, `Tester 正在运行`, or `Reviewer 正在运行`.

The concrete Agent provider remains visible as secondary context, for example `Codex · codex`, `Qoder · qoder`, or `DeepSeek Harness · deepseek-harness`. Provider identity never replaces the Capability name in primary running-state copy.

### Live Agent output

The inspector places an `Agent 实时输出` section directly below node identity and before the final structured result. While the selected Attempt is active, the section appends bounded, user-visible progress records in order. Supported records are:

- visible Agent messages;
- plan summaries and plan status;
- tool or command name and lifecycle status;
- bounded, sanitized tool result summaries;
- safe usage, mode, configuration, and status metadata;
- user-visible provider notices.

The section does not expose hidden model reasoning, raw stderr, credentials, environment contents, unrestricted tool output, or unknown provider frames. Empty and unsupported states explain that no public progress has been emitted. When an Attempt completes or fails, its published progress remains inspectable as history.

The final contract-validated result and Artifact remain separate from progress. Progress never becomes a substitute for the structured result, a Handoff, an Artifact, or a ChangeSet.

### Resizable panel layout

The desktop Run workbench uses this order:

1. node inspector on the left;
2. draggable separator in the center;
3. runtime timeline on the right.

The initial split is approximately 43 percent inspector and 57 percent timeline. Dragging the separator updates both panels continuously. Each panel has a 320-pixel minimum width when the available workbench width permits it.

The separator is a semantic vertical separator with a visible hover, focus, and drag state. It supports pointer input and keyboard adjustment:

- Left/Right Arrow changes the split by a small step;
- Home selects the minimum inspector width;
- End selects the maximum inspector width.

The last valid inspector ratio is stored in desktop-local browser storage and reused across Runs and application launches. Invalid, missing, or out-of-range values fall back to the default. Window resizing clamps the effective ratio without destroying the stored preference.

At the existing narrow-screen breakpoint, the workbench falls back to a single column with the inspector first and the timeline second. The separator is hidden in this mode.

## Runtime event model

Introduce a provider-neutral progress record rather than letting the desktop parse raw Codex JSONL or ACP frames. The record contains:

- Attempt identity;
- progress kind;
- lifecycle status where applicable;
- a short display label;
- optional bounded public text;
- optional safe metadata;
- provider-local ordering information where needed.

Runtime maps accepted provider updates into persisted `agent.progress` events associated with the owning Run, Node, and Attempt. The existing ordered Runtime event store supplies durability, ordering, crash-safe history, and the current desktop polling path. The desktop derives the selected node's live feed from these persisted events.

Event payloads are bounded before persistence. Unknown fields are dropped rather than passed through. Repeated streaming chunks may be coalesced at the provider boundary so the Runtime does not create an event per token or unbounded database growth.

## Provider behavior

### Codex

Codex already decodes stdout JSONL incrementally while writing the private diagnostic log. Extend its observer boundary to emit normalized public progress as decoded records arrive. Supported Codex items are explicitly allowlisted by item type and field; diagnostic JSONL remains private.

### Qoder

Map allowlisted ACP `session/update` notifications to the same normalized records. Agent-message chunks may be coalesced. Plan, tool lifecycle, safe tool summaries, usage, and user-visible notices are allowed. Agent thought content remains excluded.

### DeepSeek Harness

The existing bridge returns one final result frame and has no progress-frame contract. This feature adds the shared model and UI without inventing DeepSeek Harness progress. Its inspector shows the empty/unsupported live state until a later bridge protocol extension emits explicit public progress frames.

## Desktop composition

Keep event normalization separate from rendering:

- a small model helper identifies and validates `agent.progress` events for a node;
- a focused live-output component renders supported progress kinds and empty states;
- a focused split-pane component owns pointer, keyboard, clamping, and persistence behavior;
- `RunWorkbench` composes the selected node, live feed, inspector, separator, and timeline.

This avoids adding more interaction logic to the already dense `RunWorkbench` JSX and makes both behaviors independently testable.

The timeline continues to show all persisted events, including concise `agent.progress` entries. The inspector presents the same selected node's progress in a richer, Capability-centered form. This is intentional: the timeline answers when something happened in the Run, while the inspector answers what the selected Capability is doing.

## Failure and recovery behavior

- Failure to normalize an optional provider update does not expose the raw update. Protocol-invalid frames retain the provider's existing failure behavior.
- Event persistence failure follows existing Runtime failure semantics; the UI does not invent progress that was not committed.
- A malformed stored ratio is ignored and replaced by the default.
- Pointer cancellation ends resizing cleanly and preserves the last valid ratio.
- Historical Runs without `agent.progress` events remain compatible and show the existing empty state plus their final structured result.

## Testing

### Core and provider tests

- Codex JSONL records map only allowlisted public item types.
- hidden reasoning, raw stderr, unknown frames, and oversized text are excluded or bounded;
- normalized records persist with the correct Run, Node, Attempt, and order;
- Qoder allowlisted `session/update` records map to the same provider-neutral model;
- existing final-result validation and Artifact publication remain unchanged.

### Desktop tests

- Capability name is the primary running label and provider identity is secondary;
- selected-node progress is ordered and rendered by kind;
- empty, running, completed, and historical states remain understandable;
- pointer dragging updates the split within bounds;
- keyboard controls adjust the split and expose separator semantics;
- the stored ratio is restored, malformed values fall back, and effective width is clamped;
- narrow layouts stack inspector before timeline and hide the separator.

### Verification

Run focused Rust provider/runtime tests, desktop Vitest tests, formatting and lint checks, the complete relevant Rust test suite, and a desktop production build. Visually verify drag behavior, focus state, Capability-first copy, live append behavior, persisted width, and the narrow single-column fallback.

## Out of scope

- displaying hidden chain-of-thought or raw provider streams;
- turning progress into a trusted Artifact or Handoff;
- adding search, filtering, pause, export, or manual log clearing;
- extending the DeepSeek Harness bridge protocol in this change;
- synchronizing panel width between machines or user accounts.
