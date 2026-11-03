简体中文 | [English](../../../en/superpowers/plans/2026-10-29-sidebar-brand-and-navigation-localization.md)

# 侧边栏品牌及导航本地化实施方案


> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 扩大Archmage侧边栏品牌，本地化其副标题和主导航，并为团队管理提供独特的团队图标。

**架构：** 保持现有侧边栏组件和导航行为完整。仅更改 `App.tsx` 中渲染的标签和 Lucide 图标选择，调整现有品牌 CSS 尺寸，并为批准的副本、图标和尺寸添加源级合同测试。

**技术堆栈：** React 19、TypeScript、Lucide React、CSS、Vitest

---

### 任务1：侧边栏品牌和导航刷新

**文件：**
- 创建：`apps/archmage-desktop/src/sidebarNavigation.test.ts`
- 修改：`apps/archmage-desktop/src/App.tsx`
- 修改：`apps/archmage-desktop/src/styles.css`

- [x] **第 1 步：编写失败的侧边栏合约测试**

使用以下命令创建 `sidebarNavigation.test.ts`：

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

- [x] **步骤 2：运行重点测试并验证 RED**

运行：

```bash
cd apps/archmage-desktop
npm test -- sidebarNavigation.test.ts
```

预期：两项测试均失败，因为旧的英文标签、共享 `Share2` 团队图标和紧凑的品牌尺寸仍然存在。

- [x] **第 3 步：更新侧边栏标记**

在 `App.tsx` 中，将 `UsersRound` 添加到 Lucide 导入中，并将 brand/navigation 块更改为：

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

- [x] **步骤 4：应用平衡的 A 品牌尺寸**

在 `styles.css` 中，仅更改以下声明：

```css
.brand-mark{width:44px;height:44px}
.brand strong{font-size:18px}
.brand span{font-size:12px}
```

保持现有边框、半径、颜色、边距和侧边栏宽度不变。

- [x] **第 5 步：运行集中且完整的验证**

运行：

```bash
cd apps/archmage-desktop
npm test -- sidebarNavigation.test.ts
npm test
npm run build
cd ../..
git diff --check
```

预期：所有测试都通过，生产构建成功，并且没有空格错误。

- [x] **第 6 步：目视验证两个侧边栏宽度**

在标准视口中以低于 `1050px` 断点的宽度打开本地预览。确认 44px 图标、18px 标题、中文字幕、四个本地化标签、清晰的团队图标、无换行以及未更改的选定状态。

- [x] **第 7 步：提交实施**

```bash
git add apps/archmage-desktop/src/App.tsx \
  apps/archmage-desktop/src/styles.css \
  apps/archmage-desktop/src/sidebarNavigation.test.ts \
  docs/superpowers/plans/2026-10-29-sidebar-brand-and-navigation-localization.md
git commit -m "Refresh sidebar brand and navigation labels"
```

### 任务2：强调应用内向导肖像

**文件：**
- 修改：`apps/archmage-desktop/src/sidebarNavigation.test.ts`
- 修改：`apps/archmage-desktop/src/settings.css`

- [x] **第 1 步：扩展样式契约测试**

读取 `sidebarNavigation.test.ts` 中的 `settings.css`，断言品牌形象规则包含 `transform: scale(1.15)` 和 `filter: contrast(1.18)`。

- [x] **步骤 2：运行重点测试并验证 RED**

运行 `cd apps/archmage-desktop && npm test -- sidebarNavigation.test.ts`。

预期：新断言失败，因为图像当前没有变换或对比度过滤器。

- [x] **第 3 步：应用批准的仅图像强调**

将 `settings.css` 中现有的 `.brand-mark img` 规则更新为：

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

请勿更改 `44px` 品牌容器或共享 PNG 资源。

- [x] **第 4 步：运行集中且完整的验证**

运行：

```bash
cd apps/archmage-desktop
npm test -- sidebarNavigation.test.ts
npm test
npm run build
cd ../..
git diff --check
```

预期：所有测试都通过，生产构建成功，并且没有空格错误。

- [x] **步骤 5：目视验证并提交**

检查浅色主题侧边栏，并确认肖像明显更大且对比度更高，而无需逃脱圆形 `44px` 容器。承诺：

```bash
git add apps/archmage-desktop/src/settings.css \
  apps/archmage-desktop/src/sidebarNavigation.test.ts \
  docs/superpowers/plans/2026-10-29-sidebar-brand-and-navigation-localization.md
git commit -m "Emphasize sidebar wizard portrait"
```
