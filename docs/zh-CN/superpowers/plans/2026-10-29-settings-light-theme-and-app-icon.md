# 设置、浅色主题和应用程序图标实施计划

简体中文 | [English](../../../en/superpowers/plans/2026-10-29-settings-light-theme-and-app-icon.md)

> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 添加左下角的“设置”目标、默认情况下带有灯光的持久 light/dark 外观卡，以及跨侧边栏、Web 图标和 Tauri 桌面资源的统一向导肖像图标。

**架构：** 将主题状态保存在独立于框架的小模块中，并让 `App` 拥有活动主题。通过 `data-theme` 和语义 CSS 变量应用主题，在专用 `SettingsView` 中渲染外观控件，并从一个批准的方形主图像中派生出每个图标资源。

**技术堆栈：** React 19、TypeScript、CSS 自定义属性、Vitest、Vite、Tauri 2 图标生成器

---

### 任务 1：主题状态和持久性

**文件：**
- 创建：`apps/archmage-desktop/src/theme.ts`
- 创建：`apps/archmage-desktop/src/theme.test.ts`
- 修改：`apps/archmage-desktop/src/main.tsx`

- [x] **第 1 步：编写失败的主题测试**

创建断言的测试：

```ts
expect(readTheme({ getItem: () => null })).toBe("light");
expect(readTheme({ getItem: () => "dark" })).toBe("dark");
expect(readTheme({ getItem: () => "unknown" })).toBe("light");
expect(readTheme({ getItem: () => { throw new Error("blocked"); } })).toBe("light");
```

还要验证 `applyTheme("dark", root)` 写入 `root.dataset.theme === "dark"` 和 `root.style.colorScheme === "dark"`，并验证持久性异常是否被吞掉。

- [x] **步骤 2：运行重点测试并验证 RED**

运行 `cd apps/archmage-desktop && npm test -- theme.test.ts`。

预期：失败，因为 `theme.ts` 不存在。

- [x] **第3步：实现主题域模块**

从 `theme.ts` 导出以下稳定 API：

```ts
export type Theme = "light" | "dark";
export const DEFAULT_THEME: Theme = "light";
export const THEME_STORAGE_KEY = "archmage.theme";
export function readTheme(storage?: Pick<Storage, "getItem">): Theme;
export function persistTheme(theme: Theme, storage?: Pick<Storage, "setItem">): void;
export function applyTheme(theme: Theme, root?: HTMLElement): void;
export function initializeTheme(): Theme;
```

`readTheme` 仅接受 `light` 和 `dark`；每个缺失、无效或异常的路径都会返回 `light`。`persistTheme` 捕获存储故障。`applyTheme` 设置 `data-theme` 和 `color-scheme`。`initializeTheme` 在 React 渲染之前读取并应用该值。

- [x] **第4步：安装React之前初始化主题**

在 `main.tsx` 中，在 `createRoot` 之前调用 `initializeTheme()`，并将结果传递给 `<App initialTheme={initialTheme} />`，以便第一个绘制的帧已使用正确的主题。

- [x] **第 5 步：验证绿色**

运行 `npm test -- theme.test.ts` 并期望所有主题测试都能通过。

### 任务 2：设置视图和导航

**文件：**
- 创建：`apps/archmage-desktop/src/SettingsView.tsx`
- 创建：`apps/archmage-desktop/src/SettingsView.test.tsx`
- 创建：`apps/archmage-desktop/src/settings.css`
- 修改：`apps/archmage-desktop/src/model.ts`
- 修改：`apps/archmage-desktop/src/model.test.ts`
- 修改：`apps/archmage-desktop/src/App.tsx`

- [x] **第 1 步：编写失败的设置和导航测试**

使用 `renderToStaticMarkup` 断言 `SettingsView` 呈现 `设置`、`外观`、`浅色`、`暗色`、两个 `theme-option` 按钮以及与 `theme` 属性匹配的活动状态。通过以下方式扩展导航测试：

```ts
expect(shouldConfirmCapabilityExit("capabilities", "settings", true)).toBe(true);
expect(shouldConfirmTeamExit("teams", "settings", true)).toBe(true);
```

- [x] **第 2 步：运行重点测试并验证 RED**

运行 `npm test -- SettingsView.test.tsx model.test.ts teamGraph.test.ts`。

预期：失败，因为 `SettingsView` 和 `settings` 视图不存在。

- [x] **第三步：实现外观页面**

使用道具创建 `SettingsView`：

```ts
type SettingsViewProps = {
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
};
```

使用 `SETTINGS`、标题 `设置` 和描述 `调整 Archmage 的使用体验。` 渲染共享 `ManagementHero`。渲染一张 white/near-white 外观卡，其中包含两个全卡按钮。每个按钮都包含一个微型 sidebar/content 预览、标签和选定的指示器。使用 `aria-pressed` 和可见的 `已选择` 文本作为活动主题。

- [x] **第 4 步：将设置添加到应用程序导航**

扩展 `AppView` 以包含 `settings`。将 `initialTheme?: Theme` 添加到 `App`，保持活动主题处于状态，应用并保留更改，在运行时页脚正上方添加齿轮设置按钮，并为新视图渲染 `SettingsView`。通过 `navigate("settings")` 路由来保留所有现有的脏编辑器防护。

- [x] **第 5 步：设置“设置”目标的样式**

在 `settings.css` 中，添加全角管理页面间距、单个外观卡、两列主题预览网格、使用语义变量的 selected/focus 状态以及低于 780px 的一列布局。在 `styles.css` 中，添加 `.sidebar-bottom`、`.settings-nav` 和单独的运行时状态行，以便设置保持固定在底部，而无需替换运行时指示器。

- [x] **第 6 步：验证绿色**

运行 `npm test -- SettingsView.test.tsx model.test.ts teamGraph.test.ts` 并期望所有重点测试都能通过。

### 任务3：语义明暗视觉系统

**文件：**
- 创建：`apps/archmage-desktop/src/theme.css`
- 修改：`apps/archmage-desktop/src/styles.css`
- 修改：`apps/archmage-desktop/src/management.css`
- 修改：`apps/archmage-desktop/src/agents.css`
- 修改：`apps/archmage-desktop/src/capabilities.css`
- 修改：`apps/archmage-desktop/src/teams.css`
- 修改：`apps/archmage-desktop/src/newRun.css`
- 修改：`apps/archmage-desktop/src/runTeam.css`
- 修改：`apps/archmage-desktop/src/inspector.css`
- 创建：`apps/archmage-desktop/src/themeStyles.test.ts`

- [x] **第 1 步：编写失败的主题合约测试**

读取 CSS 文件并断言 `theme.css` 包含这些变量的 `:root` 灯光默认值和 `:root[data-theme="dark"]` 覆盖：

```text
--bg --surface --surface-2 --surface-raised --panel-gradient
--border --muted --text --accent --accent-strong --accent-soft
--input-bg --sidebar-bg --hero-gradient --canvas-bg --shadow
--danger --danger-bg --warning --warning-bg --info --info-bg
```

断言每个组件样式表都使用 `var(--surface-raised)`、`var(--input-bg)` 或其他语义标记作为其主表面，而不是将主表面固定为深色十六进制值。

- [x] **第2步：运行主题合约测试并验证RED**

运行 `npm test -- themeStyles.test.ts`。

预期：失败，因为光默认值和语义表面标记不存在。

- [x] **步骤 3：定义浅色默认调色板和深色覆盖**

在 `theme.css` 中使用这些核心值，使调色板与现有结构样式表分开：

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

添加与当前黑暗体验匹配的 `:root[data-theme="dark"]` 值，包括 `--bg: #0a0d12`、`--surface: #11161e`、`--text: #edf3fa` 和 `--accent: #7cf2b0`。

- [x] **步骤 4：用语义标记替换组件颜色文字**

将所有主面板、输入、侧边栏、英雄、状态芯片、画布、节点、焦点状态和运行详细信息表面映射到上面的变量。仅当需要不同的语义状态时才添加狭义命名的变量。保持透明 SVG 命中目标和结构透明背景文字。确保每个页面在浅色和深色主题下都保持可读。

- [x] **步骤 5：更新文档元数据**

将 `index.html` 更改为光兼容元数据，并让 `applyTheme` 在运行时保持 `color-scheme` 同步。

- [x] **第 6 步：验证绿色**

运行 `npm test -- themeStyles.test.ts`，然后运行 `npm test`，并期望所有测试都能通过。

### 任务4：向导肖像和应用程序图标资源

**文件：**
- 来源参考：`/var/folders/ps/s092v8p54sl6dp8_gdsjx9pcv7414b/T/codex-clipboard-e9acca1f-432c-415f-92ee-38a9406420f6.png`
- 创建：`apps/archmage-desktop/public/archmage-icon.png`
- Generate/replace: `apps/archmage-desktop/src-tauri/icons/*`
- 修改：`apps/archmage-desktop/src/App.tsx`
- 修改：`apps/archmage-desktop/index.html`
- 修改：`apps/archmage-desktop/src-tauri/tauri.conf.json`
- 创建：`apps/archmage-desktop/src/iconAssets.test.ts`

- [x] **第 1 步：编写失败的图标资产合约测试**

断言侧边栏引用 `/archmage-icon.png`、`index.html` 引用 `/archmage-icon.png` 作为图标，Tauri 配置声明 `icons/icon.icns`、`icons/icon.ico` 和 PNG 图标，并且所有引用的文件均以非零大小存在。

- [x] **步骤 2：运行图标合约测试并验证 RED**

运行 `npm test -- iconAssets.test.ts`。

预期：失败，因为新的肖像资源和完整的图标集不存在。

- [x] **第3步：创建批准的主图标**

使用图像编辑工具并以提供的肖像作为参考。创建居中的头肩部裁剪，保留脸部、长发、高领和胸章，保留原始单色铅笔样式，将其放置在纯白色的 macOS 风格的圆角上，没有轮廓或阴影，并使圆角之外的区域保持透明。将结果保存为 `public/archmage-icon.png`。

- [x] **第4步：生成平台资产**

运行：

```bash
cd apps/archmage-desktop
npm run tauri -- icon public/archmage-icon.png --output src-tauri/icons
```

验证该命令是否生成 `icon.png`、`icon.icns`、`icon.ico` 和标准 PNG 尺寸。

- [x] **第 5 步：将图标连接到每个表面**

将侧边栏通用标记替换为 `<img src="/archmage-icon.png" alt="" />`，在 `index.html` 中添加 PNG favicon 链接，并在 `bundle.icon` 下的 `tauri.conf.json` 中显式列出生成的桌面图标路径。

- [x] **第 6 步：验证绿色**

运行 `npm test -- iconAssets.test.ts` 并期望所有图标引用和文件都通过。

### 任务5：端到端验证和交付

**文件：**
- 修改：此文件中的实施计划复选框

- [x] **第 1 步：运行完整的自动验证**

运行：

```bash
cd apps/archmage-desktop
npm test
npm run build
npm run tauri -- info
git diff --check
```

预期：零测试失败、成功的 Vite/TypeScript 构建、有效的 Tauri 配置并且没有空格错误。

- [x] **第 2 步：运行目视验证**

打开本地应用程序预览并检查两个主题中的设置以及代表性运行、代理、功能、团队和运行详细信息界面。确认可读文本、清晰的卡片、可见的对焦环、正确的状态颜色、即时主题切换以及侧边栏和图标中的肖像。启动 Tauri 开发应用程序并确认 macOS Dock/application 栏中出现相同的肖像。

- [x] **第 3 步：提交实施**

仅暂存计划、主题、设置、CSS、图标、Tauri 配置和相关测试，然后提交：

```bash
git commit -m "Add Settings themes and Archmage icon"
```
