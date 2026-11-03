简体中文 | [English](../../../en/superpowers/plans/2026-09-28-one-command-desktop-dev.md)

# 一命令桌面开发实施方案


> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 添加一个可执行存储库根脚本，用于检查先决条件、安装缺少的前端依赖项并启动完整的 Tauri 桌面开发应用程序。

**架构：** 集中的 Bash 入口点从其自己的路径解析存储库根，并将生命周期管理委托给现有的 Tauri CLI。Tauri 仍然负责推出 Vite 和嵌入式 Rust Runtime；没有引入后端守护进程或重复的前端进程。

**技术堆栈：** Bash、npm、Vite、Tauri 2、Rust/Cargo。

---

### 任务 1：添加单命令启动器

**文件：**
- 创建：`scripts/dev-desktop.sh`

- [ ] **第 1 步：添加启动器**

创建一个可执行的严格模式 Bash 脚本，用于解析存储库根目录，检查 `git`、`cargo`、`node`、`npm` 和 `codex`，仅在 `apps/archmage-desktop/node_modules` 不存在时安装依赖项，然后使用 `exec`对于现有的 Tauri 命令：

```bash
#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"
desktop_dir="$repo_root/apps/archmage-desktop"

for executable in git cargo node npm codex; do
  if ! command -v "$executable" >/dev/null 2>&1; then
    printf 'Archmage desktop requires %s on PATH.\n' "$executable" >&2
    exit 1
  fi
done

if [[ ! -d "$desktop_dir/node_modules" ]]; then
  npm install --prefix "$desktop_dir"
fi

cd "$repo_root"
exec npm run tauri dev --prefix "$desktop_dir"
```

- [ ] **第 2 步：使其可执行并验证语法**

运行：

```bash
chmod +x scripts/dev-desktop.sh
bash -n scripts/dev-desktop.sh
```

预期：退出代码 0，无输出。

- [ ] **第 3 步：验证缺失的先决条件是否明确**

创建一个临时 PATH，其中包含除 `codex` 之外的所有必需可执行文件的链接，然后使用该 PATH 调用脚本。

预期：退出代码 1 且 stderr 包含 `Archmage desktop requires codex on PATH.` Tauri 命令不得启动。

### 任务 2：记录推荐的切入点

**文件：**
- 修改：`README.md`
- 修改：`docs/runbooks/desktop-s0.md`

- [ ] **第 1 步：更新自述文件**

将此作为主要桌面启动示例：

```bash
./scripts/dev-desktop.sh
```

声明它同时启动 Vite UI 和嵌入式 Rust 运行时。在故障排除小节中保留两个基础 npm 命令。

- [ ] **步骤 2：更新桌面 Runbook**

使用相同的推荐命令，解释首次运行依赖项 installation/compilation，保留 `ARCHMAGE_HOME` 传递，并保留手动 npm 命令进行诊断。

- [ ] **第 3 步：检查文档和差异格式**

运行：

```bash
rg -n "dev-desktop|tauri dev" README.md docs/runbooks/desktop-s0.md
git diff --check
```

预期：两份文件都提到单命令启动器和手动 Tauri 命令；`git diff --check` 退出 0。

### 任务 3：冒烟测试并提交

**文件：**
- 验证：`scripts/dev-desktop.sh`
- 验证：`README.md`
- 验证：`docs/runbooks/desktop-s0.md`

- [ ] **第 1 步：运行有界发射冒烟测试**

运行 `./scripts/dev-desktop.sh`，观察 Vite 准备就绪，Tauri 启动 Rust build/application，然后终止开发过程。不要创建运行或调用模型。

预期：输出包含 Vite 本地 URL 和 Tauri `Running` 行；终止不会留下任何启动器拥有的进程运行。

- [ ] **第 2 步：提交实施**

```bash
git add scripts/dev-desktop.sh README.md docs/runbooks/desktop-s0.md
git commit -m "feat: add one-command desktop startup"
```
