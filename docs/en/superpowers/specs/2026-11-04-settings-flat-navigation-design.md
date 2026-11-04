[简体中文](../../../zh-CN/superpowers/specs/2026-11-04-settings-flat-navigation-design.md) | English

# Flat Settings Navigation Design

## Background and Goal

The current settings shell adds a full-width “Back to main / ARCHMAGE / Settings” row directly below the macOS title bar. These two stacked top regions feel unnecessarily heavy. The large hero banner, card shadows, and generous corner radii add another layer of visual weight.

This change adopts a minimal linear layout: remove the separate top navigation region, move the back action into the settings sidebar, and establish hierarchy with spacing, thin dividers, and state color. Data loading, saving, leave confirmation, and category navigation behavior remain unchanged.

## Page Structure

- Keep the settings shell's category sidebar and content area without restoring the main application sidebar.
- Remove the existing full-width settings header.
- Keep only the small `ARCHMAGE` wordmark at the top of the sidebar; do not repeat a large “Settings” title there.
- Preserve the existing groups:
  - General: Appearance
  - Resource management: Agents, Capabilities, Teams
- Pin “Back to main” to the bottom of the sidebar. It uses the same icon, type, and interaction language as the category navigation and is separated from categories by a top divider.
- Start the content area at the top of the sidebar; do not preserve empty space for the removed header.

## Visual Design

### Sidebar

- Use a flat background and a 1px right divider with no shadow.
- Category items have no default border, card outline, or large radius.
- Indicate the active category with both emphasized text and a thin green bar on the left, so state is not conveyed by color alone.
- Hover and keyboard focus use a subtle background or a clear focus ring without changing layout dimensions.
- “Back to main” keeps a labeled target at least 44px tall.

### Content Area

- Remove the large gradient hero. Render `SETTINGS`, the page title, and its description as a regular page heading.
- Separate the heading and body with spacing and a 1px divider instead of shadow or elevation.
- Remove heavy card shadows and large radii from settings content; organize sections with spacing and fine dividers.
- Appearance theme options retain a clear boundary, preview, label, and selected state, but use smaller or square corners and no shadow.
- Reuse existing semantic color variables in both light and dark themes, with visible contrast and focus states.

## Responsive Behavior

- Keep the fixed-width sidebar on wide layouts.
- Retain horizontal category navigation on narrow layouts, while keeping “Back to main” visible and distinct from categories.
- Do not add another large “Settings” label on narrow layouts; the content heading remains the single primary heading.
- Content and navigation must not create page-level horizontal scrolling.

## Behavior and Scope

- Keep `onBack`, `onNavigate`, and the current `AppView` state model unchanged.
- Preserve unsaved-change confirmation behavior for Capabilities and Teams.
- Do not change categories, form fields, data APIs, or theme persistence.
- Do not refactor the main application sidebar or unrelated pages in this change.

## Tests and Acceptance

- The settings page no longer renders a separate top navigation region.
- “Back to main” lives inside settings navigation and invokes the existing back callback.
- The sidebar has no large “Settings” heading; the content heading remains present with correct hierarchy.
- The active category has both `aria-current="page"` and a visible state indicator that does not rely only on color.
- The back action and every category are keyboard accessible with a clear focus state.
- Wide and sub-960px layouts do not overflow, and the back action remains available.
- Existing navigation, unsaved-leave confirmation, and theme selection tests continue to pass.
- Desktop unit tests and the production frontend build pass.
