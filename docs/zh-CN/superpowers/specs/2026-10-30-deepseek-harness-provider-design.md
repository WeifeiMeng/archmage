简体中文 | [English](../../../en/superpowers/specs/2026-10-30-deepseek-harness-provider-design.md)

# DeepSeek Harness Provider 支持设计


**状态：** 已按 2026-10-30 的方案调整确认；本设计取代此前修改 DSH headless CLI 的方案。

## 目标

为 Archmage 增加可真实执行的 DeepSeek Harness Provider。用户可以在智能体管理中创建、检查和验证 DeepSeek Harness 实例，将 Designer、Coder、Tester 或 Reviewer 绑定到该实例，并通过现有线性 Team、咨询边和消息流 Team 完成真实 Run。

本阶段只实现 DeepSeek Harness。Hermes、Claude 和 OpenClaw 继续显示为尚未支持。

## 集成边界

Archmage 不修改、fork 或要求上游合并 DeepSeek Harness 源码。集成只依赖 DeepSeek Harness 发布的 Python SDK `deepseek-harness-sdk` 及其同版本 bundled runtime `deepseek-harness-runtime-bin`。

Archmage 提供一个自身维护的轻量 Python bridge。bridge 导入官方 `deepseek_harness` 模块，使用 SDK 的高层 owned-run API 启动 bundled JSON-RPC runtime，并在多个调用之间持有同一个 named session。Rust Runtime 只与 bridge 交换边界清晰的 JSONL 消息，不导入 DSH 内部 TypeScript 包，也不依赖未发布的 headless 参数。

SDK 的 bundled default config 只保证基础 runtime，并不等同于 Archmage 需要的完整编码工具组合。Archmage 因此随自身发布一份 Cordis 配置，通过 SDK 的公开 `cordis` 参数传入 bundled runtime。该配置只引用 runtime wheel 已打包的公开插件，明确组合 JSON-RPC server、Agent、模型适配器、持久化、受沙箱约束的 shell 与文件工具、自动拒绝式 approval 策略以及必要的上下文组件；不复制或修改 DSH 源码。

bridge 进程继承 Archmage 当前环境，包括 `DSH_HOME`、Provider 凭证、endpoint 和 SDK 支持的其他环境变量。官方 SDK 的 `initialize` 要求显式提供 Provider route 和 model，因此这两项由 Agent 实例配置，而不是从 DSH profile 猜测。

## Agent 实例与存储

- 新增 Provider 类型 `deepseek_harness`，并标记为支持真实执行。
- DeepSeek Harness Agent 实例保存 Python 可执行路径、Provider route、model、显示名称和启用状态。Python 路径留空时从 `PATH` 查找 `python3`。
- DSH 实例的 Provider route 和 model 必须为非空；Codex 实例不使用 Provider route，并继续允许自己的 model 覆盖语义。
- 数据库迁移扩展 `agent_instances.provider_kind` 约束并增加可空 `provider_route`。升级后保留所有既有实例和 Capability 绑定。
- 不创建带有猜测模型的默认 DSH 实例。用户明确新增并验证实例后再绑定 Capability。
- Run 创建时冻结实例 ID、显示名称、Provider 类型、已解析的 Python 路径、Provider route 和 model。之后修改全局 Agent 配置不改变既有 Run。

## Sidecar 与会话所有权

Archmage 为每个 `Run + Capability` 懒启动一个独立 bridge sidecar。sidecar 内部拥有一个官方 SDK client 和一个稳定 named session ID：

- 同一 Run 中，同一 Capability 因 Reviewer 反馈、重试或消息流再次执行时，继续使用同一 sidecar 和同一 DSH session。
- 不同 Run 或 Capability 永不共享 sidecar 或 session，即使它们绑定同一个 Agent 实例。
- sidecar 从第一次调用存活到 Run 完成、失败、取消或 Runtime 回收；终态必须关闭 SDK client 并等待 bridge 与 bundled runtime 退出。
- sidecar 意外退出后进入永久失败状态。本 Run 不静默创建替代 session，以免把丢失历史伪装成连续对话。
- 咨询调用使用独立临时 bridge 和临时 session，完成后立即关闭，不进入目标 Capability 的主对话。

第一版只保证一次 Archmage Run 存活周期内的连续对话。Archmage 或桌面客户端重启后，不恢复 DSH session；现有恢复逻辑终止遗留进程并将对应 Run 收敛为 `interrupted`。图形化 resume 本来就不在当前产品范围内，因此不新增跨重启承诺。

## Bridge 协议

Archmage 将 bridge 脚本作为自身资源发布，并通过 Python 启动。启动参数或第一条控制消息包含工作目录、Provider route、model、session ID 和输出上限。进程启动后：

1. bridge 构造官方 SDK client，传入 Archmage 发布的 Cordis 配置；SDK 自行解析并启动同版本 bundled runtime。
2. Rust 为每次任务发送带 invocation ID、任务文本和 JSON Schema 的单行 JSON 请求。
3. bridge 在固定 named session 上调用 SDK，等待当前 activity interval 到下一次 idle。
4. bridge 返回该 interval 的最终 committed assistant text、session ID 和有界的状态摘要。
5. Rust 将完整文本严格解析为一个 JSON 对象，再进入现有 Capability 契约校验。

bridge stdout 只允许协议帧；SDK/runtime stderr 作为有界诊断流保存。无效 JSON、未知 invocation、重复完成、越界输出或协议顺序错误均终止 sidecar并使当前 attempt 失败。实现不从说明文字或 Markdown 中宽松提取 JSON。

## Runtime 集成

Runtime 持有一个进程内、可共享的 Provider sidecar pool，以 `(run_id, capability)` 为主键。普通节点与消息流执行通过统一 Provider executor 获取或创建 sidecar；咨询路径显式请求临时 sidecar。

每个 DSH 任务仍使用现有冻结 Context Snapshot、Capability 指令和目标 JSON Schema。bridge 只提供同一 Capability 内的模型对话连续性，不能替代 Archmage 的显式 Handoff、Artifact、Changeset 或输出契约。

SDK runtime 进程由 bridge 的进程组承载。Archmage 登记并校验 bridge PID、PGID 和启动身份：

- Designer、Reviewer 和咨询通过 `DSH_PERMISSION_MODE=read-only` 启动；Coder、Tester 使用 `DSH_PERMISSION_MODE=workspace-write`。Cordis 配置必须同时组合 sandbox policy、受约束的 shell/文件工具和无交互时自动拒绝的 approval 策略，不能只靠提示词声明权限。
- SDK 协议当前没有 prompt 级取消。取消某个活动 attempt 时，Archmage 终止该 Capability 的整个 sidecar 进程组；其他 Capability 的独立 sidecar 不受影响。
- Run 进入任何终态时关闭并回收它的全部 sidecar；回收失败被记录但不能把失败 Run 改写为成功。
- 超时、进程退出和协议损坏均为 Provider 失败，不能发布成功结果或 Changeset。

## 发现与连接验证

“检查 SDK”解析 Python 可执行路径，并在隔离子进程中确认：

- Python 可启动；
- 可以导入 `deepseek_harness`；
- SDK 与 bundled runtime 包可解析到兼容版本；
- Agent 的 Provider route 和 model 配置非空。

“验证连接”会产生一次真实模型调用。Archmage 在临时 Git 工作目录中启动临时只读 bridge，使用配置的 Provider route 和 model，要求返回固定 JSON 对象，然后完整关闭 SDK runtime。成功表示 Python SDK、bundled runtime、凭证、模型路由及 JSONL bridge 当前可用。

更改 Python 路径、Provider route、model 或启用状态会使已有验证状态失效。验证失败只向界面返回不含凭证的安全摘要，原始 stderr 保存在 Archmage 私有诊断目录。

## 智能体与 Capability 管理

- 智能体管理的新增实例操作支持选择 Codex 或 DeepSeek Harness。
- DeepSeek Harness 卡片可编辑实例名称、Python 路径、Provider route、model 和启用状态，并展示 SDK/runtime 版本、连接状态与上次验证时间。
- 页面说明凭证与 endpoint 继承启动 Archmage 时的环境；Provider route 和 model 由实例明确配置。
- 页面指标统计所有受支持 Provider 的实例，不再只统计 Codex。
- Capability 管理允许选择任一已启用的 Codex 或 DeepSeek Harness 实例，并展示真实 Provider 名称。
- Hermes、Claude 和 OpenClaw 保留“尚未支持”卡片，不能创建可运行实例或绑定 Capability。

## 失败与安全边界

Python/SDK 缺失、版本不兼容、初始化失败、模型调用失败、非零退出、超时、取消、输出超限、无效 UTF-8、无效 JSON 和 Capability 契约不匹配分别产生明确失败。诊断目录使用私有权限，不记录 Provider 凭证、完整环境或未经限制的会话内容。

Agent 配置与管理 API 只返回 Python 路径、Provider route、model、版本和安全状态，不回显 API key、endpoint secret 或 `DSH_HOME` 内容。bridge 环境整体继承，但诊断元数据只记录允许字段。

官方 SDK JSON-RPC 协议当前为预发布接口且没有协议版本协商。Archmage 在 bridge 中集中隔离这一依赖，验证已知 server identity/version；不兼容版本明确失败，不把协议细节扩散到 Runtime 调度和 UI。

## 最小验证范围

- bridge 测试使用可控的 Python SDK stub，覆盖启动、同一 named session 的两轮执行、严格结果边界、异常退出和关闭。
- Provider 测试使用可控 JSONL bridge，覆盖发现、真实配置冻结、输出上限、超时和进程组取消。
- Runtime 测试证明同一 `Run + Capability` 复用同一个 sidecar/session，不同 Capability 相互隔离，咨询使用临时 sidecar，消息流与普通执行遵守同一规则。
- 前端测试覆盖 Python 路径、Provider route、model、新增实例、指标统计和 Capability 选择。
- 最后使用已安装的官方 `deepseek-harness-sdk` 运行一次小型真实 smoke Run；若 SDK、bundled runtime 或凭证不可用，明确报告缺失前置条件，不用模拟结果替代。

## 非目标

- 修改、fork 或发布 DeepSeek Harness 仓库；
- Hermes Provider 实现；
- Archmage 管理 DeepSeek 凭证、账号或 `DSH_HOME` 内容；
- DSH 会话跨 Archmage 重启恢复；
- Provider 自动选择、故障转移或负载均衡；
- DSH 实时 token UI、推理过程或工具事件展示；
- 在多个 Capability 之间共享 DSH 对话。
