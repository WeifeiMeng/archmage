简体中文 | [English](../../../en/roadmap/stages/s0-real-codex-slice.md)

# S0：真实 Codex 垂直切片

**状态：** 已批准进入实施规划

**实施计划：** [S0 分步计划](../../superpowers/plans/2026-09-16-s0-real-codex-vertical-slice.md)。规划已完成；产品实施与真实 Codex 验收尚未开始。

## 目标

连接真实的本地 Codex CLI Provider，在隔离 Git worktree 中通过固定 Workflow 协调四个独立 Capability Agent，从而验证 Archmage 协作主干。

## 用户可见流程

```bash
archmage doctor
archmage run \
  --repo /path/to/repository \
  --goal "Implement a health check endpoint" \
  --check "<project-specific validation command>"
```

命令会流式输出 Timeline，并在结束时给出 Run ID、结果、worktree 路径、branch 和 Artifact 目录。Workflow 为：

```text
Designer → Coder → Tester → Reviewer → Completed
```

每个节点调用一个新的真实 `codex exec` 会话。产品代码路径不会用 Fake Agent 替代。

## 范围内

### Runtime 基础

- 包含 UI 无关 Core 与轻量 CLI 的 Rust workspace
- Project、Provider、Capability、Run、Node Run、Agent Run、Workspace、Context Snapshot、Artifact 与 Event 的领域标识和持久状态
- SQLite 迁移、WAL、事务化状态与 Event 写入
- 支持并发与 workspace lease 兼容性的 Ready Set Scheduler
- 显式取消、故障分类、重启扫描与恢复

### 真实 Codex Provider

- 检测 Codex executable 并记录已测试版本
- 通过安全 Provider preflight 验证认证可用性
- 使用显式工作目录与 sandbox 调用 `codex exec`
- 通过 stdin 发送完整渲染 prompt
- 捕获 JSONL Event 与 Schema 约束的最终结果
- 记录 Codex thread ID 用于可观测性，但 Handoff 不使用隐藏会话历史

### Capability Agent

- Designer：只读仓库分析与设计 Artifact
- Coder：workspace-write 实现与 ImplementationResult
- Tester：通过 Archmage 管理的必需检查命令执行 workspace-write 验证并输出 TestResult
- Reviewer：只读评审目标、设计、diff 与测试证据

### 上下文与契约

- 带版本的内置指令与 JSON Schema
- 类型化 Context Reference 与不可变 Context Snapshot
- 机器 JSON 与提取的人类可读 Markdown
- Runtime 二次校验与 Capability 特定不变量

### Workspace

- 源仓库校验与持久化基线 `HEAD`
- 每个 Run 使用 `archmage/run-<id>` branch 与外部 worktree
- 对 dirty 源仓库发出警告，但不复制未提交变更
- 相对基线生成 diff 与 status Artifact
- 不自动 commit、merge、push、移除 worktree 或删除 branch

### CLI

- `doctor`
- `run`
- `status`
- `logs`
- `resume`
- `cancel`

## 范围外

- Fake Agent 或模拟 Workflow 模式
- 自动 Review-to-Fix 循环
- 用户自定义 Workflow 与 Capability 定义
- 其他 Provider
- 桌面应用
- 后台 daemon 与自动恢复
- 自动 commit、merge、push 或清理
- 云服务、多用户能力、Memory、RAG 与市场

## 功能切片

1. **Bootstrap 与诊断：** 建立 Rust workspace、数据根目录、CLI shell 和 Codex/Git preflight。
2. **持久化与领域：** 创建迁移、状态机、Event 事务与恢复查询。
3. **Workspace 隔离：** 创建真实 branch/worktree，并强制基线与 lease 不变量。
4. **Codex 协议：** 调用本地 CLI、解析 JSONL、捕获输出、安全取消并分类故障。
5. **Capability 契约：** 定义四个角色、Schema、prompt、Artifact 与 Context 绑定。
6. **Scheduler 与编排：** 推导 Ready Set、分派节点、提交转换并构建 Handoff。
7. **恢复与可观测性：** 支持 status、logs、中断检测、resume 与最终报告。
8. **真实验收：** 使用已认证 Codex 针对受控仓库运行并中断完整 Workflow。

## 退出条件

只有全部满足以下条件，S0 才算完成：

1. `archmage doctor` 确认 Git、Codex CLI、认证可用性及本地存储可写。
2. 真实 Run 启动四个独立 Codex 会话，无需人工复制粘贴即可完成。
3. Coder 只修改隔离 worktree；源 worktree 与 remote 不变。
4. 每个 Node Run 都有 Context Snapshot、prompt、Codex JSONL 流、已校验结果、摘要与有序 Event。
5. Tester 证据来自 Archmage 管理的 Check Runner，包含每条必需校验命令，且任何必需检查失败时不能报告 `passed`。
6. Reviewer 通过 Context 绑定消费持久化设计、diff 与 TestResult，而非隐藏 Codex 历史；`approved` 与 `changes_requested` 仍是不同完成结果。
7. 节点执行期间终止 Runtime 后，重启会产生 `interrupted` Attempt；`resume` 创建新 Attempt 并完成 Workflow。
8. 自动化领域、持久化、Workspace、契约与协议解析测试通过。
9. [S0 操作手册](../../runbooks/s0-acceptance.md)中的真实 Codex 验收 Run 通过。

## 产生的证据

- CLI 记录与退出状态
- SQLite Run、Node Run、Agent Run、Artifact、Context 与 Event 记录
- 四个不同的 Codex thread ID
- Design、implementation、test、review、diff 与最终报告 Artifact
- 展示 interrupted 与 replacement Attempt 的恢复证据
- 源 worktree 与 Run worktree 的 Git status 快照

## 下一阶段触发条件

只有 S0 证明显式 Artifact 与 Context Snapshot 足以支持真实 Capability Agent 协作后，S1 才开始。S1 会泛化定义，并增加有界 Review/Fix 循环。
