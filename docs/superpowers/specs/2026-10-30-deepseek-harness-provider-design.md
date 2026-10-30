# DeepSeek Harness Provider 支持设计

## 目标

为 Archmage 增加可真实执行的 DeepSeek Harness Provider。用户可以在智能体管理中创建、检查和验证 DeepSeek Harness 实例，将 Designer、Coder、Tester 或 Reviewer 绑定到该实例，并通过现有线性 Team、咨询边和消息流 Team 完成真实 Run。

本阶段只实现 DeepSeek Harness。Hermes、Claude 和 OpenClaw 继续显示为尚未支持。

## 接入方式

采用 DeepSeek Harness 的短进程 headless CLI，并为它增加显式的新建与恢复参数：

```text
dsh --profile headless --session-id <id> "<task>"
dsh --profile headless --resume-session-id <id> "<task>"
```

Archmage 以 Run 的隔离 worktree 作为当前目录启动进程。进程继承 Archmage 当前环境，包括默认 `DSH_HOME`、凭证、Provider 配置和模型选择；Archmage 不复制或管理 DeepSeek 凭证。

`--session-id` 只创建不存在的新会话，已存在时失败；`--resume-session-id` 只通过 `ctx.agents.resume()` 恢复已有会话，不存在时失败。两者都追加一条任务、等待完成、持久化并退出。每次进程仍只处理一条任务并只输出本轮最后一条 assistant 消息，因此 Archmage 不需要持有长驻服务。

第一版不提供 DeepSeek Harness 模型覆盖。智能体管理明确显示“使用 DeepSeek Harness 当前默认模型与 DSH_HOME”，实例的 `model` 必须为 `null`。

## Provider 模型与存储

- 新增 Provider 类型 `deepseek_harness`，并标记为支持真实执行。
- 数据库迁移扩展 `agent_instances.provider_kind` 约束，并创建未绑定的 `deepseek-harness-default` 实例。
- 升级后保留所有现有 Capability 绑定，不自动将工作交给新 Provider。
- Run 创建时冻结实例 ID、显示名称、Provider 类型和已解析的可执行路径。
- Archmage 为每个 `Run + Capability` 创建并持久化一个稳定、不可变的 DSH session ID。不同 Run 或 Capability 永不共享该 ID，即使它们绑定同一个 DSH 实例。
- 会话映射记录 `reserved`、`ready` 或 `uncertain` 状态。首次成功完成后从 `reserved` 进入 `ready`，后续调用只能恢复；首次 invocation 在确认持久化前异常中断则进入 `uncertain` 并明确失败，不能猜测应新建还是恢复。
- 每次子进程仍有独立的 Provider invocation ID，用于诊断、取消和 attempt 关联；DSH session ID 才是对话身份，两者不得混用。

## 发现与连接验证

“检查 CLI”解析配置的可执行文件路径；留空时从 `PATH` 查找 `dsh`。检查以下命令成功且输出可识别：

```text
dsh --version
dsh --profile headless --help
```

“验证连接”会产生一次真实模型调用。Archmage 在临时 Git 工作目录中以只读权限运行最小 headless 任务，并要求返回固定的 JSON 对象。成功表示 CLI、headless profile、默认 `DSH_HOME`、凭证和模型路由当前可用。

更改可执行路径或启用状态会使已有验证状态失效。验证失败只向界面返回安全摘要，原始 stdout 和 stderr 保存在 Archmage 私有诊断目录。

## 统一 Provider 执行边界

Core 增加 Provider 中立的执行请求，包含：

- Provider 类型和可执行路径；
- 工作目录与只读或工作区写入权限；
- 完整任务上下文和目标 JSON Schema；
- 可选的 Provider 会话 ID；
- 超时、单行、总输出和结果大小限制；
- 子进程身份注册回调。

Codex Adapter 继续使用当前 `--json`、`--output-schema` 和结果文件协议。DeepSeek Harness Adapter 将上下文、Schema 以及“只返回单个 JSON 对象，不使用 Markdown 代码块”的要求组成一个 headless task。

DeepSeek Harness 的 stdout 必须整体解析为单个 JSON 对象，再进入现有 Capability 契约校验。实现不从说明文字或 Markdown 中宽松提取 JSON，防止无效结果被接受。

普通节点、咨询边和消息流 Team 都通过同一 Provider 分派函数执行，不能只在其中一条 Runtime 路径支持 DeepSeek Harness。普通节点的重试、Reviewer 打回后的重做，以及消息流中同一 Capability 的后续投递复用该 Capability 的 DSH session。

咨询调用是独立的只读评估任务，使用一次性 DSH session，不读写目标 Capability 的主对话，避免咨询内容污染后续正式执行。

## 权限、生命周期与恢复

- Designer、Reviewer 和咨询任务使用只读权限；Coder、Tester 使用工作区写入权限。
- 权限通过 DeepSeek Harness 支持的环境配置传入，工作目录始终是该 Run 的 worktree。
- Archmage 继续拥有超时和取消。取消会终止登记的整个 `dsh` 子进程组，避免遗留工具进程。
- 取消只中止当前 invocation；此前已完成轮次的 DSH session 保留，下一次 attempt 可以恢复它。
- Archmage 恢复中断节点时启动新的 headless invocation 和新的 attempt，并恢复该 Capability 已持久化的 DSH session。
- 若应该存在的持久 session 丢失、损坏、状态不确定或不能恢复，本次执行明确失败，不能静默创建新对话。
- Archmage 的显式 Context Snapshot、Capability 输入和输出契约仍是可审计交接依据；DSH 对话历史只提供同一 Capability 内的连续性，不能替代显式上下文。
- DeepSeek Harness 第一版不提供实时 token 事件。

## 智能体与 Capability 管理

- 智能体管理的新增实例操作支持选择 Codex 或 DeepSeek Harness。
- DeepSeek Harness 卡片可编辑实例名称、`dsh` 路径和启用状态，并展示版本、连接状态与上次验证时间。
- 页面指标统计所有受支持 Provider 的实例，不再只统计 Codex。
- Capability 管理允许选择任一已启用的 Codex 或 DeepSeek Harness 实例，并展示真实 Provider 名称。
- Hermes、Claude 和 OpenClaw 保留“尚未支持”卡片，不能创建可运行实例或绑定 Capability。

## 失败处理

非零退出、超时、取消、session 恢复失败、输出超限、无效 UTF-8、无效 JSON 和 Capability 契约不匹配分别产生明确失败。诊断文件使用私有权限，不记录或回显凭证。Run 和 attempt 继续使用现有持久状态转换，Provider 失败不能发布成功结果或 Changeset。

## 最小验证范围

- DeepSeek Harness 测试覆盖 headless session 的首次创建和再次恢复。
- Archmage Provider 测试覆盖 CLI 发现、命令构造、严格 JSON 解析和失败退出。
- Runtime 测试证明 Capability 可绑定并冻结 DeepSeek Harness，同一 Capability 复用 session，不同 Capability 和咨询调用隔离。
- 前端测试覆盖新增实例、指标统计和 Capability 选择。
- 使用伪 `dsh` 完成一次离线 Run 回归。
- 最后在本机默认 `DSH_HOME` 下运行一次小型真实 smoke Run；若凭证不可用，明确报告该项未完成，不用模拟结果替代。

## 非目标

- Hermes Provider 实现；
- ACP 或 JSON-RPC 长驻连接；
- Archmage 内的 DeepSeek 凭证、账号或 `DSH_HOME` 管理；
- DeepSeek Harness 模型覆盖；
- Provider 自动选择、故障转移或负载均衡；
- DeepSeek Harness 实时事件流。
