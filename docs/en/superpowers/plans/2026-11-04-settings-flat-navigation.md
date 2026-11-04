[简体中文](../../../zh-CN/superpowers/plans/2026-11-04-settings-flat-navigation.md) | English

# Flat Settings Navigation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the settings top navigation row, move the back action to the bottom of the sidebar, and apply the selected minimal linear style to the shell, heading, and appearance options.

**Architecture:** Preserve `SettingsShell` props, `AppView` state, and every callback while changing only semantic layout. Keep visual changes in `settings.css`, with `.settings-shell-content`-scoped `ManagementHero` overrides so the main application is unaffected.

**Tech Stack:** React 19, TypeScript, Lucide React, CSS, Vitest, React DOM server renderer

---

### Task 1: Move the back action into the settings sidebar

**Files:**
- Modify: `apps/archmage-desktop/src/SettingsShell.test.tsx:8-26`
- Modify: `apps/archmage-desktop/src/SettingsShell.tsx:22-38`

- [ ] **Step 1: Add the failing structure test**

Replace the test body in `SettingsShell.test.tsx` with:

```tsx
describe("SettingsShell", () => {
  it("renders flat settings navigation with the back action inside the sidebar", () => {
    const html = renderToStaticMarkup(
      <SettingsShell activeView="agents" onNavigate={() => undefined} onBack={() => undefined}>
        <div>Agent content</div>
      </SettingsShell>,
    );

    expect(html).toContain("返回主界面");
    expect(html).toContain("外观");
    expect(html).toContain("智能体管理");
    expect(html).toContain("能力管理");
    expect(html).toContain("团队管理");
    expect(html).toContain('aria-current="page"');
    expect(html).toContain("Agent content");
    expect(html).toContain('class="settings-sidebar-brand"');
    expect(html).toMatch(/<nav[^>]*class="settings-sections"[\s\S]*class="settings-back"/);
    expect(html).not.toContain("settings-shell-header");
    expect(html).not.toContain("settings-shell-title");
    expect(html).not.toContain("<h1");
  });
});
```

- [ ] **Step 2: Run the focused test and confirm it fails**

Run: `npm test -- SettingsShell.test.tsx`

Working directory: `apps/archmage-desktop`

Expected: FAIL because the old header remains and `settings-sidebar-brand` is missing.

- [ ] **Step 3: Apply the minimal semantic layout change**

Change the component return structure to:

```tsx
export function SettingsShell({ activeView, onNavigate, onBack, children }: SettingsShellProps) {
  return <section className="settings-shell">
    <div className="settings-shell-body">
      <nav className="settings-sections" aria-label="设置分类">
        <span className="settings-sidebar-brand">ARCHMAGE</span>
        <p>通用</p>
        {sections.slice(0, 1).map(({ view, label, icon: Icon }) => <button key={view} type="button" className={activeView === view ? "active" : ""} aria-current={activeView === view ? "page" : undefined} onClick={() => onNavigate(view)}><Icon size={17} />{label}</button>)}
        <p>资源管理</p>
        {sections.slice(1).map(({ view, label, icon: Icon }) => <button key={view} type="button" className={activeView === view ? "active" : ""} aria-current={activeView === view ? "page" : undefined} onClick={() => onNavigate(view)}><Icon size={17} />{label}</button>)}
        <button type="button" className="settings-back" onClick={onBack}><ArrowLeft size={17} />返回主界面</button>
      </nav>
      <div className="settings-shell-content">{children}</div>
    </div>
  </section>;
}
```

- [ ] **Step 4: Re-run the focused test**

Run: `npm test -- SettingsShell.test.tsx`

Working directory: `apps/archmage-desktop`

Expected: PASS; one test passes.

- [ ] **Step 5: Commit the semantic change**

```bash
git add apps/archmage-desktop/src/SettingsShell.tsx apps/archmage-desktop/src/SettingsShell.test.tsx
git commit -m "refactor: move settings back action into sidebar"
```

### Task 2: Apply the minimal linear settings style

**Files:**
- Modify: `apps/archmage-desktop/src/SettingsShell.test.tsx:6-30`
- Modify: `apps/archmage-desktop/src/settings.css:1-62`

- [ ] **Step 1: Add the failing style-contract assertions**

Add these assertions at the end of the existing test:

```tsx
expect(settingsStyles).toContain(".settings-sections button.active::before");
expect(settingsStyles).toContain(".settings-shell-content .management-hero");
expect(settingsStyles).toContain(".settings-shell-content .management-hero::after");
expect(settingsStyles).toContain("@media (max-width: 960px)");
expect(settingsStyles).not.toContain(".settings-shell-header");
expect(settingsStyles).not.toContain("box-shadow: var(--shadow)");
```

- [ ] **Step 2: Run the test and confirm it fails for the expected style gaps**

Run: `npm test -- SettingsShell.test.tsx`

Working directory: `apps/archmage-desktop`

Expected: FAIL because the new indicator and hero overrides are absent and old shadow declarations remain.

- [ ] **Step 3: Replace shell and sidebar CSS**

Replace the start of `settings.css` through `.settings-shell-content` with:

```css
.settings-view { max-width: 980px; margin: 0 auto; padding: 36px 0 72px; }
.settings-shell { min-height: 100vh; background: var(--canvas-bg); }
.settings-shell-body { min-height: 100vh; display: grid; grid-template-columns: 220px minmax(0, 1fr); }
.settings-sections { min-height: 100vh; padding: 28px 14px 18px; display: flex; flex-direction: column; border-right: 1px solid var(--border); background: var(--sidebar-bg); }
.settings-sidebar-brand { margin: 0 11px 26px; color: var(--accent); font: 700 9px ui-monospace, SFMono-Regular, monospace; letter-spacing: .14em; }
.settings-sections p { margin: 4px 11px 9px; color: var(--muted); font-size: 9px; font-weight: 750; letter-spacing: .12em; text-transform: uppercase; }
.settings-sections p:not(:first-of-type) { margin-top: 26px; }
.settings-sections button { position: relative; width: 100%; min-height: 44px; padding: 0 11px; display: flex; align-items: center; gap: 9px; border: 0; background: transparent; color: var(--muted); text-align: left; font-size: 11px; font-weight: 650; }
.settings-sections button:hover { background: var(--surface-2); color: var(--text); }
.settings-sections button:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
.settings-sections button.active { color: var(--accent-strong); }
.settings-sections button.active::before { content: ""; position: absolute; left: 0; top: 10px; bottom: 10px; width: 2px; background: var(--accent); }
.settings-back { margin-top: auto; padding-top: 1px; border-top: 1px solid var(--border) !important; color: var(--text) !important; }
.settings-shell-content { min-width: 0; padding: 0 30px 32px; }
```

- [ ] **Step 4: Flatten settings content and theme options**

Replace the `.settings-card`, `.theme-option`, and related state rules with the following, then keep the existing preview colors, text styles, and brand image rules unchanged:

```css
.settings-shell-content .management-hero { overflow: visible; padding: 0 0 24px; border: 0; border-bottom: 1px solid var(--border); border-radius: 0; background: transparent; }
.settings-shell-content .management-hero::after { display: none; }
.settings-card { margin-top: 0; padding: 24px 0; border: 0; border-radius: 0; background: transparent; }
.settings-card-heading h2 { margin: 0 0 5px; font-size: 17px; }
.settings-card-heading p { margin: 0; color: var(--muted); font-size: 11px; line-height: 1.5; }
.theme-options { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; margin-top: 18px; }
.theme-option { min-width: 0; padding: 12px; border: 1px solid var(--border); border-radius: 4px; background: transparent; color: var(--text); text-align: left; }
.theme-option:hover { border-color: var(--accent); background: var(--surface-2); }
.theme-option:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
.theme-option.selected { border-color: var(--accent); }
.theme-preview { height: 132px; display: grid; grid-template-columns: 28% 1fr; gap: 9px; padding: 10px; border: 1px solid; border-radius: 3px; }
.theme-preview-sidebar, .theme-preview-content, .theme-preview-content i { display: block; border-radius: 2px; }
```

- [ ] **Step 5: Replace the sub-960px responsive rules**

Replace the existing `@media (max-width: 960px)` block with:

```css
@media (max-width: 960px) {
  .settings-shell-body { min-height: 100vh; display: block; }
  .settings-sections { min-height: 0; padding: 10px 16px; display: flex; flex-direction: row; gap: 6px; overflow-x: auto; border-right: 0; border-bottom: 1px solid var(--border); }
  .settings-sidebar-brand, .settings-sections p { display: none; }
  .settings-sections button { width: auto; flex: 0 0 auto; padding: 0 12px; }
  .settings-sections button.active::before { left: 12px; right: 12px; top: auto; bottom: 0; width: auto; height: 2px; }
  .settings-back { position: sticky !important; left: 0; z-index: 1; order: -1; margin: 0; border-top: 0 !important; border-right: 1px solid var(--border) !important; background: var(--sidebar-bg) !important; }
  .settings-shell-content { padding: 0 16px 24px; }
  .settings-view { padding-top: 24px; }
  .theme-options { grid-template-columns: 1fr; }
}
```

- [ ] **Step 6: Run focused settings tests**

Run: `npm test -- SettingsShell.test.tsx SettingsView.test.tsx ManagementHero.test.tsx`

Working directory: `apps/archmage-desktop`

Expected: PASS for shell, appearance selection, and shared heading tests.

- [ ] **Step 7: Commit the flat style**

```bash
git add apps/archmage-desktop/src/settings.css apps/archmage-desktop/src/SettingsShell.test.tsx
git commit -m "style: flatten settings navigation and content"
```

### Task 3: Regression and visual verification

**Files:**
- Verify: `apps/archmage-desktop/src/SettingsShell.tsx`
- Verify: `apps/archmage-desktop/src/settings.css`

- [ ] **Step 1: Run all desktop unit tests**

Run: `npm test`

Working directory: `apps/archmage-desktop`

Expected: PASS with no failed tests or unhandled errors.

- [ ] **Step 2: Run the production build**

Run: `npm run build`

Working directory: `apps/archmage-desktop`

Expected: PASS for TypeScript and Vite.

- [ ] **Step 3: Start the preview and inspect the wide layout**

Run: `npm run dev -- --host 127.0.0.1`

Working directory: `apps/archmage-desktop`

Expected: No top settings row; small wordmark at sidebar top; thin active bar; back action at sidebar bottom; no elevated heading or appearance cards.

- [ ] **Step 4: Inspect narrow layout and keyboard operation**

Resize below 960px and Tab through the back action and four categories.

Expected: Horizontal navigation without page-level overflow, visible back action, bottom active indicator, and clear focus rings.

- [ ] **Step 5: Inspect the final diff**

Run: `git diff --check && git status --short`

Expected: No `git diff --check` output and only planned files present, or a clean worktree after task commits.
