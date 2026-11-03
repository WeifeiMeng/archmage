[简体中文](../../../zh-CN/superpowers/plans/2026-10-29-settings-light-theme-and-app-icon.md) | English

# Settings, Light Theme, and App Icon Implementation Plan


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bottom-left Settings destination, persistent light/dark appearance cards with light as the default, and a unified wizard portrait icon across the sidebar, web favicon, and Tauri desktop assets.

**Architecture:** Keep theme state in a small framework-independent module and let `App` own the active theme. Apply the theme through `data-theme` and semantic CSS variables, render appearance controls in a dedicated `SettingsView`, and derive every icon asset from one approved square master image.

**Tech Stack:** React 19, TypeScript, CSS custom properties, Vitest, Vite, Tauri 2 icon generator

---

### Task 1: Theme state and persistence

**Files:**
- Create: `apps/archmage-desktop/src/theme.ts`
- Create: `apps/archmage-desktop/src/theme.test.ts`
- Modify: `apps/archmage-desktop/src/main.tsx`

- [x] **Step 1: Write failing theme tests**

Create tests that assert:

```ts
expect(readTheme({ getItem: () => null })).toBe("light");
expect(readTheme({ getItem: () => "dark" })).toBe("dark");
expect(readTheme({ getItem: () => "unknown" })).toBe("light");
expect(readTheme({ getItem: () => { throw new Error("blocked"); } })).toBe("light");
```

Also verify `applyTheme("dark", root)` writes both `root.dataset.theme === "dark"` and `root.style.colorScheme === "dark"`, and verify persistence exceptions are swallowed.

- [x] **Step 2: Run the focused test and verify RED**

Run `cd apps/archmage-desktop && npm test -- theme.test.ts`.

Expected: FAIL because `theme.ts` does not exist.

- [x] **Step 3: Implement the theme domain module**

Export the following stable API from `theme.ts`:

```ts
export type Theme = "light" | "dark";
export const DEFAULT_THEME: Theme = "light";
export const THEME_STORAGE_KEY = "archmage.theme";
export function readTheme(storage?: Pick<Storage, "getItem">): Theme;
export function persistTheme(theme: Theme, storage?: Pick<Storage, "setItem">): void;
export function applyTheme(theme: Theme, root?: HTMLElement): void;
export function initializeTheme(): Theme;
```

`readTheme` accepts only `light` and `dark`; every missing, invalid, or exceptional path returns `light`. `persistTheme` catches storage failures. `applyTheme` sets `data-theme` and `color-scheme`. `initializeTheme` reads and applies the value before React renders.

- [x] **Step 4: Initialize the theme before mounting React**

In `main.tsx`, call `initializeTheme()` before `createRoot` and pass the result to `<App initialTheme={initialTheme} />` so the first painted frame already uses the correct theme.

- [x] **Step 5: Verify GREEN**

Run `npm test -- theme.test.ts` and expect all theme tests to pass.

### Task 2: Settings view and navigation

**Files:**
- Create: `apps/archmage-desktop/src/SettingsView.tsx`
- Create: `apps/archmage-desktop/src/SettingsView.test.tsx`
- Create: `apps/archmage-desktop/src/settings.css`
- Modify: `apps/archmage-desktop/src/model.ts`
- Modify: `apps/archmage-desktop/src/model.test.ts`
- Modify: `apps/archmage-desktop/src/App.tsx`

- [x] **Step 1: Write failing settings and navigation tests**

Use `renderToStaticMarkup` to assert that `SettingsView` renders `设置`, `外观`, `浅色`, `暗色`, two `theme-option` buttons, and an active state matching the `theme` prop. Extend navigation tests with:

```ts
expect(shouldConfirmCapabilityExit("capabilities", "settings", true)).toBe(true);
expect(shouldConfirmTeamExit("teams", "settings", true)).toBe(true);
```

- [x] **Step 2: Run focused tests and verify RED**

Run `npm test -- SettingsView.test.tsx model.test.ts teamGraph.test.ts`.

Expected: FAIL because `SettingsView` and the `settings` view do not exist.

- [x] **Step 3: Implement the appearance page**

Create `SettingsView` with props:

```ts
type SettingsViewProps = {
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
};
```

Render a shared `ManagementHero` with `SETTINGS`, title `设置`, and description `调整 Archmage 的使用体验。`. Render one white/near-white appearance card containing two full-card buttons. Each button includes a miniature sidebar/content preview, label, and selected indicator. Use `aria-pressed` and visible `已选择` text for the active theme.

- [x] **Step 4: Add Settings to application navigation**

Extend `AppView` to include `settings`. Add `initialTheme?: Theme` to `App`, keep active theme in state, apply and persist it on changes, add a gear Settings button immediately above the Runtime footer, and render `SettingsView` for the new view. Preserve all existing dirty-editor guards by routing through `navigate("settings")`.

- [x] **Step 5: Style the Settings destination**

In `settings.css`, add full-width management-page spacing, a single appearance card, a two-column theme preview grid, selected/focus states using semantic variables, and a one-column layout below 780px. In `styles.css`, add `.sidebar-bottom`, `.settings-nav`, and a separate Runtime status row so Settings remains fixed at the bottom without replacing the Runtime indicator.

- [x] **Step 6: Verify GREEN**

Run `npm test -- SettingsView.test.tsx model.test.ts teamGraph.test.ts` and expect all focused tests to pass.

### Task 3: Semantic light and dark visual system

**Files:**
- Create: `apps/archmage-desktop/src/theme.css`
- Modify: `apps/archmage-desktop/src/styles.css`
- Modify: `apps/archmage-desktop/src/management.css`
- Modify: `apps/archmage-desktop/src/agents.css`
- Modify: `apps/archmage-desktop/src/capabilities.css`
- Modify: `apps/archmage-desktop/src/teams.css`
- Modify: `apps/archmage-desktop/src/newRun.css`
- Modify: `apps/archmage-desktop/src/runTeam.css`
- Modify: `apps/archmage-desktop/src/inspector.css`
- Create: `apps/archmage-desktop/src/themeStyles.test.ts`

- [x] **Step 1: Write a failing theme contract test**

Read the CSS files and assert that `theme.css` contains both `:root` light defaults and `:root[data-theme="dark"]` overrides for these variables:

```text
--bg --surface --surface-2 --surface-raised --panel-gradient
--border --muted --text --accent --accent-strong --accent-soft
--input-bg --sidebar-bg --hero-gradient --canvas-bg --shadow
--danger --danger-bg --warning --warning-bg --info --info-bg
```

Assert that every component stylesheet uses `var(--surface-raised)`, `var(--input-bg)`, or another semantic token for its primary surface rather than fixing the primary surface to a dark hex value.

- [x] **Step 2: Run the theme contract test and verify RED**

Run `npm test -- themeStyles.test.ts`.

Expected: FAIL because light defaults and the semantic surface tokens do not exist.

- [x] **Step 3: Define the light-default palette and dark override**

Use these core values in `theme.css`, keeping the palette separate from the existing structural stylesheet:

```css
:root {
  --bg: #f3f6f2;
  --surface: #ffffff;
  --surface-2: #e9efe9;
  --surface-raised: #ffffff;
  --border: #d7e2da;
  --muted: #68786e;
  --text: #17231d;
  --accent: #168a53;
  --accent-strong: #0f6f41;
  --accent-soft: #e1f2e7;
  --input-bg: #fbfcfb;
  --sidebar-bg: rgba(237, 242, 236, .96);
  --hero-gradient: linear-gradient(120deg, #edf4ef, #e8f2eb);
  --canvas-bg: #eef3ef;
  --shadow: 0 18px 55px rgba(37, 66, 50, .10);
  --danger: #b23b48;
  --danger-bg: #faecee;
  --warning: #8b6718;
  --warning-bg: #f8f1dd;
  --info: #356b83;
  --info-bg: #eaf3f7;
}
```

Add `:root[data-theme="dark"]` values matching the current dark experience, including `--bg: #0a0d12`, `--surface: #11161e`, `--text: #edf3fa`, and `--accent: #7cf2b0`.

- [x] **Step 4: Replace component color literals with semantic tokens**

Map all primary panels, inputs, sidebar, hero, status chips, canvas, nodes, focus states, and Run detail surfaces to the variables above. Add narrowly named variables only when a distinct semantic state is required. Keep transparent SVG hit targets and structural transparent backgrounds literal. Ensure every page remains readable in light and dark themes.

- [x] **Step 5: Update document metadata**

Change `index.html` to light-compatible metadata and let `applyTheme` keep `color-scheme` synchronized at runtime.

- [x] **Step 6: Verify GREEN**

Run `npm test -- themeStyles.test.ts`, then `npm test`, and expect all tests to pass.

### Task 4: Wizard portrait and application icon assets

**Files:**
- Source reference: `/var/folders/ps/s092v8p54sl6dp8_gdsjx9pcv7414b/T/codex-clipboard-e9acca1f-432c-415f-92ee-38a9406420f6.png`
- Create: `apps/archmage-desktop/public/archmage-icon.png`
- Generate/replace: `apps/archmage-desktop/src-tauri/icons/*`
- Modify: `apps/archmage-desktop/src/App.tsx`
- Modify: `apps/archmage-desktop/index.html`
- Modify: `apps/archmage-desktop/src-tauri/tauri.conf.json`
- Create: `apps/archmage-desktop/src/iconAssets.test.ts`

- [x] **Step 1: Write a failing icon asset contract test**

Assert that the sidebar references `/archmage-icon.png`, `index.html` references `/archmage-icon.png` as the favicon, Tauri config declares `icons/icon.icns`, `icons/icon.ico`, and PNG icons, and all referenced files exist with non-zero size.

- [x] **Step 2: Run the icon contract test and verify RED**

Run `npm test -- iconAssets.test.ts`.

Expected: FAIL because the new portrait asset and complete icon set do not exist.

- [x] **Step 3: Create the approved master icon**

Use the image editing tool with the supplied portrait as the reference. Create a centered head-and-shoulders crop that preserves the face, long hair, high collar, and chest medallion, keeps the original monochrome pencil style, places it on a pure-white macOS-style squircle with no outline or shadow, and leaves the area outside the squircle transparent. Save the result as `public/archmage-icon.png`.

- [x] **Step 4: Generate platform assets**

Run:

```bash
cd apps/archmage-desktop
npm run tauri -- icon public/archmage-icon.png --output src-tauri/icons
```

Verify that the command produces `icon.png`, `icon.icns`, `icon.ico`, and standard PNG sizes.

- [x] **Step 5: Wire the icon into every surface**

Replace the sidebar generic mark with `<img src="/archmage-icon.png" alt="" />`, add a PNG favicon link in `index.html`, and explicitly list the generated desktop icon paths in `tauri.conf.json` under `bundle.icon`.

- [x] **Step 6: Verify GREEN**

Run `npm test -- iconAssets.test.ts` and expect all icon references and files to pass.

### Task 5: End-to-end verification and delivery

**Files:**
- Modify: implementation-plan checkboxes in this file

- [x] **Step 1: Run complete automated verification**

Run:

```bash
cd apps/archmage-desktop
npm test
npm run build
npm run tauri -- info
git diff --check
```

Expected: zero test failures, successful Vite/TypeScript build, valid Tauri configuration, and no whitespace errors.

- [x] **Step 2: Run visual verification**

Open the local app preview and inspect Settings plus representative Run, Agent, Capability, Team, and Run detail surfaces in both themes. Confirm readable text, distinct cards, visible focus rings, correct state colors, immediate theme switching, and the portrait in the sidebar and favicon. Launch the Tauri development app and confirm the same portrait appears in the macOS Dock/application bar.

- [x] **Step 3: Commit the implementation**

Stage only the plan, theme, settings, CSS, icon, Tauri configuration, and related tests, then commit with:

```bash
git commit -m "Add Settings themes and Archmage icon"
```
