简体中文 | [English](../../../en/superpowers/plans/2026-10-30-deepseek-harness-provider.md)

# DeepSeek Harness提供商实施计划


> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 将当前实验性的 DeepSeek Harness 无头 CLI 集成替换为 Archmage 拥有的 Python SDK sidecar，以便 `Run + Capability` 为其完整的实时运行保留一个 DSH 对话，并且代理管理可以配置、发现、验证和绑定该提供程序。

**架构：** Archmage 提供了一个小型 JSONL Python 桥加上 Cordis 组合，并通过已发布的 `deepseek-harness-sdk` / 匹配的 `deepseek-harness-runtime-bin` 启动它们。Rust 对每个 `(run_id, capability)` 拥有一个长期存在的桥接进程，使用临时桥进行协商，将最终的辅助文本严格解析为一个 JSON 对象，并在 Run terminal/cancel 边界处销毁所有 sidecar。现有的 Codex 路径保持无状态且不变；本次交付不包含 DeepSeek Harness 存储库更改。

**技术堆栈：** Rust 2024、Tokio process/IO 原语、SQLite/rusqlite、Python 3 + `deepseek-harness-sdk`、JSONL、Cordis YAML、React/TypeScript/Vitest、Cargo 测试。

---

## 范围和替换规则

- 只能在Archmage存储库中工作。此计划不会合并、编辑、重置或删除为被取代的 CLI 设计创建的现有 DSH worktree/branch。
- 就地重写未发货的迁移 `0016_deepseek_harness.sql`。此功能分支尚未发布，因此保留过时的 `provider_sessions` 表或其种子猜测模型代理将产生不必要的兼容性债务。
- 保留已添加的 `ProviderKind::DeepseekHarness`，但用 SDK bridge/pool 模型替换每个无头 CLI 标志、create/resume 租约和持久提供者会话假设。
- 保持验证重点：unit/integration 测试使用受控 bridge/SDK 假货；真正的凭证烟雾是一个可选的最终命令，并且在不满足先决条件时报告为已跳过。

## 任务 1：更正代理架构和冻结配置

**文件：**
- 修改：`crates/archmage-core/migrations/0016_deepseek_harness.sql`
- 修改：`crates/archmage-core/src/agent.rs`
- 修改：`crates/archmage-core/src/store/mod.rs`
- 修改：`crates/archmage-core/src/runtime/mod.rs`
- 修改：`crates/archmage-core/tests/agents.rs`
- 修改：`crates/archmage-core/tests/runtime.rs`

- [ ] **第 1 步：编写失败的架构和验证测试**

添加测试证明：

1. 模式 8 升级保留现有 Codex instances/bindings 并不会自动创建 DSH 实例；
2. `agent_instances` 具有可为空的 `provider_route` 并且没有 `provider_sessions` 表；
3. DSH 输入需要非空白 `provider_route` 和 `model`；
4. Codex 拒绝 `provider_route`，但保留其可选的 `model` 行为；
5. 保存更改的 Python 路径、路由或模型会使验证无效；
6. 编辑全局 Agent 后，frozen Run 绑定会保留已解析的 Python 路径、路线和模型。

在所有新灯具中使用此输入形状：

```rust
AgentInstanceInput {
    id: "dsh-sdk".into(),
    provider_kind: ProviderKind::DeepseekHarness,
    display_name: "DSH SDK".into(),
    executable_path: Some("/usr/bin/python3".into()),
    provider_route: Some("deepseek-official".into()),
    model: Some("deepseek-v4-flash".into()),
    enabled: true,
}
```

- [ ] **步骤 2：运行重点测试并确认红色状态**

运行： `cargo test -p archmage-core --test agents --test runtime`

预期：compile/assertion 失败，因为 `provider_route` 不存在，并且迁移 16 仍播种代理并创建 `provider_sessions`。

- [ ] **步骤 3：重写迁移 16 和 Rust 数据模型**

使用此列顺序使迁移 16 重建 `agent_instances`：

```sql
id, provider_kind, display_name, executable_path, provider_route, model,
enabled, connection_state, discovered_version, last_verified_at, last_error,
verification_thread_id, created_at, updated_at
```

使用 `NULL AS provider_route` 复制现有行，重建 `capability_bindings` 以保留外键，最后仅使用 `INSERT INTO schema_migrations(version) VALUES(16);` 完成。不要播种 DSH 代理，也不要创建 `provider_sessions`。

将 `provider_route: Option<String>` 添加到 `AgentInstanceInput`、`AgentInstance` 和 `FrozenAgentBinding`。验证必须修剪存储的值并强制执行此矩阵：

```rust
match self.provider_kind {
    ProviderKind::Codex if self.provider_route.is_some() => {
        Err(Error::Preflight("Codex does not use a provider route".into()))
    }
    ProviderKind::DeepseekHarness
        if self.provider_route.as_deref().is_none_or(|v| v.trim().is_empty())
            || self.model.as_deref().is_none_or(|v| v.trim().is_empty()) =>
    {
        Err(Error::Preflight(
            "DeepSeek Harness provider route and model are required".into(),
        ))
    }
    _ => Ok(()),
}
```

更新每个代理 SELECT/row 索引并更新插入。仅当 `executable_path`、`provider_route` 和 `model` 均未更改时，才会保留验证状态。将 `FrozenRunDefinition.version` 从 5 提高到 6，并冻结所有三个 DSH 启动值。

- [ ] **步骤 4：删除持久会话租用 API**

删除 `ProviderSessionMode`、`ProviderSessionLease`、`Store::begin_provider_session` 和 `Store::mark_provider_session_ready` 及其测试。会话所有权将转移到任务 4 中的实时 sidecar 池。

- [ ] **步骤 5：重新运行重点测试并提交**

运行： `cargo test -p archmage-core --test agents --test runtime`

预期：通过。

提交： `git add crates/archmage-core && git commit -m "feat: configure DeepSeek Harness SDK agents"`

## 任务2：添加Archmage拥有的SDK桥和Cordis组合

**文件：**
- 创建：`crates/archmage-core/resources/deepseek_harness_bridge.py`
- 创建：`crates/archmage-core/resources/deepseek_harness.cordis.yml`
- 创建：`crates/archmage-core/tests/fixtures/deepseek_harness_sdk/deepseek_harness/__init__.py`
- 创建：`crates/archmage-core/tests/deepseek_harness_bridge.rs`

- [ ] **第 1 步：编写失败的黑盒桥接测试**

该测试启动真实的桥接脚本，其中 `PYTHONPATH` 指向假 `deepseek_harness` 包，发送两个 JSONL `invoke` 帧，然后发送一个 `close` 帧。断言：

- stdout 恰好包含 `ready`、两个匹配的 `result` 帧和 `closed`；
- 两个结果都带有相同的配置会话 ID；
- 伪造的 SDK 日志显示一个 `DeepSeekHarness` 实例和两个有序的 `Session.run()` 调用；
- 无效输入、重复调用 ID、过大的最终文本和 SDK 异常均返回一个有界 `error` 帧并终止非零；
- 协议输出从不包含写入 stderr 的 SDK 诊断信息。

运行： `cargo test -p archmage-core --test deepseek_harness_bridge`

预期：失败，因为 bridge/resources 不存在。

- [ ] **步骤 2：仅使用公共高级 SDK 实现桥接协议**

该桥从 stdin 读取一个启动 JSON 对象：

```json
{"op":"start","protocol":1,"cwd":"/repo","provider":"deepseek-official","model":"deepseek-v4-flash","sessionId":"archmage-...","cordis":"/private/cordis.yml","sessionRoot":"/private/sessions","maxResultBytes":1048576,"requestTimeoutSeconds":900}
```

它必须准确地实例化：

```python
config = DeepSeekHarnessConfig(
    provider=start["provider"],
    model=start["model"],
    cwd=start["cwd"],
    runtime_cwd=start["cwd"],
    session_root=start["sessionRoot"],
    cordis=start["cordis"],
    request_timeout_seconds=start["requestTimeoutSeconds"],
)
harness = DeepSeekHarness(config)
session = harness.start_session(start["sessionId"])
```

对于每个独特的请求：

```json
{"op":"invoke","id":"attempt-or-consultation-id","task":"...","schema":{}}
```

渲染一个包含任务的提示，后跟紧凑模式和指令 `Return exactly one JSON object matching this schema, without Markdown fences.`；调用 `session.run(prompt)`；拒绝非 `completed` 完成原因、空响应、进程边界处的无效 UTF-8 以及通过 `maxResultBytes` 的响应；然后发出：

```json
{"op":"result","id":"...","sessionId":"archmage-...","text":"{...}","finishReason":"completed"}
```

`close` 在 `finally` 中调用 `harness.close()` 并发出 `closed`。在顶层捕获错误，发出没有 environment/credential 值的安全 `{op:"error", id, code, message}` 帧，然后以非零值退出。在发出 `ready` 之前，使用 `importlib.metadata.version()` 验证 `deepseek-harness-sdk` 和 `deepseek-harness-runtime-bin` 是否存在且版本完全匹配。

- [ ] **步骤 3：添加无人值守的 Cordis 组合物**

该资源基于官方 JSON-RPC 和 ACP 示例证明的运行时轮插件。包括：

- `@deepseek-ai/dsh-sdk-jsonrpc-server`；
- `@deepseek-ai/dsh-llm-deepseek`；
- `@deepseek-ai/dsh-sandbox-local` 和 `@deepseek-ai/dsh-sandbox-policy`，以及 `mode: !!js process.env.DSH_PERMISSION_MODE` 和工作空间根 `DSH_CWD`；
- 本地子进程加上沙盒 Bash；
- `@deepseek-ai/dsh-user-approval` 与 `policy: never`；
- 禁用后台作业和交互式问题的代理脊柱；
- JSONL 持久化和检查点策略；
- 沙盒文件系统、观察策略和文件系统工具；
- 令牌计和有界压缩。

可能不存在控制台记录器、终端 UI、问题工具、遥测导出器或交互式审批应答器，因为 stdout 属于 JSONL 并且 Archmage 执行是无人值守的。

- [ ] **步骤 4：制作假 SDK 断言应用的配置**

假包必须公开上面使用的相同构造函数表面，并记录提供程序、模型、cwd、cordis、会话根、会话 ID、提示和关闭计数到 `FAKE_DSH_LOG` 提供的仅测试路径。它还必须通过非秘密测试变量支持确定性 exception/oversize/finish-reason 模式。

- [ ] **第5步：运行并提交**

运行： `cargo test -p archmage-core --test deepseek_harness_bridge`

预期：通过。

提交： `git add crates/archmage-core/resources crates/archmage-core/tests && git commit -m "feat: add DeepSeek Harness SDK bridge"`

## 任务 3：构建持久的 Rust sidecar 传输

**文件：**
- 修改：`crates/archmage-core/src/process.rs`
- 替换： `crates/archmage-core/src/provider/deepseek_harness.rs`
- 替换： `crates/archmage-core/tests/deepseek_harness_protocol.rs`
- 修改：`crates/archmage-core/tests/provider_execution.rs`

- [ ] **步骤 1：使用假 JSONL 桥编写失败的传输测试**

涵盖就绪握手、一个进程上的两次连续调用、调用 ID 匹配、stdout line/result 限制、超时、malformed/duplicate 帧、提前退出、有界 stderr 捕获、优雅关闭和进程组终止。假桥必须记录其 PID，以便测试可以证明已取消的边车已经消失。

运行： `cargo test -p archmage-core --test deepseek_harness_protocol --test provider_execution`

预期：失败，因为当前适配器每次调用都会启动一个无头 CLI 进程。

- [ ] **步骤 2：添加拥有的管道进程原语**

使用 `spawn_owned_piped(binary, args, cwd, env)` 扩展 `process.rs`，返回一个类型，该类型拥有 `ChildStdin`、行缓冲 `ChildStdout`、后台有界 stderr 漏极、`ProcessIdentity`、`try_wait`、`wait` 和现有的经过身份检查的 TERM/KILL 梯子。对于 Codex 和现有测试，保持 `spawn_owned` 和有界一次性捕获行为不变。

不要跨异步 reads/writes 持有阻塞互斥体。sidecar 由提供者池层的一个 Tokio 互斥体序列化。

- [ ] **步骤 3：用 SDK sidecar 替换 CLI 适配器**

定义这些核心类型：

```rust
pub struct SdkInspection {
    pub python_binary: PathBuf,
    pub sdk_version: String,
    pub runtime_version: String,
}

pub struct SidecarConfig {
    pub python_binary: PathBuf,
    pub repository: PathBuf,
    pub provider_route: String,
    pub model: String,
    pub session_id: String,
    pub permission_mode: PermissionMode,
    pub session_root: PathBuf,
    pub diagnostics_dir: PathBuf,
    pub deadline: Duration,
    pub max_line_bytes: usize,
    pub max_stream_bytes: usize,
    pub max_result_bytes: usize,
}

pub struct DeepseekHarnessSidecar { /* process, framed IO, config, poisoned */ }
```

`DeepseekHarnessSidecar::start` 以仅限所有者的权限实现 `<data_root>/provider-resources/deepseek-harness/<sha256>/` 下嵌入的 Python/YAML 资源，启动 `<python> -u bridge.py`，继承当前环境（包括 `DSH_HOME` 和提供者凭据），并仅覆盖 `DSH_PERMISSION_MODE`。它发送 `start`，等待恰好一个兼容的 `ready`，并公开其进程身份。

`invoke(id, prompt, schema)` 写入一帧，在截止日期内准确读取匹配的终端帧，验证 byte/line 边界，严格运行 `serde_json::from_str::<serde_json::Value>(&text)`，并返回 `ProviderOutcome { conversation_id: session_id, result }`。任何 timeout/protocol/exit 错误都标志着 sidecar 中毒；稍后的调用将返回原始的稳定故障而不重生。

- [ ] **步骤 4：简化公共执行器边界**

删除 `ProviderSession`、`ProviderLifecycle::ConversationPersisted` 和所有旧的 DSH CLI 调用构造。在 `executor.rs` 中保留 Codex 一次性分支；公开提供者中立的结果和进程生成观察者，而 DSH 调用由任务 4 中的池提供已拥有的 sidecar。

- [ ] **第5步：运行并提交**

运行： `cargo test -p archmage-core --test deepseek_harness_protocol --test provider_execution`

预期：通过。

提交： `git add crates/archmage-core/src crates/archmage-core/tests && git commit -m "feat: run DeepSeek Harness through an SDK sidecar"`

## 任务 4：添加 `(Run, Capability)` sidecar 池

**文件：**
- 创建：`crates/archmage-core/src/provider/pool.rs`
- 修改：`crates/archmage-core/src/provider/mod.rs`
- 修改：`crates/archmage-core/src/runtime/mod.rs`
- 创建：`crates/archmage-core/tests/provider_pool.rs`

- [ ] **第 1 步：编写失败的池测试**

使用可配置的假桥并断言：

- 对 `(run-a, coder)` 的两次调用使用一个 PID/session；
- `(run-a, reviewer)` 和 `(run-b, coder)` 使用不同的 PID/session 值；
- 协商使用新的临时 sidecar，并且不会更改目标的主要 sidecar；
- 中毒的钥匙永远不会重生；
- `close_run("run-a")`仅关闭run-a sidecar；
- `cancel_capability("run-a", "coder")` 仅杀死编码员并毒害该密钥。

运行： `cargo test -p archmage-core --test provider_pool`

预期：失败，因为不存在池。

- [ ] **步骤 2：在没有全局序列化的情况下实现池所有权**

用途：

```rust
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct SidecarKey { pub run_id: String, pub capability: String }

pub struct ProviderPool {
    entries: tokio::sync::Mutex<HashMap<SidecarKey, Arc<tokio::sync::Mutex<PoolEntry>>>>,
}
```

地图锁仅针对 lookup/insert/remove 持有；每个 sidecar 锁都会序列化该能力的轮次。`invoke_main` 延迟创建稳定的 `archmage-<uuid>` 会话并记录 `Live(sidecar)` 或 `Poisoned(error)`。`invoke_temporary` 在最终样式路径中使用 `archmage-consult-<uuid>` 并始终使用 closes/kills。`close_run` 在等待关闭之前删除条目，因此新的调用者无法获取关闭条目。

- [ ] **第 3 步：将池置于运行时**

更改 `Runtime` 以将 `Arc<ProviderPool>` 与 `Config` 一起保存，在 `Runtime::open` 中对其进行初始化，并保持 `Clone` 共享同一池。通过 `Config` 添加仅测试构造函数或桥重写，而不是全局环境突变，以便并行测试保持确定性。

- [ ] **第 4 步：运行并提交**

运行： `cargo test -p archmage-core --test provider_pool`

预期：通过。

提交： `git add crates/archmage-core && git commit -m "feat: retain DSH conversations per run capability"`

## 任务5：路由正常、流量、咨询、取消和恢复路径

**文件：**
- 修改：`crates/archmage-core/src/runtime/dispatch.rs`
- 修改：`crates/archmage-core/src/runtime/flow.rs`
- 修改：`crates/archmage-core/src/runtime/consultation.rs`
- 修改：`crates/archmage-core/src/runtime/recovery.rs`
- 修改：`crates/archmage-core/src/runtime/mod.rs`
- 修改：`crates/archmage-core/tests/runtime.rs`
- 修改：`crates/archmage-core/tests/flow.rs`
- 修改：`crates/archmage-core/tests/consultations.rs`

- [ ] **步骤 1：添加失败的运行时连续性测试**

仅添加四种高价值场景：

1. 审阅者反馈重新打开编码器，并且编码器的假桥日志显示在一个会话上有两个有序轮次；
2. 重新访问相同功能的流交付重用其 sidecar；
3. 协商得到一个临时会话，稍后的主要目标轮流使用其未触及的主要会话；
4. cancel/terminal 完成后不会留下任何记录的假桥 PID。

运行： `cargo test -p archmage-core --test runtime --test flow --test consultations`

预期：失败，直到所有运行时路径都使用该池并且集中进行终端清理。

- [ ] **步骤 2：通过池路由 DSH 执行**

替换 `main_provider_session(...)` 和旧的提供商会话租用调用。从冻结的绑定（`executable_path`、`provider_route`、`model`）、工作空间、架构、限制和功能权限构建 `SidecarConfig`：

```rust
let permission = match kind {
    CapabilityKind::Designer | CapabilityKind::Reviewer => PermissionMode::ReadOnly,
    CapabilityKind::Coder | CapabilityKind::Tester => PermissionMode::WorkspaceWrite,
};
```

正常路径和流路径调用 `pool.invoke_main(SidecarKey { run_id, capability }, ...)`。咨询调用 `pool.invoke_temporary(...)` 具有只读权限。法典通过现有的执行者继续进行。将 DSH 桥 `ProcessIdentity` 连接到活动 attempt/consultation 记录，以便现有恢复可以在崩溃后终止孤立进程组。

- [ ] **第 3 步：集中实时运行清理**

包装传统终端驱动程序和流终端驱动程序，以便每个返回路径都执行 `pool.close_run(run_id).await`。在显式取消、请求取消时，调用 `pool.cancel_run(run_id).await`，终止任何数据库记录的孤立身份作为后备，然后应用取消转换。清理错误被记录为诊断，并且可能会将原本成功的提供程序清理变成报告的基础设施故障，但永远不会将 failed/cancelled 运行成功。

`reconcile_interrupted` 保持重新启动安全：重新启动后内存池为空，因此它会杀死持久的活动 attempt/consultation 身份并将运行标记为已中断。不要在 Archmage 重新启动时声明或实施 DSH 会话恢复。

- [ ] **第 4 步：运行并提交**

运行： `cargo test -p archmage-core --test runtime --test flow --test consultations`

预期：通过。

提交： `git add crates/archmage-core && git commit -m "feat: route Archmage runs through persistent DSH sidecars"`

## 任务 6：用 SDK 检查替换发现和验证

**文件：**
- 修改：`crates/archmage-core/src/agent.rs`
- 修改：`crates/archmage-core/src/provider/deepseek_harness.rs`
- 修改：`crates/archmage-core/tests/agents.rs`

- [ ] **第 1 步：编写失败的管理测试**

测试默认 Python 分辨率 (`python3`)、显式 Python 路径、导入失败、缺少运行时分发、SDK/runtime 版本不匹配、非空白 route/model 验证以及执行一次临时只读模型转动并关闭进程的成功验证。

运行： `cargo test -p archmage-core --test agents`

预期：失败，因为发现仍然检查 DSH CLI 并且验证使用无头标志。

- [ ] **步骤 2：实施安全 SDK 检查**

解析配置的可执行文件或 `python3`，然后运行相当于以下内容的有界探测：

```python
import json
from importlib.metadata import version
import deepseek_harness
sdk = version("deepseek-harness-sdk")
runtime = version("deepseek-harness-runtime-bin")
print(json.dumps({"sdkVersion": sdk, "runtimeVersion": runtime}))
```

要求精确的版本相等。返回组合显示版本，例如 `SDK 0.4.0 / runtime 0.4.0`，并在保留错误之前清理所有错误。

- [ ] **第三步：通过真实的桥边界进行验证**

`AgentService::verify` 创建一个私有临时 Git 目录，使用配置的 route/model 启动临时只读 sidecar，请求固定的微型模式（例如 `{"ok": boolean}`），需要 `{"ok": true}`，关闭 sidecar，并存储 `Ready { version, thread_id: session_id }`。它不得读取、重写或显示 `DSH_HOME` 或凭据。

- [ ] **第4步：运行并提交**

运行： `cargo test -p archmage-core --test agents`

预期：通过。

提交： `git add crates/archmage-core && git commit -m "feat: discover and verify DeepSeek Harness SDK agents"`

## 任务 7：更新代理管理 UI 和 API 类型

**文件：**
- 修改：`apps/archmage-desktop/src/model.ts`
- 修改：`apps/archmage-desktop/src/AgentManagement.tsx`
- 修改：`apps/archmage-desktop/src/AgentManagement.test.tsx`
- 修改：`apps/archmage-desktop/src/model.test.ts`
- 修改：`apps/archmage-desktop/src/managementLayout.test.ts`

- [ ] **步骤 1：编写失败的 UI/model 测试**

断言 DSH 代理携带 `providerRoute`，create/edit 表格显示 `Python 路径`、`Provider route` 和 `Model`、route/model 对于 DSH 是必需的，模型对于 Codex 仍然是可选的，解释性副本表示credentials/endpoints 继承了 Archmage 的环境，而 route/model 是显式的。

从 `apps/archmage-desktop` 运行：`npm test -- --run AgentManagement.test.tsx model.test.ts managementLayout.test.ts`。

预期：失败，因为当前表单隐藏 DSH 模型并描述继承的 `DSH_HOME` 模型选择。

- [ ] **第 2 步：更新类型和表格**

将 `providerRoute: string | null` 添加到 `AgentInstance`、`AgentInstanceInput` 和 `FrozenAgentBinding`。对于 DSH，默认 Python 路径为空白（后端解析 `python3`）并提交修剪后的非空白 route/model。对于 Codex，请提交 `providerRoute: null`。保持不受支持的提供商卡不可创建。

显示发现的 SDK/runtime 版本和现有连接状态字段，而不显示环境值或凭证材料。

- [ ] **步骤 3：运行 build/tests 并提交**

从 `apps/archmage-desktop` 运行：

```bash
npm test
npm run build
```

预期：通过。

提交： `git add apps/archmage-desktop && git commit -m "feat: configure DSH SDK agents in the desktop UI"`

## 任务 8：替换过时的文档并运行紧凑的验收套件

**文件：**
- 修改：`README.md`
- 修改：`docs/runbooks/desktop-s0.md`
- Modify/Delete：`crates/archmage-core/tests/` 下过时的无头 CLI 测试夹具
- 评论： `docs/superpowers/specs/2026-10-30-deepseek-harness-provider-design.md`

- [ ] **第 1 步：删除过时的 CLI 语言和固定装置**

删除 tests/fixtures，其唯一目的是声明 `--headless`、`--session-id` 或 `--resume-session`。更新 Runbook 以记录：

- 将匹配的 `deepseek-harness-sdk` / 运行时轮安装到已配置的 Python 中；
- 使用 Python 路径、路由和模型创建 DSH 代理；
- 继承环境/`DSH_HOME` 行为；
- live 运行连续性范围且无交叉重启恢复；
- 只读与工作区写入能力权限；
- 一个可选的真实 verification/smoke 程序。

明确声明不需要或交付任何 DeepSeek Harness 存储库补丁。不要 delete/reset 单独的被取代的 DSH 分支；将清理工作留给明确的用户决定。

- [ ] **第 2 步：运行格式化和重点整套**

运行：

```bash
cargo fmt --all -- --check
cargo test -p archmage-core
(cd apps/archmage-desktop && npm test && npm run build)
```

预期：通过。

- [ ] **第 3 步：运行一个可选的真实 SDK 烟雾**

如果所选的 Python 可以导入匹配的包并且存在所需的提供程序凭据，则创建一个临时 Agent/config 并使用一个现有功能架构运行一项微小验证和两轮运行。否则记录精确缺失的先决条件，并仅将真实烟雾标记为已跳过；不要用假结果代替。

- [ ] **步骤 4：检查最终差异的范围和秘密**

运行：

```bash
git diff --check
git status --short
git diff --stat HEAD~8..HEAD
rg -n -- "--headless|--resume-session|provider_sessions|DEEPSEEK_API_KEY=|api[_-]?key" crates apps README.md docs
```

预期：没有过时的运行时依赖项，没有凭证值，并且仅更改了预期的Archmage文件。

- [ ] **步骤 5：提交 documentation/cleanup**

提交： `git add README.md docs crates/archmage-core/tests && git commit -m "docs: document the DeepSeek Harness SDK provider"`

## 最终验收清单

- [ ] 代理管理可以使用显式 Python/route/model 创建、发现、验证、编辑和绑定 DSH SDK 代理。
- [ ] 同一 `Run + Capability` 在重试、审阅者反馈和流程重新访问中重复使用一个网桥 PID 和 DSH 会话。
- [ ] 不同的 Run/Capability 密钥和协商是隔离的。
- [ ] 取消、失败、完成和中断不会导致任何拥有的 sidecar 进程组运行。
- [ ] DSH 最终文本仅作为满足现有功能契约的有效 JSON 对象而被接受。
- [ ] Cordis 强制执行 read-only/workspace-write 和自动批准拒绝，无需交互式 UI。
- [ ] Codex 行为和现有的 Run 持久性保持绿色。
- [ ] 交付时无需更改 DSH 存储库。
