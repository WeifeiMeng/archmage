# Sidebar Brand and Navigation Localization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enlarge the Archmage sidebar brand, localize its subtitle and primary navigation, and give Team management a distinct team icon.

**Architecture:** Keep the existing sidebar component and navigation behavior intact. Change only the rendered labels and Lucide icon selection in `App.tsx`, adjust the existing brand CSS dimensions, and add a source-level contract test for the approved copy, icon, and sizes.

**Tech Stack:** React 19, TypeScript, Lucide React, CSS, Vitest

---

### Task 1: Sidebar brand and navigation refresh

**Files:**
- Create: `apps/archmage-desktop/src/sidebarNavigation.test.ts`
- Modify: `apps/archmage-desktop/src/App.tsx`
- Modify: `apps/archmage-desktop/src/styles.css`

- [x] **Step 1: Write the failing sidebar contract test**

Create `sidebarNavigation.test.ts` with:

```ts
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const appSource = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");
const styles = readFileSync(new URL("./styles.css", import.meta.url), "utf8");

describe("sidebar brand and navigation", () => {
  it("uses the approved Chinese labels and a distinct team icon", () => {
    expect(appSource).toContain("智能体工作台");
    expect(appSource).toContain("新建任务");
    expect(appSource).toContain("智能体管理");
    expect(appSource).toContain("能力管理");
    expect(appSource).toContain("<UsersRound size={17} />团队管理");
  });

  it("uses the approved balanced brand size", () => {
    expect(styles).toContain(".brand-mark{width:44px;height:44px");
    expect(styles).toContain(".brand strong{font-size:18px}");
    expect(styles).toContain(".brand span{font-size:12px");
  });
});
```

- [x] **Step 2: Run the focused test and verify RED**

Run:

```bash
cd apps/archmage-desktop
npm test -- sidebarNavigation.test.ts
```

Expected: both tests fail because the old English labels, shared `Share2` team icon, and compact brand sizes are still present.

- [x] **Step 3: Update the sidebar markup**

In `App.tsx`, add `UsersRound` to the Lucide imports and change the brand/navigation block to:

```tsx
<div className="brand">
  <div className="brand-mark"><img src="/archmage-icon.png" alt="" /></div>
  <div><strong>Archmage</strong><span>智能体工作台</span></div>
</div>
<button className={`new-run ${view === "runs" && selectedId === null ? "active" : ""}`} onClick={() => navigate("runs", null)}><Plus size={17} />新建任务</button>
<button className={`new-run ${view === "agents" ? "active" : ""}`} onClick={() => navigate("agents")}><Bot size={17} />智能体管理</button>
<button className={`new-run ${view === "capabilities" ? "active" : ""}`} onClick={() => navigate("capabilities")}><Share2 size={17} />能力管理</button>
<button className={`new-run ${view === "teams" ? "active" : ""}`} onClick={() => navigate("teams")}><UsersRound size={17} />团队管理</button>
```

- [x] **Step 4: Apply the balanced A brand dimensions**

In `styles.css`, change only these declarations:

```css
.brand-mark{width:44px;height:44px}
.brand strong{font-size:18px}
.brand span{font-size:12px}
```

Keep the existing border, radius, color, margin, and sidebar widths unchanged.

- [x] **Step 5: Run focused and complete verification**

Run:

```bash
cd apps/archmage-desktop
npm test -- sidebarNavigation.test.ts
npm test
npm run build
cd ../..
git diff --check
```

Expected: all tests pass, the production build succeeds, and there are no whitespace errors.

- [x] **Step 6: Visually verify both sidebar widths**

Open the local preview at the standard viewport and at a width below the `1050px` breakpoint. Confirm the 44px icon, 18px title, Chinese subtitle, four localized labels, distinct team icon, no wrapping, and unchanged selected states.

- [x] **Step 7: Commit the implementation**

```bash
git add apps/archmage-desktop/src/App.tsx \
  apps/archmage-desktop/src/styles.css \
  apps/archmage-desktop/src/sidebarNavigation.test.ts \
  docs/superpowers/plans/2026-10-29-sidebar-brand-and-navigation-localization.md
git commit -m "Refresh sidebar brand and navigation labels"
```

### Task 2: Emphasize the in-app wizard portrait

**Files:**
- Modify: `apps/archmage-desktop/src/sidebarNavigation.test.ts`
- Modify: `apps/archmage-desktop/src/settings.css`

- [x] **Step 1: Extend the style contract test**

Read `settings.css` in `sidebarNavigation.test.ts` and assert that the brand image rule contains `transform: scale(1.15)` and `filter: contrast(1.18)`.

- [x] **Step 2: Run the focused test and verify RED**

Run `cd apps/archmage-desktop && npm test -- sidebarNavigation.test.ts`.

Expected: the new assertion fails because the image currently has no transform or contrast filter.

- [x] **Step 3: Apply the approved image-only emphasis**

Update the existing `.brand-mark img` rule in `settings.css` to:

```css
.brand-mark img {
  width: 100%;
  height: 100%;
  display: block;
  object-fit: cover;
  transform: scale(1.15);
  filter: contrast(1.18);
}
```

Do not change the `44px` brand container or the shared PNG asset.

- [x] **Step 4: Run focused and complete verification**

Run:

```bash
cd apps/archmage-desktop
npm test -- sidebarNavigation.test.ts
npm test
npm run build
cd ../..
git diff --check
```

Expected: all tests pass, the production build succeeds, and there are no whitespace errors.

- [x] **Step 5: Visually verify and commit**

Inspect the light-theme sidebar and confirm that the portrait is visibly larger and higher contrast without escaping the rounded `44px` container. Commit with:

```bash
git add apps/archmage-desktop/src/settings.css \
  apps/archmage-desktop/src/sidebarNavigation.test.ts \
  docs/superpowers/plans/2026-10-29-sidebar-brand-and-navigation-localization.md
git commit -m "Emphasize sidebar wizard portrait"
```
