[简体中文](../../../zh-CN/superpowers/plans/2026-10-29-white-borderless-app-icon.md) | English

# White Borderless App Icon Implementation Plan


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace every Archmage application icon with the approved wizard portrait on a pure-white rounded tile with no outline, shadow, tint, or edge artifacts.

**Architecture:** Keep `public/archmage-icon.png` as the single source of truth. Produce one visually approved transparent PNG, apply an exact rounded alpha mask outside the white tile, then regenerate the Tauri PNG, ICNS, and ICO assets from that master.

**Tech Stack:** Built-in image editor, PNG alpha channel, FFmpeg, Tauri 2 icon generator, Vitest

---

### Task 1: Produce and validate the white master icon

**Files:**
- Modify: `apps/archmage-desktop/public/archmage-icon.png`
- Test: `apps/archmage-desktop/src/iconAssets.test.ts`

- [x] **Step 1: Capture the current asset contract**

Run:

```bash
cd apps/archmage-desktop
npm test -- iconAssets.test.ts
sips -g pixelWidth -g pixelHeight -g hasAlpha public/archmage-icon.png
```

Expected: the icon-reference tests pass and the source PNG has an alpha channel.

- [x] **Step 2: Edit the approved portrait master**

Use the built-in image editor with `public/archmage-icon.png` as the edit target. Change only the rounded tile from pale gray-green to pure white. Preserve the wizard portrait, crop, facial features, hair, beard, collar, medallion, and monochrome pencil style. Require no outline, border, shadow, colored tint, halo, or stray pixels, with full transparency outside the rounded tile.

- [x] **Step 3: Apply a deterministic rounded alpha mask**

Use FFmpeg to keep the generated image inside the existing rounded-square geometry and set every pixel outside it to alpha zero. Do not alter RGB values inside the tile.

- [x] **Step 4: Inspect the master at full and small sizes**

Open the master and a 128-pixel preview. Confirm that the tile is visually pure white, its edge has no black outline or gray-green halo, the outside is transparent, and the portrait remains recognizable at Dock size.

### Task 2: Regenerate and verify every application icon

**Files:**
- Modify: `apps/archmage-desktop/src-tauri/icons/32x32.png`
- Modify: `apps/archmage-desktop/src-tauri/icons/128x128.png`
- Modify: `apps/archmage-desktop/src-tauri/icons/128x128@2x.png`
- Modify: `apps/archmage-desktop/src-tauri/icons/icon.png`
- Modify: `apps/archmage-desktop/src-tauri/icons/icon.icns`
- Modify: `apps/archmage-desktop/src-tauri/icons/icon.ico`

- [x] **Step 1: Generate the complete icon set from the master**

Run:

```bash
cd apps/archmage-desktop
npm run tauri -- icon public/archmage-icon.png --output src-tauri/icons
```

Expected: Tauri regenerates the standard PNG sizes plus ICNS and ICO without error.

- [x] **Step 2: Run focused and full verification**

Run:

```bash
cd apps/archmage-desktop
npm test -- iconAssets.test.ts
npm test
npm run build
cd ../..
git diff --check
```

Expected: 0 test failures, a successful production build, and no whitespace errors.

- [x] **Step 3: Restart and visually verify the desktop application**

Restart the existing Tauri development process so macOS reloads the ICNS resource. Confirm that the Dock/application bar, sidebar image, and browser favicon all use the same pure-white borderless icon.

- [x] **Step 4: Commit the replacement assets**

```bash
git add apps/archmage-desktop/public/archmage-icon.png \
  apps/archmage-desktop/src-tauri/icons/32x32.png \
  apps/archmage-desktop/src-tauri/icons/128x128.png \
  apps/archmage-desktop/src-tauri/icons/128x128@2x.png \
  apps/archmage-desktop/src-tauri/icons/icon.png \
  apps/archmage-desktop/src-tauri/icons/icon.icns \
  apps/archmage-desktop/src-tauri/icons/icon.ico \
  docs/superpowers/plans/2026-10-29-white-borderless-app-icon.md
git commit -m "Refine Archmage app icon background"
```
