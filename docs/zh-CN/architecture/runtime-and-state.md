简体中文 | [English](../../en/architecture/runtime-and-state.md)

# Runtime 与状态

## 定义与执行分离

Workflow Definition 描述节点与依赖。Workflow Run 会快照定义版本、输入、基线 commit、Provider 绑定与执行策略。之后编辑定义不会改变已有 Run。

Node 定义指定一个 Capability 及其 Context 绑定，Node Run 记录执行状态。每次重试或恢复都会创建新 Attempt；Attempt 永不覆盖。

Capability 绑定属于全局配置，但创建新 Run 时会解析为具体 Agent instance ID 并冻结。Agent instance 保存 Provider 类型以及 executable/model 配置。改变全局绑定不会重写活跃或历史 Run 定义。

内置 Tester 接收测试任务，而不是预先声明的命令列表。绑定的 Agent 在 Run worktree 中选择相关命令，并返回结构化命令结果与限制。Runtime 继续保留旧 CheckRunner ledger 作为底层基础设施，但内置目标驱动 Workflow 不使用它。

## 节点状态

```text
pending → ready → running → succeeded
                     ├────→ failed
                     ├────→ interrupted
                     └────→ cancelled
```

`succeeded` 必须同时满足：

- Provider 进程成功完成。
- 最终输出可解析为 JSON。
- JSON Schema 校验通过。
- Capability 特定不变量通过。
- 必需 Artifact 已持久注册。

## Scheduler

Scheduler 从持久状态推导 Ready Set。只有所有必需前驱均已成功且 Context Reference 可以解析时，节点才会 ready。

Run 不存在全局单节点限制。分派受以下因素限制：

- `run.max_parallelism`
- 依赖就绪状态
- Provider 容量
- 工作区访问兼容性

工作区访问兼容性：

| 当前访问 | 候选 `none` | 候选 `read` | 候选 `write` |
|---|---:|---:|---:|
| `none` | 允许 | 允许 | 允许 |
| `read` | 允许 | 允许 | 等待 |
| `write` | 允许 | 等待 | 等待 |

S0 Workflow 因数据依赖呈线性，但 Scheduler 与 lease 模型并未硬编码为串行执行。

## 原子状态转换

每次状态转换及其对应 Event 都在同一个 SQLite 事务中提交。消费者必须从持久化实体重建 Runtime 状态，不能只依赖内存 Event。

## 恢复

Runtime 启动时扫描持久化的 `running` Attempt。如果记录的进程无法确认是受控的存活进程，该 Attempt 会变为 `interrupted`，同时由 Event 记录恢复决策。

`archmage resume <run-id>` 根据持久状态重建 Ready Set。它保留成功节点、忽略不完整 Artifact，并为 interrupted 或可重试的失败节点创建新 Attempt。S0 恢复是显式操作；不存在自动恢复工作的 daemon。

## 取消

取消操作针对受控 Provider process group，等待有界宽限期，必要时强制终止。Runtime 记录 signal 路径，并一致地标记 Attempt 与下游 Run 状态。一旦取消赢得状态转换，已取消 Attempt 就不能再发布成功结果。
