[简体中文](../../../zh-CN/superpowers/specs/2026-10-31-settings-management-hub-design.md) | English

# Set up the admin center design


## target

Move "Agent Management, Capability Management, Team Management" from the sidebar of the main page into settings. The setting adopts an independent full-screen layout, does not display the sidebar of the main page, and only provides a clear "return to the main interface" entrance.

## Page structure

- The sidebar of the main interface only retains: new tasks, recent runs, settings, and local runtime status.
- After entering the settings, the entire workspace switches to an independent settings shell: "Return to the main interface" and "Settings" are displayed at the top.
- Setting up the internal left category navigation is divided into:
  - General: Appearance
  - Resource management: agent management, capability management, team management
- The content area on the right directly reuses the existing four management pages without changing their data reading, saving and error display logic.

## Navigation and Status

- `settings` continues to serve as the boundary between the main page and the setting space; an independent classification status is added inside the setting.
- "Appearance" is turned on by default when entering Settings from the main sidebar; switching between pages within Settings does not redisplay the main sidebar.
- "Go to capability management" in agent management is changed to toggle settings internal classification.
- On the new task page, "Go to team management/configuration capability" is changed to first enter settings and then locate the corresponding category.
- When there are unsaved modifications to abilities or teams, the existing confirmation prompt will be used when switching setting categories or returning to the main interface; after canceling, the current category will remain.

## visual principles

- The setting uses a full-width independent shell to avoid the main sidebar and the setting category bar appearing at the same time.
- The back button is in the upper left corner, the category bar remains fixed-width, and the admin page takes up the remaining space.
- Inherit the current theme variables and admin page styles without redesigning existing forms and cards.

## Acceptance

- Agent, ability, and team entries no longer appear in the main sidebar.
- The settings page hides the main sidebar and returns to the new task main interface.
- All four settings categories are accessible, and jumps across categories are correct.
- Unsaved departure confirmations for abilities and teams remain in effect.
- Desktop unit tests and front-end build passed.
