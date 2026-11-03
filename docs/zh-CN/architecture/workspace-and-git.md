简体中文 | [English](../../en/architecture/workspace-and-git.md)

# 工作区与 Git

## Run 基线

创建 Run 时，Archmage 解析源仓库规范路径与当前 `HEAD`，该 commit 成为 Run 的不可变基线。

源 worktree 中未提交的变更不会复制进 Run 工作区。系统会把这些变更的存在记录为警告，让用户明确 Run 从 `HEAD` 而不是 dirty working tree 开始。

## Worktree 生命周期

每个 Workflow Run 获得：

```text
branch:    archmage/run-<run-id>
worktree:  <archmage-data>/workspaces/<run-id>
baseline:  <source-repository HEAD at creation>
```

创建操作相对于持久化 Workspace 记录是幂等的。如果 branch 或目录发生冲突，除非能证明现有资源属于同一个 Run，否则操作会失败关闭。

S0 在成功、失败或取消后都会保留 branch 与 worktree 并输出其位置。它绝不会自动 commit、merge、push、移除 worktree 或删除 branch。

## 访问模式与 lease

每个节点声明 `none`、`read` 或 `write` 访问权限。Scheduler 向写节点授予独占 workspace lease。读节点使用冻结的 Context Snapshot，不能与同一 worktree 上的存活写节点重叠执行。

S0 映射如下：

| Capability Agent | 访问 | Codex sandbox |
|---|---|---|
| Designer | `read` | `read-only` |
| Coder | `write` | `workspace-write` |
| Tester | `write` | `workspace-write` |
| Reviewer | `read` | `read-only` |

Tester 被视为写节点，因为构建与测试通常会创建本地文件，即使源码本身不应变化。测试后的 diff 不变量会记录并分类验证期间产生的源码变更。

## 安全不变量

- Provider 进程以 worktree 为工作根目录。
- S0 从不使用 `danger-full-access`。
- 源仓库当前 checkout 的 branch 不会变化。
- Capability 指令禁止 Git commit，且整个 S0 期间 Run worktree 的 `HEAD` 必须等于已持久化基线。
- Diff Artifact 相对于已持久化基线计算。
- Runtime 在每个可写节点执行前记录 branch、worktree 路径、基线和当前 commit。
- 凭证通过 Provider 支持的认证机制继承，绝不会复制进 prompt、Event 或 Artifact。
