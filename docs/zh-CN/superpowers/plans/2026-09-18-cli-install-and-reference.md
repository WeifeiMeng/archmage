简体中文 | [English](../../../en/superpowers/plans/2026-09-18-cli-install-and-reference.md)

# CLI安装及参考实施方案


> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 将当前的开发 CLI 安装为普通的 `archmage` 命令，并发布简洁、真实的命令手册。

**架构：** 使用 Cargo 的本机路径安装，而不是添加自定义安装程序、构建挂钩、别名或符号链接。保留 `docs/cli-reference.md` 作为规范的面向用户的命令参考；自述文件和更广泛的开发指南链接到它，同时保留更深入的贡献者材料。

**技术堆栈：** Cargo、Rust CLI、Markdown、基于 shell 的验收检查

---

## 文件图

- 创建 `docs/cli-reference.md`：规范安装和命令手册。
- 修改 `README.md`：将首次用户指向安装和命令手册。
- 修改 `docs/guide.md`：删除过时的worktree指令，使用已安装的命令示例，并将命令语义委托给规范手册。
- 修改`docs/README.md`：将命令手册添加到推荐阅读顺序中。

### 任务 1：创建规范的 CLI 手册

**文件：**
- 创建：`docs/cli-reference.md`

- [ ] **第 1 步：验证当前是否缺少手册**

运行：

```bash
test -f docs/cli-reference.md
```

预期：退出状态 `1`，因为规范手册尚不存在。

- [ ] **第2步：编写完整的手册**

使用这些确切的部分和语义创建 `docs/cli-reference.md`：

```markdown
# Archmage CLI 命令手册

## 当前状态

可用：`--help`、`--version`、`doctor --offline`、`doctor`。
尚未实现：`run`、`status`、`logs`、`resume`、`cancel`；调用时返回退出码 20。

## 安装

在仓库根目录执行：

    cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"

验证 `command -v archmage`、`archmage --version` 和 `archmage --help`。
说明 `cargo build -p archmage-cli` 只生成 `target/debug/archmage`，不会安装全局命令。

## 更新与卸载

更新使用同一条带 `--force` 的安装命令；卸载使用 `cargo uninstall --root "$HOME/.local" archmage-cli`。

## 数据目录

说明平台默认目录、`ARCHMAGE_HOME` 覆盖方式、数据库和 probe 诊断文件，以及不得公开原始诊断日志。

## 环境检查

分别解释 `archmage doctor --offline` 与 `archmage doctor`。在线命令会真实调用 Codex、使用 read-only sandbox、可能消耗额度；Doctor 只证明最小环境与连接，不证明完整 Workflow 可运行。

## 命令参考

列出当前可用命令的语法、输出字段和示例。把未实现 Workflow 命令放进独立警告区，只用于展示未来接口形态。

## 退出码

记录当前可确认的 0、2、20，以及规划中的 10、21、22，明确区分当前实现和预留语义。

## 常见问题

覆盖 command not found、Codex 不在 PATH、离线成功在线失败、Workflow 未实现和敏感诊断文件处理。
```

实际的散文必须是完整的中文文档，而不是上面的提纲文本。每个命令都必须可以从存储库根目录复制。

- [ ] **第 3 步：检查所需的事实是否存在**

运行：

```bash
rg -n 'cargo install --path crates/archmage-cli --locked --force --root|doctor --offline|真实调用 Codex|尚未实现|退出码 20|cargo uninstall --root' docs/cli-reference.md
```

预期：每个模式至少出现一次。

- [ ] **第 4 步：提交手册**

```bash
git add docs/cli-reference.md
git commit -m "docs: add canonical CLI command reference"
```

### 任务 2：将用户引导至规范手册

**文件：**
- 修改：`README.md`
- 修改：`docs/README.md`
- 修改：`docs/guide.md`

- [ ] **步骤 1：编辑前记录过时的入口点**

运行：

```bash
rg -n 'current usage guide|进入开发 Worktree|cargo run -p archmage-cli -- doctor' README.md docs/README.md docs/guide.md
```

预期：README 仅将用户引导至广泛指南，并且 `docs/guide.md` 仍指已删除的 `s0-runtime-fencing` worktree和 Cargo 运行示例。

- [ ] **第 2 步：更新入口点**

做出这些有针对性的改变：

- 自述文件：添加 `cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"`，然后将 `docs/cli-reference.md` 链接为第一个 CLI 参考。
- 文档索引：将 `cli-reference.md` 放在更广泛的当前版本指南之前。
- 当前版本指南：将日期更新为 2026-09-18，声明内容描述 `main`，删除已删除的worktree指令，添加一次性安装命令，使用 `archmage doctor` 示例，并链接规范手册。在本指南中保留仅限贡献者的测试命令和内部架构详细信息。

- [ ] **步骤 3：验证链接和过时路径**

运行：

```bash
rg -n 'cli-reference.md|cargo install --path crates/archmage-cli --locked --force --root' README.md docs/README.md docs/guide.md
! rg -n 's0-runtime-fencing|\.worktrees/s0-runtime-fencing' README.md docs/README.md docs/guide.md
```

预期：所有三个条目文档都链接或路由到手册，并且不保留已删除的worktree路径。

- [ ] **第 4 步：提交入口点更新**

```bash
git add README.md docs/README.md docs/guide.md
git commit -m "docs: route CLI users through installed command"
```

### 任务 3：安装并验证命令

**文件：**
- 预计不会发生存储库文件更改。

- [ ] **第 1 步：运行存储库质量检查**

运行：

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

预期：每个命令退出 `0`；真正的 Codex 测试仍然被普通测试套件明确忽略。

- [ ] **步骤 2：安装当前的二进制文件**

运行：

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
```

预期：Cargo 报告安装了来自 `archmage-cli v0.1.0` 包 `$HOME/.local/bin/archmage` 的可执行 `archmage`。

- [ ] **步骤 3：验证命令解析和基本行为**

运行：

```bash
command -v archmage
archmage --version
archmage --help
```

预期：`command -v` 解析 `$HOME/.local/bin/archmage`，版本为 `archmage 0.1.0`，帮助列表 `doctor`、`run`、`status`、`logs`、`resume` 和`cancel`。

- [ ] **步骤4：不使用普通数据目录验证离线Doctor**

运行：

```bash
ARCHMAGE_HOME="$(mktemp -d)/data" archmage doctor --offline
```

预期：退出 `0`，`authentication` 为 `not_verified_offline`，数据库 `schema_version` 为 `5`。

- [ ] **步骤 5：验证不可用的工作流行为是否保持诚实**

运行：

```bash
archmage run --repo "$PWD" --goal "CLI documentation verification" --check "cargo test"
```

预期：退出 `20` 和 `workflow command is not implemented in this batch`。

- [ ] **第 6 步：记录最终存储库状态**

运行：

```bash
git status --short
git log -3 --oneline
```

预期：包含设计、计划、手册和入口点提交的干净worktree。
