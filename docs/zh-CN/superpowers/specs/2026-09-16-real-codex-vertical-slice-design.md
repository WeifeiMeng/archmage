简体中文 | [English](../../../en/superpowers/specs/2026-09-16-real-codex-vertical-slice-design.md)

# Real Codex 垂直切片设计


**状态：** 已批准

**日期：** 2026-09-16

## 总结

S0 连接一个真正的本地 Codex CLI 提供商，并通过每次运行的 Git worktree中的显式交接来协调四个逻辑能力代理（设计者、编码者、测试者和审阅者）。每个节点都在新的 Codex 会话中运行，接收冻结的上下文快照，并且必须返回架构有效的结果。SQLite 和本地工件存储使每个状态转换、输入、输出和恢复决策都持久。

该切片有意跨越原始 M0-M5 层。它在概括工作流定义或构建桌面 UI 之前生成可运行的协作证明。

## 决定已获批准

1. 提供垂直切片而不是纯域基础。
2. 从第一阶段开始就使用真正的 Git worktree。
3. 为每个能力代理调用真正的 Codex；产品执行没有Fake Agent模式。
4. 在 Tauri 和 React 之前发布运行时和 CLI。
5. 将Provider视为物理执行后端，将Capability Agent视为协作参与者。
6. 允许并发就绪节点受依赖性、提供者容量和工作区访问限制。
7. 使用独立的 Codex 会话和显式切换，而不是跨节点会话历史记录。
8. 保持worktree和树枝完好无损；不要自动提交、合并、推送或删除。

## 架构

Rust 运行时拥有工作流状态、调度、上下文解析、契约、工作空间生命周期、持久性、工件和事件。提供者中立的接口将运行时策略与 Codex 流程细节分开。CLI 是运行时 API 的表示和控制层。

```text
CLI → Runtime → Capability Agent invocation → Codex Provider → codex exec
        │                 │                       │
        │                 └→ Context + Contract  └→ JSONL + final JSON
        ├→ Workspace Manager → Git worktree
        └→ Store → SQLite + local artifacts
```

尽管 S0 只有一种提供程序实现，但运行时类型使用提供程序中立的身份和结果。特定于提供者的 JSONL 类型保留在 Codex 适配器内。

## 协作流程

### 设计师

接收目标、基线存储库元数据和有界存储库视图。它以只读方式运行并返回版本化设计 JSON 和 `summary_markdown`。

### 编码员

接收目标和经过验证的设计工件。它在运行worktree中使用工作区写入访问权限运行，并返回一个 ImplementResult。运行时独立捕获结果状态以及与基线的差异。

### 测试员

接收目标、设计、实施结果、工作空间和显式验证策略。每项必需的检查都有一个 ID，并且必须由测试者通过​​Archmage拥有的检查运行程序调用。该运行程序在实际worktree中执行配置的命令，并在将控制权返回给 Codex 之前持久记录命令、退出状态、stdout、stderr 和持续时间。测试人员解释此证据并返回 TestResult。当所需证据丢失或包含非零结果时，运行时不变量会阻止 `passed`。

### 审稿人

接收目标、设计、实施结果、基线差异和测试结果。它以只读方式运行并返回 ReviewResult。S0 记录 `approved` 或 `changes_requested`，但将其视为最终审阅者结果；S1 向 Fixer 添加条件路由。两种结果都完成了工作流生命周期，而单独的完成结果则保留审核决策。CLI 返回 `approved` 成功，并为 `changes_requested` 返回不同的非零结果代码。

## 提供商协议

适配器使用非交互式界面调用已安装的 Codex CLI：

```text
codex exec
  --cd <run-worktree>
  --sandbox <read-only|workspace-write>
  --json
  --output-schema <capability-schema>
  --output-last-message <result-path>
  -
```

呈现的提示符写入标准输入。stdout 上的 JSONL 被增量解析并持久化；stderr 是单独捕获的。适配器记录 `thread.started` 标识符、进程标识、版本、时间戳、退出状态、可用时的使用情况以及终止路径。

正常切换永远不会调用 `codex exec resume`。在Archmage恢复时，中断的节点会通过新的尝试和相同的持久逻辑输入启动新的 Codex 会话。如果被中断的编写器留下部分工作区更改，则新尝试还会收到一个新的恢复上下文快照，该快照描述当前工作区及其与前一个快照的增量。以前的快照保持不变。这使得恢复独立于提供商本地对话文件，而不会假装代码更改在中断后消失。

## 上下文和切换

每次尝试在开始之前都会将类型化的上下文引用解析为不可变的快照。快照记录源、哈希、序列化版本、提示版本、提供程序绑定和工作区标识。下游节点只能读取已提交的上游工件。

切换是将生产者工件链接到消费者上下文绑定的显式记录。它们不会暴露隐藏的提供者推理或对话历史记录。

## 合约

每个内置功能都有一个版本化的 JSON 架构。Codex 被要求通过 `--output-schema` 来符合，并且运行时再次验证最终文件。特定于功能的不变量验证模式本身无法表达的关系。

每个有效结果都包含 `summary_markdown`。运行时为机器存储经过验证的 JSON、为人员提取的 Markdown 以及用于诊断的原始提供商流。

## 调度程序和并发

调度程序从持久的依赖关系和上下文可用性中派生出就绪集。S0的内置工作流程是线性的，但引擎允许兼容的节点同时运行。

节点声明工作空间访问：

- `none`：与所有其他访问模式兼容
- `read`：与 `none` 和其他读卡器兼容
- `write`：仅与 `none` 兼容并拥有独家worktree租约

Dispatch 还尊重 `run.max_parallelism` 和提供商容量。状态转换加上事件插入是原子的。

## 工作区生命周期

运行创建将源存储库的当前 `HEAD` 冻结为基线，并在 `<archmage-data>/workspaces/<run-id>` 中创建 `archmage/run-<run-id>`。脏源更改被排除并报告为警告。

在每个可写节点之前和之后都会检查worktree身份、基线、分支、`HEAD` 和状态。功能指令禁止提交，并且更改的 `HEAD` 使工作区不变式失败。差异工件的生成与代理声明无关。S0 在所有最终结果之后保留worktree和分支。

## 坚持与恢复

SQLite 使用迁移、WAL、外键、事务和有界繁忙处理。工件内容在元数据插入之前使用临时文件写入、刷新和原子重命名。

启动时，将协调 `running` 中剩余的拥有尝试。如果无法安全地确认其进程，则尝试将变为 `interrupted`。`archmage resume` 保留成功的节点，丢弃上下文资格中的不完整输出，并为可重试工作创建新的尝试。

协调需要运行的排他调度程序所有权。中断的分类并不表明旧的子进程已停止：替换编写器必须等到拥有的 Codex/check 进程被确认退出或安全终止。不确定的进程身份会阻止执行，而不是将信号授权给可能重用的 PID。只读状态检查不会接管活动运行。

## 误差模型

S0至少区分：

- Archmage代币00000X
- Archmage代币00000X
- Archmage代币00000X
- Archmage代币00000X
- Archmage代币00000X
- Archmage代币00000X
- Archmage代币00000X
- Archmage代币00000X
- Archmage代币00000X
- Archmage代币00000X

每次故障都会保留可用的诊断信息，发出类型化事件，并使worktree可供检查。自由形式的散文不能覆盖失败的退出状态或不变量。

## 命令行界面

S0暴露：

```text
archmage doctor
archmage run --repo <path> --goal <text> --check <command> [--check ...]
archmage status <run-id> [--json]
archmage logs <run-id> [--follow]
archmage resume <run-id>
archmage cancel <run-id>
```

`run` 在前台执行并传输已提交的事件。Ctrl-C 请求取消。命令共享运行时包而不是复制工作流行为。

工作流命令退出代码为：

|代码|意义|
|---:|---|
|`0` |工作流程已完成并获得审阅者批准 |
|`10` |使用 `changes_requested` 完成工作流程 |
|`20` |预检或基础设施故障阻碍了可靠的执行 |
|`21` |能力代理、契约或工作区不变量失败 |
|`22` |跑步被取消 |

CLI 参数错误保留解析器的标准非零行为并且从不创建运行。

## 测试

自动化测试涵盖域转换、Ready Set 和租赁兼容性、迁移、事务回滚、恢复查询、工件原子性、真正的临时 Git 存储库和worktree、schema/invariant 验证、上下文哈希以及来自固定协议示例的 Codex JSONL 解析。

协议样本是解析器固定装置，不是可执行提供程序，也不是产品Fake Agent路径。完整的快乐路径运行将为所有四个角色调用真正经过身份验证的 Codex。单独的 interruption/resume 场景必然会创建额外的实际调用。

## 安全性

- 使用显式 `read-only` 或 `workspace-write` 沙箱模式；S0 中从来没有 `danger-full-access`。
- 不要将凭据或环境机密序列化为提示、事件或工件。
- 将存储库内容和代理输出视为不可信数据。
- 无需 shell 插值即可传递进程参数。
- 限制文件系统写入运行worktree和Archmage数据根。
- 不要改变源存储库的签出分支或任何远程分支。

## 范围排除

S0 不包括修复循环、人性门、外部工作流程编辑、其他提供程序、桌面 UI、守护进程、自动清理、远程执行、内存、RAG、市场和 cloud/multi-user 功能。

## 验收

权威退出标准位于 [S0 阶段文档 ](../../roadmap/stages/s0-real-codex-slice.md) 中，可执行手动过程位于 [S0 验收运行手册 ](../../runbooks/s0-acceptance.md) 中。

[S0 实施计划 ](../plans/2026-09-16-s0-real-codex-vertical-slice.md) 指定文件所有权、任务依赖性、测试门以及上述实施细节。本文档未声明任何产品实施或真正的提供商接受。
