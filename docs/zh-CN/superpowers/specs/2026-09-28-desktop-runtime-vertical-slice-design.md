# 桌面运行时垂直切片设计

简体中文 | [English](../../../en/superpowers/specs/2026-09-28-desktop-runtime-vertical-slice-design.md)

**状态：** 批准实施规划

## 目标

交付第一个真正可运行的 Archmage 产品切片：Tauri 桌面工作台，选择本地 Git 存储库，接受目标和所需的检查，通过真正的本地 Codex CLI 运行固定的设计者→编码者→测试者→审阅者工作流程，并呈现持久的进度和结果，而无需用户操作 CLI。

## 适用范围

该切片包括：

- 一个真正的 Codex 提供者由四个逻辑功能角色共享；
- 一次运行包含所有四个节点运行，而不是每个节点一次运行；
- S0 的固定线性工作流程；
- 真正的每次运行 Git worktree隔离；
- 明确的上下文快照、交接、合同、工件、检查证据和事件；
- 使用 React、TypeScript 和 Vite 的 Tauri 2 桌面应用程序；
- 存储库选择、goal/check 输入、环境预检、运行启动、实时时间线、节点检查、取消、历史运行检查和工作区显示；
- 应用程序重新启动时的中断协调。

该切片不包括后台守护程序、系统托盘、工作流编辑器、提供程序设置 UI、修复循环、自动重试、自动 commit/merge/push、签名打包或多提供程序类型。

## 运行时架构

`archmage-core` 仍然独立于 Tauri 和 CLI。它公开了一个面向应用程序的小型运行时外观：

```text
RunRequest
  repo: absolute path
  goal: nonblank string
  checks: one or more frozen shell commands

Runtime
  preflight(request)
  start_run(request) -> run_id
  run_to_terminal(run_id)
  snapshot(run_id) -> RunSnapshot
  events(run_id, after_seq) -> Event[]
  cancel(run_id)
  reconcile_interrupted()
```

`start_run` 验证输入、保留运行定义、创建运行worktree，并仅在持久运行标识存在后返回。`run_to_terminal` 拥有执行权，并且可以安全地放置在应用程序管理的 Tokio 任务上。

调度程序从持久节点状态和固定依赖关系中导出就绪集。初始图是线性的，但调度是基于状态的，而不是硬编码的函数链，因此后面的条件和并行节点不需要替换运行时边界。工作空间访问租赁和提供商容量仍然由 Core 强制执行。

每个节点执行执行以下序列：

1. 在当前 Run 所有者一代下进行尝试；
2. 解析并持久化不可变的上下文快照；
3. 渲染角色提示并调用新的真实 `codex exec` 回合；
4. 保留流式提供程序诊断和线程标识；
5. 根据角色的架构和业务不变量验证角色 JSON；
6. 将人类可读的输出和结构化结果发布为工件；
7. 记录切换并自动完成尝试和节点运行；
8. 发出 CLI 和桌面客户端使用的已提交事件。

测试人员是唯一可以通过特定于尝试的 Unix 套接字请求冻结验证检查的角色。运行时在现有的受限 Codex 沙箱下启动检查并拥有规范证据。Reviewer 接收 ChangeSet 和 TestResult 工件；未通过所需检查不能成为批准的结果。

## 桌面架构

添加 `apps/archmage-desktop` 应用程序：

```text
React Workbench
    │ invoke/listen
    ▼
Tauri command layer
    │ typed DTO conversion only
    ▼
archmage-core Runtime
    ├── Codex CLI
    ├── Git/worktrees
    └── SQLite/artifacts
```

Tauri 拥有一个应用程序 `AppState`，其中包含运行时句柄、活动运行任务句柄和事件转发订阅。React 从不打开 SQLite、直接调用 Git/Codex 或解析 CLI 标准输出。

初始 Tauri 命令面为：

- Archmage代币00000X
- Archmage代币00000X
- Archmage代币00000X
- Archmage代币00000X
- Archmage代币00000X
- Archmage代币00000X
- Archmage代币00000X

提交的核心事件作为包含 `run_id` 和 `event_seq` 的 `run-event` Tauri 事件转发。React 将该事件视为 invalidation/append 信号。在初始加载、刷新、序列间隙或侦听器重新连接时，它会调用 `get_run_snapshot` 和 `list_events`；正确性从来不依赖于接收每个内存中的事件。

## 用户体验

第一个布局使用批准的工作台优先选项 A：

- 左侧边栏：新运行和最近运行；
- 主标题：存储库、目标摘要、运行状态、运行 ID、worktree、取消和显示工作区操作；
- 新的运行卡：存储库选择器、目标文本区域、可重复检查命令字段和环境状态；
- 四节点进度条：Designer、Coder、Tester、Reviewer；
- 时间线：有序的持久运行时事件和简洁的提供程序进度；
- 节点检查器：上下文、结果、日志、工件、变更集和检查证据。

最初的视觉系统是有意可替换的。组件按产品职责而不是像素布局进行划分，因此后来用户提供的原型可以更改导航和外观，而无需更改 Tauri 命令或运行时 DTO。

客户端将 `doctor` 呈现为“环境检查”。启用“启动”之前会自动运行离线检查。真正的在线 Codex 探测是显式的，因为它消耗模型使用量。

## 生命周期和错误语义

在运行存在之前，存储库、目标、检查、Git、Codex 可执行文件、存储和模式必须通过预检。失败仍保留在表单上，​​并且不会创建部分运行。

持久创建运行后，故障将成为持久事件和最终 node/Run 状态。客户端不会从进程退出或乐观的 UI 状态推断成功。它呈现运行时快照。

取消是一个持久的控制请求。运行时验证拥有的进程身份，终止相关 Codex/check 进程组，并在 UI 将其报告为终端之前记录取消。

窗口关闭后，第一个桌面切片不会继续。以活动运行关闭会提示用户返回应用程序或取消并退出。如果进程被强制终止，则下次启动会将旧的 Run 调整为中断；它不要求后台执行。历史终端运行在重新启动后仍然可以检查。

## 测试策略

测试故意保持小规模：

- 重用现有的 Store、worktree、所有权、合同、上下文和检查测试；
- 添加针对 Ready Set 进程、一个完整的持续生命周期、取消和中断协调的重点核心测试；
- 仅针对运行表单序列化和事件状态减少添加小型 TypeScript 测试；
- 使用 `npm run build` 和 Rust `cargo check` 作为主要桌面编译门；
- 针对临时夹具存储库保留一个被忽略的、明确的真实 Codex 端到端验收；
- 手动启动 Tauri 开发应用程序一次，并验证存储库选择、运行进度、最终状态、重新启动历史记录和工作区显示。

不会运送或接受任何假冒代理作为产品证据。确定性单元测试可以执行纯 scheduling/state 函数，而无需假装模拟的提供程序证明了真正的集成。最终的垂直验收始终使用经过身份验证的本地 Codex CLI。

## 交货单

1. 公开最小的运行时外观和固定的工作流调度。
2. 完成真实的 Codex 功能调用和节点 artifact/Handoff 发布。
3. 添加取消、中断协调、快照和事件查询。
4. 支架 Tauri/React 并键入命令。
5. 构建工作台优先的 UI 并重新连接行为。
6. 运行真正的Codex垂直验收并更新用户指南。

第一个可用的检查点不是可视外壳：它必须从桌面启动真正的运行，并通过 CLI 使用的相同核心运行时取得持久进展。
