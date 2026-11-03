简体中文 | [English](../../../en/superpowers/specs/2026-11-03-bilingual-documentation-system-design.md)

# 双语文献系统设计


**状态：** 批准实施规划。

## 目的

为Archmage打造简体中文和英文一流、完整的文档语言。当前 `docs/` 下的每个 Markdown 文档必须有一个英文版本和一个简体中文版本，并具有匹配的路径、结构、示例、约束和源参考。

存储库登陆页面仍然是中文优先。英语读者会在根自述文件的顶部收到一个明确的条目。

## 目标布局

```text
README.md                 # Chinese project landing page
README.en.md              # English project landing page

docs/
├── en/
│   ├── README.md
│   ├── acceptance/
│   ├── adr/
│   ├── architecture/
│   ├── concepts/
│   ├── product/
│   ├── roadmap/
│   ├── runbooks/
│   └── superpowers/
└── zh-CN/
    ├── README.md
    └── the same relative paths as docs/en/
```

迁移后，包含语言的 Markdown 文件不得直接保留在 `docs/` 下。两个语言树是完全镜像：对于每个 `docs/en/<relative-path>.md`，恰好存在一个 `docs/zh-CN/<relative-path>.md`，反之亦然。

## 根自述文件行为

`README.md` 成为完整的简体中文项目登陆页面，并以 `README.en.md` 的可见链接开始。`README.en.md` 保留完整的英文项目登陆页面，并以 `README.md` 的相互链接开始。

两个页面都描述了相同的当前产品行为并链接到其匹配的文档索引：

- `README.md` → `docs/zh-CN/README.md`;
- `README.en.md` → `docs/en/README.md`。

根 README 对位于 `docs/` 镜像之外，但遵循相同的语义奇偶校验规则。

## 迁移和翻译规则

### 货源选择

对于现有英文文档，将原始文档移至 `docs/en/` 中，并在 `docs/zh-CN/` 下创建忠实的简体中文翻译。对于现有的中文文档，将原始文档移至 `docs/zh-CN/` 中，并在 `docs/en/` 下创建忠实的英文翻译。

混合语言文档必须根据其主导散文语言进行规范化，然后进行完整翻译。必须保留现有的文档历史记录、状态措辞、日期、资格和已知限制；翻译不得将设计转变为实现声明或将部分验证转变为已完成的验收声明。

### 内容保持不变

除非示例明确代表人类创作的散文，否则请在两种语言中逐字保留这些值：

- shell命令和命令输出；
- 代码和配置片段；
- JSON 字段名称和枚举值；
- 数据库列、事件名称、API 名称、CLI 标志、环境变量、文件路径、标识符和错误代码；
- Markdown 链接目标，除非迁移需要新的相对路径；
- 产品名称包括 Archmage、Codex、DeepSeek Harness、MCP 和 ACP。

翻译标题、解释、表格散文、标注、人类可读的示例值和散文注释，这样做不会改变可执行的含义。

### 语言切换器

每个镜像文档都以标题前的紧凑语言切换器开头。当前语言是纯文本，对等语言是链接：

```markdown
# In an English document
[简体中文](<relative path to zh-CN peer>) | English

# In a Simplified Chinese document
简体中文 | [English](<relative path to en peer>)
```

相对路径必须考虑文档的嵌套深度。

### 内部链接

文档页面之间的链接保留在读者选择的语言树中。由于两棵树都使用相同的相对路径，因此大多数文档内链接在初始移动后可以保持结构相同。

从文档到存储库文件的链接必须获得 `docs/en/` 或 `docs/zh-CN/` 引入的附加父目录级别。必须更新从源文件、根自述文件和其他存储库内容到文档的链接，以定位正确的语言路径。

每个旧 `docs/<path>.md` 位置都不需要兼容性存根。存储库拥有的链接必须在同一更改中迁移，以便最终的树不会有意引用已删除的路径。

## 覆盖范围

覆盖范围包括当前 `docs/` 下的每个 Markdown 文档，包括：

- 文档索引和当前用户指南；
- 产品、概念、架构、路线图和运行手册文档；
- ADR 和验收记录；
- 历史 `superpowers/specs` 和 `superpowers/plans` 文档。

历史材料仍然是历史的。它的翻译必须保留原始状态、时间顺序、范围和未经检查的清单状态，而不是将其重写为当前指南。

先前批准的跨提供商切换指南在新的镜像架构目录中实现：

- `docs/en/architecture/cross-provider-handoff.md`；
- `docs/zh-CN/architecture/cross-provider-handoff.md`。

`2026-11-03-cross-provider-handoff-guide-design.md` 中的要求仍然适用于这两个文件。

## 文档索引

`docs/en/README.md` 和 `docs/zh-CN/README.md` 是等效的导航索引。每个索引必须：

- 解释文档层；
- 清楚地识别当前和历史材料；
- 仅链接到其自己的语言树中的页面，语言切换器除外；
- 暴露相同的推荐阅读顺序；
- 在架构部分包含跨提供商切换指南。

## 交付策略

以可审查的批次实施迁移，同时保持最终结果完整：

1. **布局和登陆页面** — 创建两种语言树，将现有文件移动到其源语言端，创建 `README.en.md`，并更新根登陆页面链接。
2. **当前产品文档** — 完成产品、概念、架构、路线图、操作手册、CLI/current 指南和文档索引的两种语言。
3. **历史和治理文档** — 填写 ADR、验收记录以及所有 `superpowers/specs` 和 `superpowers/plans` 的两种语言。
4. **跨提供商指南和存储库范围的链接修复** - 添加两个指南版本，修复所有剩余的存储库引用，并运行完整的奇偶校验和链接验证。

中间提交可能包含不完整的翻译覆盖范围，但每个提交必须准确说明其范围。只有在最终镜像和链接检查通过后才能声明完成。

## 一致性政策

英文和简体中文文档是同等支持的工件。影响行为、约束、命令、状态或示例的后续文档更改必须在同一更改中更新两个语言版本。

如果翻译措辞发生冲突，则当前的实现和权威模式决定行为。这两种语言都不允许默默地引入更严格或更宽松的协议。

## 验证

最终验证必须包括：

1. **镜像奇偶校验** — 比较 `docs/en/` 和 `docs/zh-CN/` 下的相对 `.md` 路径；两组必须相同。
2. **语言切换器** — 确保每个镜像文档都链接到其对等体并且链接可以解析。
3. **相对链接** — 从两棵树中提取本地 Markdown 链接，确认每个目标文件存在，并验证 Markdown 目标的锚点。
4. **存储库引用** — 在 `docs/en/` 和 `docs/zh-CN/` 外部搜索已删除 `docs/<old-path>` 位置的过时链接。
5. **语义采样** — 比较两种语言的标题、代码块、表格、状态语句、日期、警告和示例，特别注意当前的操作手册和架构合同。
6. **格式化** — 运行 `git diff --check` 并扫描占位符，例如 `TODO`、`TBD` 或迁移引入的未翻译的临时标记。
7. **范围** — 确认在文档迁移过程中没有运行时、架构、提供程序或 UI 行为发生更改。
