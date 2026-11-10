# Qoder ACP Provider 支持设计

**状态：** 已确认，待实现计划

**日期：** 2026-11-10

## 目标

为 Archmage 增加可真实执行的本地 Qoder Provider。Archmage 作为 ACP Client 启动并管理独立的 `qoder --acp` 子进程，使 Designer、Coder、Tester 或 Reviewer 可以绑定到 Qoder 实例，并继续使用现有 Context Snapshot、Capability Schema、Artifact、ChangeSet 和 Team 路由完成 Run。

本设计不接管已经打开的 Qoder IDE、WorkBuddy 界面或既有聊天。Archmage 创建并拥有 Qoder 会话、执行目录、权限决策和生命周期。

## 核心决策

- 使用 Qoder CLI 的 ACP v1 stdio 接口，不使用桌面 UI 自动化，也不以 Headless print mode 作为正式传输层。
- 每个 `Run + Capability` 独占一个 Qoder ACP 子进程和 session。同一 Capability 的后续反馈轮次复用该 session；不同 Capability 不共享隐藏历史。
- Archmage 是权限、工作区、交付物和持久化的事实所有者。Qoder 负责 session 内的推理、计划和工具调用。
- Designer 和 Reviewer 只读；Coder 和 Tester 只允许写当前 Run 的独立 worktree。工作区外写入和高风险破坏操作默认拒绝。
- 最终回复必须且只能是一个符合当前 Capability JSON Schema 的 JSON 对象。Archmage 不从 Markdown 或附加文字中宽松提取结果，也不自动发起模型修复。
- Archmage 不保存 Qoder 凭证。实例复用本机 Qoder CLI 的认证状态或启动环境中的 `QODER_PERSONAL_ACCESS_TOKEN`。
- 默认只复用认证状态。个人 Memory、MCP、Plugin、隐式 Skill 和默认 Agent 不自动继承；模型、Qoder Agent 和扩展必须在实例中显式配置。

## 架构

```text
Archmage Runtime
  ├─ QoderProvider
  │   ├─ discover / inspect / verify
  │   └─ create_session / prompt / cancel / close
  ├─ AcpClient
  │   ├─ JSON-RPC v1 编解码与请求关联
  │   ├─ session/update 事件处理
  │   ├─ permission / filesystem / terminal 回调
  │   └─ 超时、输出上限与协议校验
  └─ ProviderSessionPool
      └─ key = Run ID + Capability
              ↓ stdio
         qoder --acp
```

新增 `ProviderKind::Qoder`。`QoderProvider` 负责 Qoder 的可执行文件发现、版本检查、启动参数、ACP session 生命周期和结果转换。通用 `AcpClient` 只实现 ACP 协议，不包含 Archmage Capability 规则，以便未来复用到其他 ACP Agent。

Runtime 通过统一的 Provider session 接口取得单次或持久执行会话。该接口容纳 Codex 的单次调用、DeepSeek Harness sidecar 和 Qoder ACP session，替代 Runtime 中按具体 Provider 扩张的条件分支。此重构只服务于 Provider 生命周期统一，不改变既有 Codex 或 DeepSeek Harness 的外部行为。

Archmage 继续拥有以下职责：

- 构造并持久化不可变 Context Snapshot；
- 选择并冻结 Agent 实例配置；
- 校验 Capability 输出与消息流路由；
- 发布 Artifact 和 Coder ChangeSet；
- 维护 Attempt、delivery、事件和终态；
- 创建并约束 Run worktree；
- 裁决资源访问并保存有界诊断。

Qoder session 内的计划、工具调用和对话历史不能替代明确的 Artifact 交付。

## ACP 连接和调用流程

Archmage 以 stdio 启动 `qoder --acp`，登记 PID、进程组、启动身份和私有诊断目录。stdout 只作为 ACP JSON-RPC 2.0 协议流，stderr 作为有界诊断流。

一次新 session 的流程为：

1. Archmage 发送 `initialize`，声明 ACP v1 以及实际支持的权限、文件和终端 Client capabilities。
2. Qoder 必须返回兼容的协议版本和所需 Agent capabilities。版本或关键能力不兼容时立即失败。
3. Archmage 发送 `session/new`，其中 `cwd` 固定为当前 Run 的独立 worktree，MCP 列表只包含实例显式配置并已冻结的条目。
4. Archmage 保存 Qoder 返回的 session ID，并把它作为 Provider conversation identity 持久化。
5. Archmage 通过 `session/prompt` 发送渲染后的 Context Snapshot、Capability 指令和严格 JSON 输出契约。
6. Qoder 通过 `session/update` 报告文本、计划、工具调用和状态；Archmage 将允许公开的部分转换成有界 Runtime 事件。
7. `session/prompt` 返回 stop reason 后，Archmage 汇总最终 `agent_message_chunk`，严格解析一个 JSON 对象，并执行既有 Capability 和路由校验。
8. 校验成功后发布 Artifact；Coder 的实际 Git 差异仍由 Archmage 扫描并发布为 ChangeSet。

同一 `Run + Capability` 的后续轮次复用进程和 session。不同 Capability 即使绑定同一 Qoder 实例，也各自创建进程和 session。咨询调用使用临时隔离 session，不进入 Capability 主对话。

## 事件与内容处理

允许映射到 Archmage Runtime 的 ACP 更新包括：

- Agent 最终可见消息块；
- 计划摘要和计划状态；
- 工具调用名称、状态和脱敏后的有界结果摘要；
- session mode、配置和使用量等安全元数据；
- Qoder 返回的用户可见 notice。

Agent thought 内容不进入 Artifact，默认不持久化原文。Archmage 对单帧、单行、累计 stdout、stderr、最终消息和工具输出分别设置大小上限。未知但符合 JSON-RPC 扩展规则的通知可以忽略并记录安全元数据；违反 ACP v1 基础顺序、请求关联或 session 身份的消息使 Attempt 失败。

## 权限、文件和终端边界

Archmage 不以 Qoder 的全局 YOLO 或 bypass-permissions 模式启动 Provider。所有需要授权的操作由 ACP Client 回调裁决，授权只对当前请求生效。

### 路径规则

- 所有文件路径在使用前转换为规范绝对路径。
- 路径必须位于当前 Run worktree 内；符号链接解析后也必须满足该约束。
- Designer 和 Reviewer 可以读取 worktree，但不能写入。
- Coder 和 Tester 可以读写 worktree。
- Archmage 数据目录、用户凭证目录、其他仓库、其他 Run worktree 和系统敏感路径不可访问。

### 终端规则

- 终端的工作目录必须位于当前 Run worktree 内。
- Designer 和 Reviewer 只允许符合只读执行策略的命令。
- Coder 和 Tester 允许执行构建、测试和工作区内开发命令。
- 提权、修改系统配置、访问凭证、越界写入以及针对工作区、HOME、文件系统根或父目录的破坏性操作一律拒绝。
- ACP permission request 无法被安全分类时默认拒绝，不从自然语言说明推断授权。

权限拒绝本身不是 Provider 故障。Archmage 把拒绝结果返回 Qoder，由 Qoder 选择替代方案；如果最终无法完成，由 turn stop reason 或严格结构化结果收敛 Attempt。

## 输出契约

ACP 是传输协议，不替代 Archmage Capability Schema。每个 prompt 都包含当前版本的输出约束，要求最终可见回复只包含一个 JSON 对象。

Archmage 按以下规则处理最终输出：

- 仅拼接最终 Agent 可见消息块；
- 输入必须是合法 UTF-8；
- 去除 JSON 语法允许的首尾空白后，只能存在一个 JSON 对象；
- Markdown 代码围栏、解释性前后缀、多个 JSON 值、截断内容和未声明字段均不接受；
- 解析成功后继续执行现有 Capability Schema 和路由校验；
- 失败统一归属 `contract_invalid`，不进行宽松提取、猜测或自动修复轮次。

## Agent 实例和冻结配置

Qoder Agent 实例保存：

- 实例 ID 和显示名称；
- `qoder` 可执行路径，留空时从 `PATH` 查找；
- 可选模型；
- 可选的显式 Qoder Agent 名称；
- 显式允许的 MCP、Plugin 和 Skill 配置；
- 启用状态、发现版本、连接状态、最后验证时间和安全错误摘要。

Archmage 不保存或回显 Qoder Token、登录 URL、完整环境或 `~/.qoder` 内容。启动环境可以包含 `QODER_PERSONAL_ACCESS_TOKEN`，但诊断信息不得记录它。

Run 创建时冻结实例 ID、显示名称、Provider 类型、已解析可执行路径、发现版本、模型、Agent 名称和显式扩展配置。之后修改全局 Agent 实例不改变既有 Run。

为保证可复现性，启动 Qoder ACP 时使用隔离配置边界：

- 复用 Qoder 认证状态；
- 不自动加载个人 Memory；
- 不自动加载未显式选择的 MCP、Plugin、Skill 或默认 Agent；
- 显式配置进入 Run 快照、hash 和审计记录；
- 未进入快照的本机设置不得影响执行。

## 发现和连接验证

Provider 发现执行 `qoder --version` 并校验可执行文件可启动。实例“验证连接”在临时只读 Git 工作目录中执行一个完整、最小化的真实 ACP 流程：

1. 启动 Qoder ACP；
2. 完成 `initialize` 和能力协商；
3. 创建只读 session；
4. 发送固定验证 prompt，要求返回固定 JSON 对象；
5. 严格校验响应；
6. 取消仍在运行的操作并完整关闭进程。

验证成功表示当前可执行文件、ACP 版本、认证、模型、显式 Agent、真实模型调用和严格输出路径均可用。修改可执行路径、模型、Agent、扩展配置或启用状态会使既有验证状态失效。

## 生命周期、取消和恢复

- Run 完成、失败或取消时，如果 turn 仍在运行，Archmage 发送 `session/cancel`。
- cancel 宽限期后进程仍未退出，Archmage 终止已登记并重新校验身份的整个进程组。
- Qoder 进程意外退出时，当前 Attempt 失败；Archmage 不在后台静默创建新 session。
- Archmage 重启时，仍处于执行中的 Attempt 收敛为 `interrupted`，不自动续跑可能执行到一半的工具调用。
- 用户显式重试时，只有 Qoder 声明 `loadSession` capability、session ID 可用且 worktree 基线与记录的 snapshot/hash 一致，Archmage 才调用 `session/load`。
- 任一恢复前提不成立时，重试创建新的 Attempt 和独立 session，不伪造历史连续性。
- consultation session 始终临时创建和销毁，不参与恢复。

## 错误归属与诊断

| 条件 | 归属 |
|---|---|
| `qoder` 缺失、版本检查失败、必要 ACP capability 缺失 | `provider_unavailable` |
| 未登录或 Token 无效 | `authentication_required` |
| 非法 JSON-RPC、ID 不匹配、未知 session、重复完成 | `provider_protocol_error` |
| 用户或 Runtime 取消 | `cancelled` |
| 超时 | `provider_timeout` |
| Qoder 非预期退出 | `provider_failed` |
| 最终输出缺失、超限、非单一 JSON 或 Schema 不符 | `contract_invalid` |
| Archmage 重启中断正在执行的 turn | `interrupted` |

诊断目录使用私有权限，只保存有界协议元数据、脱敏后的 Qoder stderr、进程身份、状态码和错误分类。不保存 Token、完整环境、无限制会话正文或 Agent thought 原文。对文件和终端请求只记录完成诊断所需的最小信息。

## 数据库与界面

数据库迁移扩展 `agent_instances.provider_kind` 约束以支持 `qoder`，并增加保存可选 Qoder Agent 与显式扩展配置所需的字段。迁移保留既有实例、Capability 绑定和 Run 数据。

Agent 管理界面新增固定 Qoder Provider 分组和“添加 Qoder”入口。实例卡片允许编辑显示名称、可执行路径、可选模型、可选 Agent、显式扩展和启用状态，并展示版本、ACP 协议状态、连接状态和最后验证时间。

界面不提供 Token 输入框。认证失败时只提示用户使用 Qoder CLI 登录或通过启动环境提供 PAT，不显示或保存敏感值。

Capability 管理允许 Designer、Coder、Tester 和 Reviewer 绑定任意已启用且验证通过的 Qoder 实例。Run 启动检查继续要求所有使用的 Capability 都绑定到受支持、启用且就绪的实例。

## 测试与验收

### 协议单元测试

- JSON-RPC 分帧、部分读取、并发请求关联和乱序响应；
- notification、error response、未知扩展和非法基础消息；
- session/update 各受支持类型；
- 单帧、单流、stderr、工具结果和最终消息上限；
- 严格 UTF-8 与单一 JSON 对象解析。

### 可控 ACP 集成测试

使用仓库内可控的假 Qoder ACP 进程覆盖：

- initialize 和 capability 协商；
- session/new、连续 prompt 和独立 Capability session；
- permission 允许、拒绝和无法分类；
- fs read/write、符号链接逃逸和终端 cwd 逃逸；
- cancel、超时、进程组关闭和异常退出；
- loadSession 可用、不可用和 worktree 基线不一致；
- stderr 脱敏与诊断上限。

### Runtime 与界面回归

- Qoder Run 的 Context Snapshot、Schema、Artifact、ChangeSet 和 delivery；
- Codex、DeepSeek Harness 与 Qoder 混合 Team 的交付；
- Run 配置冻结和实例修改失效验证；
- Agent 管理分组、实例表单、连接验证和 Capability 绑定；
- 既有 Codex 与 DeepSeek Harness 行为保持不变。

### 真实 smoke test

增加默认 ignored 的真实 Qoder smoke test，使用已安装并已认证的 Qoder CLI 完成一个最小 Run。缺少 CLI、登录、网络或额度时，测试明确报告缺失的前置条件，不以 mock 结果替代真实通过。

## 不在范围内

- 控制 Qoder IDE 或 WorkBuddy GUI；
- 导入或接管既有 Qoder 聊天；
- 跨 Run 自动共享 Qoder session；
- 把个人 Qoder Memory 隐式注入 Run；
- Archmage 管理 Qoder 账号、Token 或订阅；
- Provider 自动选择、故障转移或负载均衡；
- ACP v2 draft 支持；
- 修改、fork 或发布 Qoder 客户端。

## 完成标准

- 用户可以创建、发现并验证本地 Qoder ACP 实例。
- 任一 Capability 可以绑定 Qoder 并完成真实 Run。
- 每个 `Run + Capability` 使用独立且受控的 Qoder session。
- 读写权限、取消、超时、异常退出和重启恢复符合本设计。
- 最终结果经过与其他 Provider 相同的严格 Capability 和路由校验。
- Codex 和 DeepSeek Harness 的现有测试与真实执行边界不退化。
