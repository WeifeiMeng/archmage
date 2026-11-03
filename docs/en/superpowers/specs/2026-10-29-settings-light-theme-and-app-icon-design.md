[简体中文](../../../zh-CN/superpowers/specs/2026-10-29-settings-light-theme-and-app-icon-design.md) | English

# Settings page, light theme and app icon design


## target

Add an independent settings entrance and appearance settings page to Archmage, providing light and dark themes covering the entire application. Light color is used as the default theme for first startup, and user selection needs to be retained across startups. At the same time, the sidebar, web page and desktop application icons are uniformly replaced with the mage portrait provided by the user.

## Navigation and settings page

- Add a "Settings" button with a gear icon at the bottom of the left column.
- Settings is above the existing Local Runtime state; the Runtime state remains.
- Set as independent `AppView`. Click to enter the independent settings page and display the selected status in the sidebar.
- The first version of the settings page only contains the "Appearance" area and does not reserve general, runtime or other setting items without functions.
- The settings page uses the "theme preview card" layout: light and dark colors respectively display thumbnail previews, names and selected statuses, and you can switch by clicking on the entire card.
- Existing Capability and Team Editor unsaved exit protections continue to apply to navigation to the Settings page.

## subject behavior

- Supports two explicit themes, `light` and `dark`, without adding the "Follow System" option.
- Light color is used by default when there are no saved selections.
- Clicking on a theme card instantly updates the entire app, no additional save button is required.
- User selections are saved in browser/WebView local storage and restored next time.
- Falling back to light color when local storage is unavailable, reading fails, or the value is illegal does not prevent the application from starting.
- The application root node exposes the current theme via `data-theme="light|dark"` for use by all pages.
- `color-scheme` for HTML Makes native form controls and scrollbars look correct, consistent with the current theme.

## visual system

Light colors follow the confirmed "soft and natural" direction:

- The page background uses a light grayish green instead of pure white.
- The sidebar uses a slightly darker soft gray-green color to form a low-contrast separation from the main content area.
- Cards are white or near-white with a low-contrast gray-green border and a slight shadow.
- Use dark dark green for main text and neutral gray-green for secondary text.
- Continuation of the existing green brand color is used for main actions, selected states, focus and success states.
- Danger, warning and operational status colors are kept semantically clear, with contrast calibrated for light and dark colors respectively.

The dark theme is based on the current vision and does not change the overall style. During implementation, the page background, surface, input box, border, text, interactive state, shadow and canvas color are classified into semantic CSS variables to avoid maintaining two independent sets of style sheets or using filters to invert colors. The theme covers new Run, Agent, Capability, team management, settings page and Run details page.

## Icon design and products

- Use the user-provided mage portrait as the only source image.
- A head-and-shoulders half-length cut retains the face, long hair, collar and chest badge.
- Leave safe white space around the subject so that macOS corner masks, the Dock, the Windows taskbar, and small favicons don't cut off critical parts.
- Maintain the black-and-white line drawing quality of the original image without additional coloring; the icon uses a pure white rounded base plate with a transparent exterior.
- There should be no black strokes, shadows, gray-green blooming, or residual pixels on the rounded edges; the character composition, size, and details remain unchanged.
- Sidebar branding uses images instead of letters or generic square icons.
- Generate the PNG, macOS ICNS, Windows ICO and common size resources required for Tauri from the same master image.
- The web page `favicon` uses the same icon system and updates the page theme color element information.
- Tauri configuration explicitly references the new icon resource to ensure that development startup, installation packages, and Dock/taskbar use consistent icons.

## Component and data boundaries

- A new independent topic area module is added, responsible for topic type, default value, persistent reading, writing and root node synchronization.
- Added an independent `SettingsView`, which is only responsible for displaying the settings page and triggering theme changes.
- `App` holds the current theme and current page status, and passes the theme and change callback to the settings page.
- The theme module does not rely on Tauri API, ensuring consistent behavior of desktop WebView and browser preview.
- The settings page does not call the Runtime API and does not affect Run, Agent, Capability, or Team data.

## Error handling

- Catch storage exception when reading local theme selection and use light color.
- When writing fails, the topic switching in this session is still retained and no blocking error is displayed to the user.
- Icon generation and configuration verification are done during the build phase; missing resources should cause verification to fail, rather than silently using the old icon after release.

## Verify

- Unit tests cover default light color, legal selection recovery, illegal value fallback and storage exception fallback.
- Component testing covers setting navigation, light/dark card selection state and switching callbacks.
- Navigation test coverage still triggers protection when going to the settings page from an unsaved Capability or Team editor.
- Style checking covers the root theme selector and key semantic variables to prevent new pages from bypassing the theme system.
- Resource checks cover sidebar icons, favicon, PNG, ICNS, ICO, and Tauri configuration references.
- Run full frontend tests, production builds and Tauri config/icon checks.
- Check the readability, focus status, and status color of the new Run, the three management pages, the settings page, and the Run details page under the light and dark themes respectively.

## non-target

- This time, no system theme following, timed themes or more color schemes will be added.
- No runtime, account, notification or other settings will be added this time.
- This time, the business data, Runtime interface, team message flow or Run execution behavior will not be changed.
- This time we will not redesign the information structure of the dark theme.
