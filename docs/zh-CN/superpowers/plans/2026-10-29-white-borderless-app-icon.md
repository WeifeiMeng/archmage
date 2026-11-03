# 白色无边框应用程序图标实施计划

简体中文 | [English](../../../en/superpowers/plans/2026-10-29-white-borderless-app-icon.md)

> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 将每个Archmage应用程序图标替换为纯白色圆形瓷砖上经过批准的巫师肖像，没有轮廓、阴影、色调或边缘伪影。

**架构：** 将 `public/archmage-icon.png` 保留为单一事实来源。生成一个视觉上认可的透明 PNG，在白色图块外部应用精确的圆形 alpha 蒙版，然后从该母版重新生成 Tauri PNG、ICNS 和 ICO 资源。

**技术堆栈：** 内置图像编辑器、PNG alpha 通道、FFmpeg、Tauri 2 图标生成器、Vitest

---

### 任务1：制作并验证白色主图标

**文件：**
- 修改：`apps/archmage-desktop/public/archmage-icon.png`
- 测试： `apps/archmage-desktop/src/iconAssets.test.ts`

- [x] **第1步：获取当前资产合约**

运行：

```bash
cd apps/archmage-desktop
npm test -- iconAssets.test.ts
sips -g pixelWidth -g pixelHeight -g hasAlpha public/archmage-icon.png
```

预期：图标参考测试通过并且源 PNG 具有 Alpha 通道。

- [x] **第二步：编辑批准的肖像大师**

使用内置图像编辑器，以 `public/archmage-icon.png` 作为编辑目标。仅将圆形瓷砖从浅灰绿色更改为纯白色。保留巫师肖像、裁剪、面部特征、头发、胡须、衣领、徽章和单色铅笔风格。不需要轮廓、边框、阴影、彩色色调、光晕或杂散像素，圆形图块外部具有完全透明度。

- [x] **第 3 步：应用确定性圆形 alpha 蒙版**

使用 FFmpeg 将生成的图像保留在现有圆角正方形几何形状内，并将其外部的每个像素设置为 alpha 零。不要更改图块内的 RGB 值。

- [x] **步骤 4：检查全尺寸和小尺寸的母版**

打开母版和 128 像素预览。确认图块在视觉上是纯白色的，其边缘没有黑色轮廓或灰绿色光晕，外部是透明的，并且肖像在 Dock 尺寸下仍然可识别。

### 任务 2：重新生成并验证每个应用程序图标

**文件：**
- 修改：`apps/archmage-desktop/src-tauri/icons/32x32.png`
- 修改：`apps/archmage-desktop/src-tauri/icons/128x128.png`
- 修改：`apps/archmage-desktop/src-tauri/icons/128x128@2x.png`
- 修改：`apps/archmage-desktop/src-tauri/icons/icon.png`
- 修改：`apps/archmage-desktop/src-tauri/icons/icon.icns`
- 修改：`apps/archmage-desktop/src-tauri/icons/icon.ico`

- [x] **第1步：从master生成完整的图标集**

运行：

```bash
cd apps/archmage-desktop
npm run tauri -- icon public/archmage-icon.png --output src-tauri/icons
```

预期：Tauri 重新生成标准 PNG 尺寸以及 ICNS 和 ICO，不会出现错误。

- [x] **第 2 步：运行集中且全面的验证**

运行：

```bash
cd apps/archmage-desktop
npm test -- iconAssets.test.ts
npm test
npm run build
cd ../..
git diff --check
```

预期：0 次测试失败，生产构建成功，并且没有空格错误。

- [x] **步骤 3：重新启动并目视验证桌面应用程序**

重新启动现有的 Tauri 开发进程，以便 macOS 重新加载 ICNS 资源。确认 Dock/application 栏、侧边栏图像和浏览器图标都使用相同的纯白色无边框图标。

- [x] **第 4 步：提交替换资产**

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
