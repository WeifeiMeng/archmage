简体中文 | [English](../../en/architecture/overview.md)

# 架构概览

## 边界

Archmage 从一个包含 UI 无关 Runtime 与轻量 CLI 的 Rust workspace 起步。只有协作 Runtime 得到验证后，才引入 Tauri 与 React。

```text
archmage CLI
    │
    ▼
collaboration runtime
    ├── scheduler and state machine
    ├── capability and context resolver
    ├── contract validation
    ├── provider adapters
    ├── workspace manager
    ├── artifact store
    └── event and persistence store
          │                  │
          ▼                  ▼
     Codex CLI          Git + SQLite + files
```

CLI 依赖 Runtime 的公共接口。Runtime 不依赖 Tauri、React、终端呈现或 Provider 特定的 UI 概念。

## S0 组件边界

| 组件 | 职责 | 不得负责 |
|---|---|---|
| CLI | 解析命令、流式展示事件、渲染状态、映射退出码 | Workflow 策略或持久化 SQL |
| Runtime | 协调生命周期、调度、状态转换与恢复 | Codex 事件细节或 shell 呈现 |
| Capability | 定义角色指令、输入/输出契约与权限 | Provider 进程管理 |
| Context Resolver | 解析类型化引用并冻结不可变 Snapshot | 隐藏的 Provider 历史 |
| Codex Provider | 检测 CLI、调用 `codex exec`、解析 JSONL、取消进程 | Workflow 路由决策 |
| Workspace Manager | 创建和检查 branch/worktree、强制 lease | Capability prompt |
| Store | 执行迁移、持久化状态/事件、管理 Artifact 元数据 | 业务调度策略 |

## S0 执行路径

1. 校验仓库、Codex CLI 可用性、目标、Schema 与校验策略。
2. 持久化 Run 及其内置 Workflow Snapshot。
3. 从源仓库当前 `HEAD` 创建 Run branch 与隔离 Git worktree。
4. 构建 Ready Set 并分派兼容节点。
5. 对每个节点冻结上下文、渲染 Capability 指令，并调用新的真实 Codex 会话。
6. 持久化 Codex JSONL 事件、校验最终 JSON 契约、创建 Artifact，并原子转换节点状态。
7. 依次完成 Designer、Coder、Tester 与 Reviewer，然后写入最终 Run 报告。

## 演进

S1 将 Workflow 和 Capability 定义外部化，并引入条件、重试、Fixer 循环与人工门禁。S2 在同一个 Adapter 边界后增加 Provider。S3 通过 Tauri command 使用 Runtime，并把 Runtime 事件流传给 React。
