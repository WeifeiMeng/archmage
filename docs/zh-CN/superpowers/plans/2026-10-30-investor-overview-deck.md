# Archmage投资者概览套牌实施计划

简体中文 | [English](../../../en/superpowers/plans/2026-10-30-investor-overview-deck.md)

> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 制作一份经过验证的五张幻灯片的中国投资者演示文稿，以极简的视觉语言解释 Archmage。

**架构：** 使用捆绑的 Artifact Tool 从可编辑的演示对象构建平台。将创作文件和渲染预览保留在私有构建目录中，然后运行演示文稿终结器以在 `output/` 中创建单独的可交付成果。

**技术堆栈：** JavaScript ES 模块、`@oai/artifact-tool`、捆绑演示文稿终结器、PPTX、PNG 预览渲染

---

### 任务 1：准备创作工作区

**文件：**
- 创建：`.codex-build/archmage-investor-deck/build.mjs`
- 创建：`.codex-build/archmage-investor-deck/previews/`
- 创建：`output/`

- [ ] **第 1 步：加载捆绑的工作区运行时**

运行 Codex 工作区依赖加载器并记录返回的 Node.js 可执行文件、模块目录、二进制目录和创作命令的 Python 可执行文件。

- [ ] **第二步：标记演示操作**

运行：

```bash
node container_tools/mark_artifact_operation_started.mjs --operation-kind create --expected-output-count 1 --output-format pptx
```

从已安装的演示技能目录中。预期：操作标记成功且没有错误。

- [ ] **步骤 3：创建私有构建和输出目录**

创建 `.codex-build/archmage-investor-deck/previews` 和 `output`，并将捆绑的 `node_modules` 目录链接到私有构建目录中。预期：脚本可以导入 `@oai/artifact-tool` 而无需安装软件包。

### 任务 2：编写五张幻灯片

**文件：**
- 创建：`.codex-build/archmage-investor-deck/build.mjs`
- 创建：`.codex-build/archmage-investor-deck/previews/slide-01.png`
- 创建：`.codex-build/archmage-investor-deck/previews/slide-02.png`
- 创建：`.codex-build/archmage-investor-deck/previews/slide-03.png`
- 创建：`.codex-build/archmage-investor-deck/previews/slide-04.png`
- 创建：`.codex-build/archmage-investor-deck/previews/slide-05.png`
- 创建：`output/Archmage-投资人概览-5页.pptx`

- [ ] **第 1 步：实施共享视觉系统**

使用 1280×720 画布、温暖的灰白色背景 `F7F6F2`、近乎黑色的文本 `171A17`、静音文本 `666B66` 和鼠尾草口音 `5F7A68`。使用从捆绑运行时解析的支持中文的无衬线字体。将每张幻灯片保留一个主要信息，没有渐变、卡片网格、库存图像、装饰条纹或装饰图标。

- [ ] **第 2 步：实施五张幻灯片的叙述**

准确创建这些幻灯片：

1. `Archmage` — 副标题 `让本地 AI Agent 围绕同一代码库协作`。
2. `AI 编程进入协作瓶颈` — 显示四个短痛点：手动上下文传输、不透明的决策、工作空间冲突、碎片证据。
3. `把 Agent 组织成能力系统` — 显示能力、上下文、契约和工作空间作为四个产品原则。
4. `一次目标，形成研发闭环` — 显示可编辑流程 `设计 / 编码 / 测试 / 评审 / 修复`，以及简洁的投资者价值标签。
5. `本地优先的协作运行时` — 显示桌面和 CLI、Rust 运行时服务、提供程序适配器、Git worktree和本地持久性的可编辑层；将多提供商支持标记为架构方向。

- [ ] **第 3 步：添加源注释**

添加演讲者注释，标识 `README.md`、`docs/product/vision.md`、`docs/product/mvp-definition.md`、`docs/architecture/overview.md` 和 `docs/concepts/collaboration-model.md` 作为该套牌所使用的产品来源。

- [ ] **第 4 步：导出预览并完成 PPTX**

将所有五张幻灯片渲染为全尺寸 PNG 文件。导出私有候选 PPTX 并使用 `explicitTotalSlideCount: 5`、空本机表格和图表要求、设计字体策略以及预期的 16:9 幻灯片大小调用捆绑的终结器。预期：终结器写入 `output/Archmage-投资人概览-5页.pptx`。

### 任务3：验证和修复

**文件：**
- 修改：`.codex-build/archmage-investor-deck/build.mjs`
- 如果需要，请替换为新的最终修订版：`output/Archmage-投资人概览-5页.pptx`

- [ ] **第 1 步：检查每张幻灯片预览**

检查所有五个全尺寸 PNG 是否存在剪裁、重叠、意外换行、对比度弱、边距低于 48 像素、图表拥挤或最小视觉方向丢失的情况。

- [ ] **第二步：检查内容和包装**

提取演示文稿文本并确认正好五张幻灯片、中文措辞正确、顺序正确且没有占位符。确认验证收据报告包、布局、字体和 Artifact Tool 导入检查成功。

- [ ] **第3步：修复和再生**

如果出现任何缺陷，请更新创作脚本，重新生成每个受影响的预览，最终确定为新的输出文件名，然后重复检查。预期：不存在可见的剪裁、重叠、占位符或不受支持的声明。
