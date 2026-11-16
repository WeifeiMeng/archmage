# Archmage 主机侧分支管理与 Flow 阻塞终态设计

## 背景

Archmage 会在 Run 启动时创建独立 Git worktree 和 `archmage/run-<run_id>` 分支，但 Coder 仍可能根据用户目标尝试创建、切换或重命名分支。Provider 只拥有 Run worktree 的 `workspace-write` 权限，而 worktree 的 Git 元数据位于源仓库 `.git/worktrees/...`，因此底层 Agent 不应也不能可靠地修改分支引用。

当 Coder 因此返回 `status: blocked` 时，现有 Flow 契约只允许 `continue` 或 `complete`，并显式拒绝 blocked Coder 路由。有效的 Provider 结果最终被折算成 `preflight_failed`，结构化阻塞原因和已完成改动无法作为正式节点结果展示。

## 目标

1. 分支生命周期由 Archmage 主机侧编排层负责，Provider 只修改 Run worktree 内的文件。
2. Run 可选指定目标分支；未指定时保持当前自动分支行为。
3. Coder 和 Tester 不再因 Git 分支元数据权限而误判任务阻塞。
4. `blocked` 成为可持久化、可查询、可展示的 Flow 终态，而不是通用预检失败。
5. 阻塞时保留有效结构化结果、Result Artifact、Coder ChangeSet 和工作树改动。
6. 不扩大 Codex、Qoder 或 DeepSeek Harness 对源仓库 `.git` 的写权限。

## 非目标

- 不从自然语言目标中猜测分支名称。
- 不允许 Provider 自行创建、切换、重命名或删除 Git 分支。
- 不在本次设计中实现 blocked Run 的原地恢复或从指定节点继续执行。
- 不改变现有自动 Run 分支的清理策略。

## 方案选择

### 采用：主机侧可选目标分支 + 明确 Provider 边界

RunRequest 增加可选 `targetBranch`。Archmage 在创建 worktree 时校验并创建该分支；没有提供时仍使用 `archmage/run-<run_id>`。实际分支名写入 Workspace 记录并注入 Capability 上下文。

该方案保持 Git 元数据操作位于可信主机进程中，既支持明确的 topic branch，也兼容当前自动隔离分支。

### 不采用：扩大 Provider 沙箱写目录

将源仓库 `.git/worktrees/...` 加入 Provider 可写范围，会让 Agent 获得修改其他分支引用、worktree 注册和锁文件的能力，破坏 Run 隔离边界。

### 不采用：解析自然语言并自动提取分支名

自然语言中的分支意图可能不完整或歧义。分支名必须作为结构化输入进入 RunRequest；“新建分支”但没有给出名称时，由现有自动 Run 分支满足隔离要求。

## Run API 与桌面端

### RunRequest

增加可选字段：

```text
target_branch: Option<String>  // Rust
targetBranch?: string          // TypeScript / Tauri JSON
```

创建 Run 时对非空值执行：

1. 去除首尾空白；空字符串等价于未指定。
2. 使用 `git check-ref-format --branch` 校验。
3. 拒绝已存在的本地分支，禁止隐式复用或覆盖。
4. 将目标分支冻结进 Run definition，保证恢复和重启不受后续 UI 状态影响。

桌面端“新建 Run”表单增加“目标分支（可选）”输入框，并说明留空会创建 Archmage 自动分支。表单只做即时格式提示；Runtime 仍是最终校验边界。

### Workspace provisioning

`workspace::provision` 接收冻结后的可选目标分支：

- 指定：`git worktree add -b <target_branch> <path> <baseline>`
- 未指定：沿用 `archmage/run-<run_id>`

在任何文件工作或 Provider 启动前完成分支创建。继续沿用现有 provisioning intent 和 reconcile 机制；数据库记录最终实际分支，而不是重新推导。

若分支校验、冲突或 worktree 创建失败，Run 在 provisioning 阶段以 `workspace_failed` 结束，不启动 Coder。

## Provider 上下文边界

每次执行 Capability 时，从已持久化 Workspace 记录读取实际分支，并向上下文加入明确约束：

```text
Archmage 已创建并检出隔离分支 <branch>。
不要创建、切换、重命名或删除 Git 分支，也不要修改 worktree Git 元数据。
用户要求“新建分支”时，分支准备已由 Archmage 编排层完成；只需在当前分支完成交付。
```

该约束属于平台执行上下文，不依赖用户自定义 Capability 指令，并同时适用于 Codex、Qoder 和 DeepSeek Harness。

## Flow blocked 终态

### 契约

Flow routing 增加：

```json
{
  "decision": "blocked",
  "next": []
}
```

校验规则：

- `deliverable.status == "blocked"` 时，routing 必须为 `blocked` 且 `next` 为空。
- routing 为 `blocked` 时，deliverable 必须具有对应 Capability 已有 schema 中的 blocked 状态。
- `blocked` 不允许继续投递到 Reviewer 或其他下游。
- 非 blocked 输出不得使用 blocked routing。
- Designer 和 Reviewer 当前 schema 没有 blocked 状态，因此不能产生 blocked routing。

### 持久化语义

Provider 调用和结构化输出本身是成功的，因此 Attempt 记为 `succeeded`，并正常持久化结果：

- Result Artifact
- Coder ChangeSet Artifact
- Conversation ID
- 结构化 deliverable
- routing decision

业务执行状态记录为：

- Node：`blocked`
- Flow delivery：`blocked`
- Run：`blocked`

新增 Runtime 事件：

- `flow.delivery_blocked`
- `node.blocked`
- `workflow.blocked`

Run outcome 固定使用 `<capability>_blocked`，当前合法值为 `coder_blocked` 和 `tester_blocked`。具体说明继续保存在结构化结果中，避免把自由文本复制到顶层事件。

### 数据库迁移

新增下一版本迁移，扩展以下 CHECK 状态集合：

- `runs.state` 增加 `blocked`
- `node_runs.state` 增加 `blocked`
- `flow_deliveries.state` 增加 `blocked`

Attempts 不增加 blocked：它表示 Provider 调用及结果持久化是否成功；合法 blocked deliverable 对应成功 Attempt。

迁移必须保留现有行、外键、索引和历史状态，并覆盖从当前最新 schema 升级的测试。

## Runtime 控制流

`validate_flow_output` 返回合法 blocked routing 后：

1. 发布 Result Artifact 和可选 ChangeSet。
2. 将 Attempt 完成标记为 succeeded。
3. 将当前 Delivery、Node、Run 原子地置为 blocked。
4. 将其他尚未执行的 queued Delivery 按现有终止清理规则处理，不再启动新 Provider。
5. `run_flow_to_terminal` 将 blocked 视为正常的终态快照返回，而不是抛出 `Error::Preflight`。

真正的 schema 错误、Provider 失败、存储错误仍走现有 failed 路径。

## UI 表现

- Run 列表：显示“已阻塞”，与“失败”区分。
- 节点检查器：显示“Coder 已阻塞”或“Tester 已阻塞”。
- Flow delivery：显示“阻塞”，保留结构化结果、Artifact、ChangeSet 和 Agent 输出。
- 阻塞原因从节点结构化结果的 summary/notes 展示；不要求用户打开私有 diagnostics。
- 运行时间线显示 `workflow.blocked`，不再只显示 `preflight_failed`。

## 兼容性与恢复

- `targetBranch` 缺省时与旧客户端、旧 Run 行为一致。
- FrozenRunDefinition 新字段使用 serde default，历史 definition 可正常读取。
- 历史 failed Run 不回填或改写为 blocked。
- 恢复 provisioning 中的 Run 时，使用已持久化 Workspace branch，不重新读取请求字段。
- blocked Run 本次只读展示，不能调用现有 resume；后续可独立设计恢复能力。

## 测试策略

### Rust

- RunRequest/FrozenRunDefinition：目标分支冻结、缺省兼容、非法名称拒绝。
- Workspace：指定分支创建、默认分支、冲突失败、reconcile 使用持久化分支。
- Contract：blocked status 与 routing 的双向一致性；blocked 不允许 recipients。
- Persistence migration：当前 schema 升级后保留数据并接受 blocked 状态。
- Flow integration：Coder blocked 后 Attempt succeeded，Result/ChangeSet 可查询，Node/Delivery/Run blocked，Reviewer 不启动。
- 非 blocked 成功、Provider 失败和历史 Flow 测试保持通过。

### Desktop

- RunRequest 序列化目标分支，空值省略。
- 新建 Run 表单格式、提示和错误展示。
- Run、Node、Delivery 的 blocked 中文状态。
- blocked 节点仍展示结构化结果和 Artifact。

### 验收场景

1. 留空目标分支启动 Run：创建 `archmage/run-<id>`，Coder 不操作分支。
2. 指定合法目标分支：主机侧创建并检出，Coder 在该分支工作。
3. 指定已存在或非法分支：在 Provider 启动前失败并给出明确错误。
4. Coder 因真实外部依赖阻塞：Run 显示 blocked，保留结果和改动，Reviewer 不运行。
5. 任务文本包含“新建分支”但未指定结构化名称：自动 Run 分支满足要求，不再因 Git 元数据权限失败。

## 安全性

- Provider 沙箱策略不变。
- 目标分支只作为单个 Git 参数传递，不经 shell 拼接。
- 分支名通过 Git 原生校验并拒绝覆盖。
- 私有 diagnostics、环境变量和凭据不进入 blocked 顶层事件。
