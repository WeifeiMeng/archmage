简体中文 | [English](../../../en/superpowers/plans/2026-11-04-settings-flat-navigation.md)

# 设置页扁平导航实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 删除设置页顶部导航条，将返回入口放入侧栏底部，并把设置壳层、标题区和外观选项改为方案 A 的极简线性样式。

**Architecture:** 保持 `SettingsShell` 的属性、`AppView` 状态和所有回调不变，只重排壳层语义结构。视觉变化集中在 `settings.css`，通过 `.settings-shell-content` 作用域覆盖共享 `ManagementHero`，避免影响主界面。

**Tech Stack:** React 19、TypeScript、Lucide React、CSS、Vitest、React DOM server renderer

---

### Task 1: 将返回入口移入设置侧栏

**Files:**
- Modify: `apps/archmage-desktop/src/SettingsShell.test.tsx:8-26`
- Modify: `apps/archmage-desktop/src/SettingsShell.tsx:22-38`

- [ ] **Step 1: 写出失败的结构测试**

将 `SettingsShell.test.tsx` 中的测试替换为：

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

- [ ] **Step 2: 运行测试并确认正确失败**

Run: `npm test -- SettingsShell.test.tsx`

Working directory: `apps/archmage-desktop`

Expected: FAIL；输出显示仍存在 `settings-shell-header`，且缺少 `settings-sidebar-brand`。

- [ ] **Step 3: 最小化重排 `SettingsShell`**

将组件返回结构改为：

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

- [ ] **Step 4: 运行聚焦测试并确认通过**

Run: `npm test -- SettingsShell.test.tsx`

Working directory: `apps/archmage-desktop`

Expected: PASS；1 个测试通过。

- [ ] **Step 5: 提交结构改动**

```bash
git add apps/archmage-desktop/src/SettingsShell.tsx apps/archmage-desktop/src/SettingsShell.test.tsx
git commit -m "refactor: move settings back action into sidebar"
```

### Task 2: 实现极简线性设置样式

**Files:**
- Modify: `apps/archmage-desktop/src/SettingsShell.test.tsx:6-30`
- Modify: `apps/archmage-desktop/src/settings.css:1-62`

- [ ] **Step 1: 写出失败的样式契约测试**

在 `SettingsShell.test.tsx` 的现有测试末尾加入：

```tsx
expect(settingsStyles).toContain(".settings-sections button.active::before");
expect(settingsStyles).toContain(".settings-shell-content .management-hero");
expect(settingsStyles).toContain(".settings-shell-content .management-hero::after");
expect(settingsStyles).toContain("@media (max-width: 960px)");
expect(settingsStyles).not.toContain(".settings-shell-header");
expect(settingsStyles).not.toContain("box-shadow: var(--shadow)");
```

- [ ] **Step 2: 运行测试并确认正确失败**

Run: `npm test -- SettingsShell.test.tsx`

Working directory: `apps/archmage-desktop`

Expected: FAIL；输出显示缺少活动状态伪元素和扁平 `management-hero` 覆盖，同时仍有旧阴影声明。

- [ ] **Step 3: 替换设置壳层与侧栏样式**

将 `settings.css` 开头至 `.settings-shell-content` 替换为：

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

- [ ] **Step 4: 扁平化设置内容与主题选项**

用以下规则替换 `.settings-card`、`.theme-option` 和对应状态规则，并增加共享标题的设置作用域覆盖：

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

保留现有主题预览颜色、文字和品牌图标规则不变。

- [ ] **Step 5: 更新窄屏规则**

将现有 `@media (max-width: 960px)` 块替换为：

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

- [ ] **Step 6: 运行设置页聚焦测试**

Run: `npm test -- SettingsShell.test.tsx SettingsView.test.tsx ManagementHero.test.tsx`

Working directory: `apps/archmage-desktop`

Expected: PASS；设置壳层、外观选择和共享标题测试全部通过。

- [ ] **Step 7: 提交扁平样式**

```bash
git add apps/archmage-desktop/src/settings.css apps/archmage-desktop/src/SettingsShell.test.tsx
git commit -m "style: flatten settings navigation and content"
```

### Task 3: 回归验证与视觉检查

**Files:**
- Verify: `apps/archmage-desktop/src/SettingsShell.tsx`
- Verify: `apps/archmage-desktop/src/settings.css`

- [ ] **Step 1: 运行桌面端全部单元测试**

Run: `npm test`

Working directory: `apps/archmage-desktop`

Expected: PASS；全部 Vitest 测试通过，无失败或未处理异常。

- [ ] **Step 2: 运行生产构建**

Run: `npm run build`

Working directory: `apps/archmage-desktop`

Expected: PASS；TypeScript 编译和 Vite 构建成功。

- [ ] **Step 3: 启动预览并检查宽屏布局**

Run: `npm run dev -- --host 127.0.0.1`

Working directory: `apps/archmage-desktop`

Expected: 设置页没有顶部导航条；品牌字样位于侧栏顶部；活动分类有左侧细条；返回入口位于侧栏底部；右侧标题和外观区没有阴影或大型圆角。

- [ ] **Step 4: 检查窄屏布局和键盘操作**

在浏览器开发者工具中将视口缩至 960px 以下，并依次使用 Tab 聚焦返回入口和四个分类。

Expected: 导航横向排列且无页面级横向溢出；返回入口保持可见；活动项使用底部细条；所有交互项均有清晰焦点环。

- [ ] **Step 5: 检查最终差异**

Run: `git diff --check && git status --short`

Expected: `git diff --check` 无输出；工作区只包含计划内文件，或在任务提交后保持干净。

