[简体中文](../../../zh-CN/superpowers/specs/2026-10-29-sidebar-brand-and-navigation-localization-design.md) | English

# Sidebar Branding and Navigation Chinese Culture Design


## target

Enhance the recognition of the brand area in the upper left corner, and change the main navigation in the sidebar to natural and concise Chinese expressions. This transformation only affects the sidebar and does not modify the title, description or data structure of each management page.

## brand area

- The brand icon is enlarged from `36px` to `44px`, while the rounded corners and existing image assets remain unchanged.
- The image inside the brand container is enlarged using `scale(1.15)` to offset the transparent safety margins that come with standard app icons; the container still maintains `44px`.
- Brand images use `contrast(1.18)` to improve the contrast between light and dark in line art. Zoom and contrast only apply to the upper left corner of the app and do not modify the Dock, favicon or installation package icon resources.
- The "Archmage" title font size is enlarged from `15px` to `18px`.
- The English subtitle "Agent Workbench" is changed to the Chinese "Agent Workbench", and the font size is `12px`.
- The icon, title, and subtitle remain vertically centered and must not wrap or overflow under either the `264px` standard sidebar or the `220px` narrow sidebar.

## Navigation copy and icons

- Change "New Run" to "New Task" and continue to use the plus icon.
- "Agent Management" is changed to "Agent Management" and the robot icon continues to be used.
- "Capability Management" is changed to "Capability Management", and the connection relationship icon continues to be used.
- “Team Management” retains the Chinese copy, and the icon is changed from a connection relationship icon to a two-person team icon to clearly distinguish it from capability management.
- Navigation button interaction, selection, spacing, and jump behavior remain unchanged.

## Verify

- The static rendering test covers four Chinese navigation copywriting, Chinese brand subtitles and team icons.
- Style contract testing covers `scale(1.15)` and `contrast(1.18)` for brand images.
- Check the standard sidebar and narrow sidebar under the light theme to confirm that there are no line breaks in the brand area, the icon proportions are coordinated, and the navigation selected state is not affected.
- Run full test and production builds to confirm there are no regressions.
