# 双语文献系统实施方案

简体中文 | [English](../../../en/superpowers/plans/2026-11-03-bilingual-documentation-system.md)

> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 将每个 Archmage 文档移至镜像的英文和简体中文树中，与英文同行一起使存储库登陆页面中文优先，并验证完整的双语覆盖和有效链接。

**架构：** `docs/en/` 和 `docs/zh-CN/` 包含相同的 Markdown 路径和相同语言的内部链接。首先镜像现有文档，然后在不更改代码、命令、状态或历史证据的情况下翻译每个对等点。根 `README.md` 和 `README.en.md` 形成单独的双语对。

**技术堆栈：** Markdown、Git、shell 实用程序（`rg`、`comm`、`find`）、存储库源代码作为当前行为的权限。

---

### 任务 1：建立镜像目录布局和根 README 对

**文件：**
- 移动：将每个当前 `docs/**/*.md` 文件移动到 `docs/en/<same-relative-path>`
- 创建：每个匹配的 `docs/zh-CN/<same-relative-path>`
- 创建：`README.en.md`
- 修改：`README.md`

- [ ] **第 1 步：获取迁移前库存**

```bash
rg --files docs -g '*.md' | sort > /tmp/archmage-docs-before.txt
wc -l /tmp/archmage-docs-before.txt
```

预期：添加此计划后的 `72` Markdown 文件。

- [ ] **第2步：将当前树移动到`docs/en/`并镜像**

```bash
mkdir -p docs/en docs/zh-CN
while IFS= read -r path; do
  relative=${path#docs/}
  mkdir -p "docs/en/$(dirname "$relative")"
  git mv "$path" "docs/en/$relative"
done < /tmp/archmage-docs-before.txt
cp -R docs/en/. docs/zh-CN/
```

预期：下面的两个命令都会生成相同的路径列表，并且 `docs/` 下不会直接保留任何 Markdown。

```bash
(cd docs/en && rg --files -g '*.md' | sort) > /tmp/archmage-en.txt
(cd docs/zh-CN && rg --files -g '*.md' | sort) > /tmp/archmage-zh.txt
comm -3 /tmp/archmage-en.txt /tmp/archmage-zh.txt
find docs -mindepth 1 -maxdepth 1 -type f -name '*.md'
```

- [ ] **第 3 步：拆分并翻译根 README**

将当前英文项目页面复制到 `README.en.md`。在其标题上方添加 `[简体中文](README.md) | English` 并将其链接到 `docs/en/README.md`。将完整的根 `README.md` 翻译成简体中文，在其标题上方添加 `简体中文 | [English](README.en.md)`，并将其链接到 `docs/zh-CN/README.md`。保留所有命令和当前产品资格。

- [ ] **第 4 步：验证并提交布局**

```bash
test -f README.en.md
test -f docs/en/README.md
test -f docs/zh-CN/README.md
git diff --check
git add README.md README.en.md docs/en docs/zh-CN
git commit -m "docs: establish bilingual documentation layout"
```

### 任务 2：翻译索引和当前面向用户的文档

**文件：**
- 修改：`docs/{en,zh-CN}/README.md`
- 修改：`docs/{en,zh-CN}/cli-reference.md`
- 修改：`docs/{en,zh-CN}/guide.md`
- 修改：`docs/{en,zh-CN}/guide-online-doctor.md`
- 修改：`docs/{en,zh-CN}/product/*.md`
- 修改：`docs/{en,zh-CN}/concepts/*.md`
- 修改：`docs/{en,zh-CN}/architecture/*.md`
- 修改：`docs/{en,zh-CN}/roadmap/**/*.md`
- 修改：`docs/{en,zh-CN}/runbooks/*.md`

- [ ] **步骤 1：保留源语言副本并翻译其同行**

在 `zh-CN` 中保留现有的中文散文，以用于 CLI/current 指南和桌面操作手册。在 `en` 中保留产品、概念、架构、路线图和剩余操作手册的现有英文散文。完整翻译每个同行；不总结。

- [ ] **第 2 步：在每个标题之前添加语言切换器**

在正确的嵌套深度处使用相同的图案。例如：

```markdown
# docs/en/architecture/overview.md
[简体中文](../../zh-CN/architecture/overview.md) | English

# docs/zh-CN/architecture/overview.md
简体中文 | [English](../../en/architecture/overview.md)
```

- [ ] **步骤3：修复添加的语言目录级别的链接**

将文档链接保留在活动语言树内。添加一个父级别以链接到 `docs/` 之外的目标存储库文件。逐字保留命令、标志、环境变量、JSON 字段、事件名称、文件路径、日期和验证结果。

- [ ] **第 4 步：验证并提交当前文档**

```bash
for root in docs/en docs/zh-CN; do
  rg --files "$root" -g '*.md' -g '!superpowers/**' -g '!adr/**' -g '!acceptance/**' | while IFS= read -r file; do
    head -n 1 "$file"
  done
done
git diff --check
git add docs/en docs/zh-CN
git commit -m "docs: translate current product documentation"
```

预期：每个打印的第一行都是语言切换器并且格式化成功。

### 任务 3：翻译 ADR 和验收记录

**文件：**
- 修改：`docs/{en,zh-CN}/adr/0001-deliver-vertical-slices.md`
- 修改：`docs/{en,zh-CN}/adr/0002-separate-provider-and-capability-agent.md`
- 修改：`docs/{en,zh-CN}/adr/0003-use-explicit-context-handoffs.md`
- 修改：`docs/{en,zh-CN}/adr/0004-use-per-run-git-worktrees.md`
- 修改：`docs/{en,zh-CN}/adr/0005-use-real-codex-only-in-product-flow.md`
- 修改：`docs/{en,zh-CN}/acceptance/s0/batch-a-progress.md`
- 修改：`docs/{en,zh-CN}/acceptance/s0/batch-b-progress.md`
- 修改：`docs/{en,zh-CN}/acceptance/s0/batch-c-progress.md`

- [ ] **第 1 步：翻译所有 ADR 对**

保留决策状态、上下文、后果、取代关系和提供者名称。翻译早期决策时不要将其现代化。

- [ ] **第 2 步：翻译所有接受对**

保留日期、命令、pass/fail 措辞、限制、跳过的检查和部分完成。翻译不得升级证据主张。

- [ ] **步骤 3：添加对等切换器、验证计数并提交**

```bash
test "$(rg --files docs/en/adr -g '*.md' | wc -l | tr -d ' ')" = 5
test "$(rg --files docs/zh-CN/adr -g '*.md' | wc -l | tr -d ' ')" = 5
test "$(rg --files docs/en/acceptance -g '*.md' | wc -l | tr -d ' ')" = 3
test "$(rg --files docs/zh-CN/acceptance -g '*.md' | wc -l | tr -d ' ')" = 3
git diff --check
git add docs/en/adr docs/zh-CN/adr docs/en/acceptance docs/zh-CN/acceptance
git commit -m "docs: translate decisions and acceptance records"
```

### 任务 4：翻译所有设计规范

**文件：**
- 修改：每个镜像 `docs/{en,zh-CN}/superpowers/specs/*.md`

- [ ] **第 1 步：翻译所有 9 月规格对**

覆盖每一对 `2026-09-*.md`。保留 proposed/approved 状态、边界、架构、UI 标签和显式推迟的工作。

- [ ] **第 2 步：翻译所有 10 月规范对**

覆盖每一对 `2026-10-*.md`。保留团队兼容性、legacy/flow 语义、提供者约束、UI 详细信息以及设计批准和实施证据之间的差异。

- [ ] **步骤 3：翻译所有 11 月规格对**

完整翻译跨提供商交接设计和双语文档设计。保持路径、JSON 字段、命令和验证规则相同。

- [ ] **步骤 4：添加深度正确的切换器并验证奇偶校验**

每个英文规范都链接到 `../../../zh-CN/superpowers/specs/` 下的相同文件名；每个中文规范都链接到 `../../../en/superpowers/specs/` 下的相同文件名。例如，英文 `2026-09-16-real-codex-vertical-slice-design.md` 链接到 `../../../zh-CN/superpowers/specs/2026-09-16-real-codex-vertical-slice-design.md`。

```bash
test "$(rg --files docs/en/superpowers/specs -g '*.md' | wc -l | tr -d ' ')" = "$(rg --files docs/zh-CN/superpowers/specs -g '*.md' | wc -l | tr -d ' ')"
git diff --check
git add docs/en/superpowers/specs docs/zh-CN/superpowers/specs
git commit -m "docs: translate design specifications"
```

### 任务5：翻译所有实施计划

**文件：**
- 修改：每个镜像 `docs/{en,zh-CN}/superpowers/plans/*.md`

- [ ] **第 1 步：翻译所有 9 月计划对**

保留复选框状态、命令、预期结果、提交消息、文件路径、代码块、日期和记录的进度。不要将未经检查的工作标记为已完成。

- [ ] **第 2 步：翻译 10 月 14 日和 15 日团队计划对**

保留兼容性约束、测试、预期故障和进度说明。

- [ ] **第 3 步：将 10 月 28 日转换为 31 个计划对**

保留 UI 选择器、CSS 类名称、SDK/runtime 包名称、提供者路线、测试装置、验收证据和完成资格。

- [ ] **第 4 步：翻译此 11 月迁移计划**

除了切换器和迁移路径之外，保持英语计划完好无损。将每个复选框、命令、预期结果、提交消息和解释翻译成中文对等体。

- [ ] **步骤 5：添加对等切换器、验证奇偶校验并提交**

```bash
test "$(rg --files docs/en/superpowers/plans -g '*.md' | wc -l | tr -d ' ')" = "$(rg --files docs/zh-CN/superpowers/plans -g '*.md' | wc -l | tr -d ' ')"
git diff --check
git add docs/en/superpowers/plans docs/zh-CN/superpowers/plans
git commit -m "docs: translate implementation plans"
```

### 任务 6：添加双语跨提供商切换指南

**文件：**
- 创建：`docs/en/architecture/cross-provider-handoff.md`
- 创建：`docs/zh-CN/architecture/cross-provider-handoff.md`
- 修改：`docs/{en,zh-CN}/README.md`
- 修改：`docs/{en,zh-CN}/architecture/context-and-contracts.md`
- 修改：`docs/{en,zh-CN}/superpowers/specs/2026-10-14-message-flow-teams-design.md`
- 修改：`docs/{en,zh-CN}/runbooks/desktop-s0.md`

- [ ] **第 1 步：根据当前代码编写英文指南**

记录 Archmage 编排边界、JSON/JSON 架构公共语言、交付信封、持久 FlowDelivery、接收方上下文、Coder/Codex → Reviewer/DSH 示例、提供程序适配器差异、conversation/workspace 边界、验证失败、兼容性和代码源。

- [ ] **第2步：编写语义等效的中文指南**

使用相同的标题、JSON 字段名称、示例、约束和源链接。翻译散文和人类可读的价值观而不改变规范含义。

- [ ] **步骤 3：应用批准的一致性修正**

将消息流团队标记为已实施，并指出其实施计划以验证历史记录。将桌面指南的“每次交付都执行真正的 Codex”措辞替换为目标能力提供商中立的措辞。在新指南中添加同语言链接。

- [ ] **步骤 4：验证并提交指南对**

```bash
git diff --check
git add docs/en docs/zh-CN
git commit -m "docs: add bilingual cross-provider handoff guide"
```

### 任务 7：修复存储库范围内的文档链接

**文件：**
- 修改：任何链接到已删除 `docs/<old-path>.md` 的跟踪 Markdown 文件
- 修改：`README.md`、`README.en.md` 以及所需的两个文档索引

- [ ] **第 1 步：查找陈旧的旧布局链接**

```bash
rg -n '\]\(docs/(?!en/|zh-CN/)|\]\((?:\.\./)*docs/(?!en/|zh-CN/)' . -g '*.md' --pcre2
```

- [ ] **步骤 2：通过散文语言路由每个过时的链接**

中文散文链接至 `docs/zh-CN/...`；英文散文链接至 `docs/en/...`。不要在旧路径上创建兼容性存根。

- [ ] **步骤 3：验证不存在任何陈旧链接并提交**

再次运行步骤1搜索；预期输出为空。

```bash
git diff --check
git add -u README.md README.en.md docs
git commit -m "docs: repair bilingual documentation links"
```

### 任务 8：运行最终奇偶校验、切换器、链路和范围验证

**文件：**
- 修改：仅验证发现需要更正的文档文件

- [ ] **第1步：证明镜像奇偶校验**

```bash
(cd docs/en && rg --files -g '*.md' | sort) > /tmp/archmage-en-final.txt
(cd docs/zh-CN && rg --files -g '*.md' | sort) > /tmp/archmage-zh-final.txt
comm -3 /tmp/archmage-en-final.txt /tmp/archmage-zh-final.txt
```

预期：无输出。

- [ ] **第 2 步：证明每个语言切换器都能解析**

对于奇偶校验列表中的每个相对路径，计算其嵌套深度，确认每个第一行包含所需的对等链接，并将该链接解析为现有文件。纠正每个丢失或畸形的切换器。

- [ ] **步骤 3：验证所有本地 Markdown 目标和锚点**

提取每个非 HTTP、非 mailto Markdown 链接。相对于其源文件解析它，确认目标存在，并对于 Markdown 片段确认标准化标题锚点存在。更正每个损坏的链接。

- [ ] **步骤 4：检查语义和结构奇偶性**

比较每对的标题计数、隔离代码计数、表结构、日期、状态标记、命令、路径、JSON 字段和复选框计数。手动审查当前的运行手册、架构合同和跨提供商指南以获取等效声明。

- [ ] **第 5 步：运行格式和范围检查**

```bash
git diff --check origin/main...HEAD
git status --short
git diff --name-only 6554406...HEAD | rg -v '^(README\.md|README\.en\.md|docs/)'
```

预期：格式化成功，worktree在提交后是干净的，并且此迁移不会引入运行时、架构、提供程序或 UI 源更改。

- [ ] **第 6 步：如果需要，提交验证更正**

```bash
git add README.md README.en.md docs/en docs/zh-CN
git commit -m "docs: verify bilingual documentation mirror"
```

当验证不需要更正时，不要创建空提交。
