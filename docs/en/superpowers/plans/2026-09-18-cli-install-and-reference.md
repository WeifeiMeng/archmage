# CLI Installation and Reference Implementation Plan

[简体中文](../../../zh-CN/superpowers/plans/2026-09-18-cli-install-and-reference.md) | English

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Install the current development CLI as a normal `archmage` command and publish a concise, truthful command manual.

**Architecture:** Use Cargo's native path installation rather than adding a custom installer, build hook, alias, or symlink. Keep `docs/cli-reference.md` as the canonical user-facing command reference; README and the broader development guide link to it while retaining deeper contributor material.

**Tech Stack:** Cargo, Rust CLI, Markdown, shell-based acceptance checks

---

## File map

- Create `docs/cli-reference.md`: canonical installation and command manual.
- Modify `README.md`: point first-time users to installation and the command manual.
- Modify `docs/guide.md`: remove stale worktree instructions, use installed-command examples, and delegate command semantics to the canonical manual.
- Modify `docs/README.md`: add the command manual to the recommended reading order.

### Task 1: Create the canonical CLI manual

**Files:**
- Create: `docs/cli-reference.md`

- [ ] **Step 1: Verify the manual is currently absent**

Run:

```bash
test -f docs/cli-reference.md
```

Expected: exit status `1` because the canonical manual does not exist yet.

- [ ] **Step 2: Write the complete manual**

Create `docs/cli-reference.md` with these exact sections and semantics:

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

The actual prose must be complete Chinese documentation, not the outline text above. Every command must be copyable from the repository root.

- [ ] **Step 3: Check required truths are present**

Run:

```bash
rg -n 'cargo install --path crates/archmage-cli --locked --force --root|doctor --offline|真实调用 Codex|尚未实现|退出码 20|cargo uninstall --root' docs/cli-reference.md
```

Expected: every pattern appears at least once.

- [ ] **Step 4: Commit the manual**

```bash
git add docs/cli-reference.md
git commit -m "docs: add canonical CLI command reference"
```

### Task 2: Route users to the canonical manual

**Files:**
- Modify: `README.md`
- Modify: `docs/README.md`
- Modify: `docs/guide.md`

- [ ] **Step 1: Record the stale entry points before editing**

Run:

```bash
rg -n 'current usage guide|进入开发 Worktree|cargo run -p archmage-cli -- doctor' README.md docs/README.md docs/guide.md
```

Expected: README routes users only to the broad guide, and `docs/guide.md` still refers to the deleted `s0-runtime-fencing` worktree and Cargo-run examples.

- [ ] **Step 2: Update the entry points**

Make these focused changes:

- README: add `cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"`, then link `docs/cli-reference.md` as the first CLI reference.
- Documentation index: put `cli-reference.md` before the broader current-version guide.
- Current-version guide: update the date to 2026-09-18, state that content describes `main`, remove the deleted worktree instructions, add the one-time install command, use `archmage doctor` examples, and link the canonical manual. Keep contributor-only test commands and internal architecture details in this guide.

- [ ] **Step 3: Verify links and stale paths**

Run:

```bash
rg -n 'cli-reference.md|cargo install --path crates/archmage-cli --locked --force --root' README.md docs/README.md docs/guide.md
! rg -n 's0-runtime-fencing|\.worktrees/s0-runtime-fencing' README.md docs/README.md docs/guide.md
```

Expected: all three entry documents link or route to the manual, and no deleted worktree path remains.

- [ ] **Step 4: Commit the entry-point updates**

```bash
git add README.md docs/README.md docs/guide.md
git commit -m "docs: route CLI users through installed command"
```

### Task 3: Install and verify the command

**Files:**
- No repository file changes expected.

- [ ] **Step 1: Run repository quality checks**

Run:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

Expected: every command exits `0`; real Codex tests remain explicitly ignored by the normal test suite.

- [ ] **Step 2: Install the current binary**

Run:

```bash
cargo install --path crates/archmage-cli --locked --force --root "$HOME/.local"
```

Expected: Cargo reports installation of executable `archmage` from package `archmage-cli v0.1.0` at `$HOME/.local/bin/archmage`.

- [ ] **Step 3: Verify command resolution and basic behavior**

Run:

```bash
command -v archmage
archmage --version
archmage --help
```

Expected: `command -v` resolves `$HOME/.local/bin/archmage`, version is `archmage 0.1.0`, and help lists `doctor`, `run`, `status`, `logs`, `resume`, and `cancel`.

- [ ] **Step 4: Verify offline Doctor without using the normal data directory**

Run:

```bash
ARCHMAGE_HOME="$(mktemp -d)/data" archmage doctor --offline
```

Expected: exit `0`, `authentication` is `not_verified_offline`, and database `schema_version` is `5`.

- [ ] **Step 5: Verify unavailable Workflow behavior remains honest**

Run:

```bash
archmage run --repo "$PWD" --goal "CLI documentation verification" --check "cargo test"
```

Expected: exit `20` and `workflow command is not implemented in this batch`.

- [ ] **Step 6: Record final repository state**

Run:

```bash
git status --short
git log -3 --oneline
```

Expected: clean worktree containing the design, plan, manual, and entry-point commits.
