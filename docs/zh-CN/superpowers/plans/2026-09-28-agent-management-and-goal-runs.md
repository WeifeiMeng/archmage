简体中文 | [English](../../../en/superpowers/plans/2026-09-28-agent-management-and-goal-runs.md)

# 代理管理和目标驱动的运行实施计划


> **面向执行 Agent：** 必需子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 添加具有持久功能绑定的真正本地 Codex Agent 管理页面，并让用户仅使用存储库和目标即可启动运行，同时测试人员自主执行验证。

**架构：** `archmage-core` 拥有持久的代理实例注册表、全局功能绑定、真正的 Codex discovery/probing 和不可变的每次运行绑定快照。Tauri 将键入的命令暴露给 React；工作台添加了代理目标并从新运行中删除了预先声明的检查，而无需删除旧的 CheckRunner 基础设施。

**技术堆栈：** Rust 1.95、Tokio、rusqlite/SQLite、真正的 Codex CLI、Tauri 2、React 19、TypeScript、Vite、Vitest。

---

## 文件结构

```text
crates/archmage-core/
  migrations/0008_agents.sql            # Agent instances and global bindings
  src/agent.rs                          # Agent DTOs, validation, discovery and verification
  src/provider/codex.rs                 # configured executable/model support
  src/runtime/mod.rs                    # goal-only RunRequest and frozen bindings
  src/runtime/dispatch.rs               # per-node frozen Agent dispatch; autonomous Tester
  src/contract.rs                       # Tester v2 and Reviewer evidence rules
  capabilities/v1/tester.md             # autonomous test task prompt
  capabilities/v1/tester.schema.json    # command/outcome evidence contract
  capabilities/v1/reviewer.md           # consume autonomous Tester evidence
  src/store/mod.rs                      # Agent/binding persistence queries
  tests/agents.rs                       # migration, multi-instance and binding tests
  tests/runtime.rs                      # goal-only and frozen binding tests
  tests/contracts.rs                    # Tester/Reviewer contract tests
  tests/codex_live.rs                   # ignored real verification path
apps/archmage-desktop/
  src-tauri/src/lib.rs                  # Agent management Tauri commands
  src/model.ts                          # goal-only Run and Agent DTOs
  src/api.ts                            # typed Agent invokes
  src/AgentManagement.tsx               # connections and capability routing page
  src/App.tsx                           # Runs/Agents navigation and simplified form
  src/styles.css                        # Agent page and navigation styles
  src/model.test.ts                     # request and status presentation tests
docs/guide.md                           # Agent setup and Run usage
docs/runbooks/desktop-s0.md             # real connection verification steps
```

### 任务 1：保留 Agent 实例和功能绑定

**文件：**
- 创建：`crates/archmage-core/migrations/0008_agents.sql`
- 创建：`crates/archmage-core/src/agent.rs`
- 创建：`crates/archmage-core/tests/agents.rs`
- 修改：`crates/archmage-core/src/lib.rs`
- 修改：`crates/archmage-core/src/store/mod.rs`

- [ ] **第 1 步：编写失败的迁移和存储测试**

```rust
#[test]
fn seeds_one_codex_instance_and_binds_all_builtin_capabilities() {
    let store = fixture_store();
    let snapshot = store.agent_management_snapshot().unwrap();
    assert_eq!(snapshot.instances.len(), 1);
    assert_eq!(snapshot.instances[0].id, "codex-default");
    assert_eq!(snapshot.bindings.len(), 4);
    assert!(snapshot.bindings.iter().all(|binding|
        binding.agent_instance_id == "codex-default"));
}

#[test]
fn stores_multiple_codex_instances_with_stable_ids() {
    let mut store = fixture_store();
    store.save_agent_instance(&codex_input("codex-work", "Codex Work")).unwrap();
    assert_eq!(store.agent_instances().unwrap().len(), 2);
}
```

- [ ] **第 2 步：运行 RED**

运行： `cargo test -p archmage-core --test agents`

预期：编译失败，因为迁移 v8 和代理存储 API 不存在。

- [ ] **步骤 3：添加迁移和键入记录**

使用 `id`、`provider_kind`、`display_name`、可空 `executable_path` 和 `model`、`enabled`、连接字段、时间戳和 JSON 安全约束创建 `agent_instances`。创建由四个内置功能键键入的 `capability_bindings`，并引用 `agent_instances(id)`。种子 `codex-default` 以及迁移中的四个绑定。

在 `agent.rs` 中定义这些公共类型：

```rust
pub enum ProviderKind { Codex, Claude, OpenClaw, Hermes }
pub enum ConnectionState { Unverified, Ready, Failed }
pub struct AgentInstance { /* persisted safe fields */ }
pub struct CapabilityBinding { pub capability: String, pub agent_instance_id: String }
pub struct AdapterDescriptor { pub kind: ProviderKind, pub supported: bool }
pub struct AgentManagementSnapshot { pub adapters: Vec<AdapterDescriptor>, pub instances: Vec<AgentInstance>, pub bindings: Vec<CapabilityBinding> }
```

更新存储迁移顺序以需要架构版本 8 并添加 CRUD/query 方法。拒绝绑定未知、已禁用或不受支持的实例。拒绝禁用当前绑定的实例。

- [ ] **第 4 步：运行绿色**

运行： `cargo test -p archmage-core --test agents`

预期：两项测试均通过。

- [ ] **第 5 步：承诺**

```bash
git add crates/archmage-core/migrations/0008_agents.sql crates/archmage-core/src/agent.rs crates/archmage-core/src/lib.rs crates/archmage-core/src/store/mod.rs crates/archmage-core/tests/agents.rs
git commit -m "feat: persist agent instances and bindings"
```

### 任务 2：配置并验证真实的 Codex 实例

**文件：**
- 修改：`crates/archmage-core/src/provider/codex.rs`
- 修改：`crates/archmage-core/src/agent.rs`
- 修改：`crates/archmage-core/tests/agents.rs`
- 修改：`crates/archmage-core/tests/codex_protocol.rs`
- 修改：`crates/archmage-core/tests/codex_live.rs`

- [ ] **步骤 1：编写失败的可执行解析测试**

```rust
#[tokio::test]
async fn discovery_uses_the_instances_explicit_executable() {
    let binary = fixture_codex_binary("codex 1.2.3", valid_exec_help());
    let tools = codex::inspect_at(Some(&binary)).await.unwrap();
    assert_eq!(tools.codex_binary, binary);
    assert_eq!(tools.codex_version, "codex 1.2.3");
}

#[test]
fn changing_execution_config_invalidates_verification() {
    let mut store = fixture_store();
    store.record_agent_ready("codex-default", "codex 1.2.3", "thread-1").unwrap();
    store.save_agent_instance(&changed_path_input()).unwrap();
    assert_eq!(store.agent_instance("codex-default").unwrap().connection_state,
               ConnectionState::Unverified);
}
```

- [ ] **第 2 步：运行 RED**

运行： `cargo test -p archmage-core --test agents --test codex_protocol`

预期：失败，因为配置的发现和持久验证不存在。

- [ ] **第3步：实施发现和真实验证**

添加 `codex::inspect_at(Option<&Path>)`；验证显式可执行文件而不进行 PATH 查找，并将现有 `inspect()` 保留为默认包装器。将可选模型添加到 `CodexInvocation` 并仅在配置时发出 `--model <value>`。

实施 `AgentService::discover(id)` 和 `AgentService::verify(id)`。`verify` 运行 `readonly_probe_logged`，使用清理后的消息存储 ready/failed 状态，并返回刷新的管理快照。不受支持的提供程序类型会返回类型化的预检错误并且从不执​​行回退。

- [ ] **第 4 步：运行 GREEN 并选择加入真实探测**

运行： `cargo test -p archmage-core --test agents --test codex_protocol`

预期：所有非网络测试均通过。

当本地 Codex 身份验证可用时运行：`cargo test -p archmage-core --test codex_live real_readonly_probe -- --ignored --nocapture`

预期：一个被忽略的真实探针测试通过非空线程 ID。

- [ ] **第 5 步：承诺**

```bash
git add crates/archmage-core/src/provider/codex.rs crates/archmage-core/src/agent.rs crates/archmage-core/tests/agents.rs crates/archmage-core/tests/codex_protocol.rs crates/archmage-core/tests/codex_live.rs
git commit -m "feat: verify configured Codex agents"
```

### 任务 3：将全局功能绑定冻结为仅目标运行

**文件：**
- 修改：`crates/archmage-core/src/runtime/mod.rs`
- 修改：`crates/archmage-core/src/runtime/dispatch.rs`
- 修改：`crates/archmage-core/tests/runtime.rs`

- [ ] **第 1 步：编写失败的仅目标测试和冻结绑定测试**

```rust
#[test]
fn repository_and_goal_are_sufficient_to_define_a_run() {
    let request = RunRequest::new(repo(), "add health endpoint").unwrap();
    let run_id = runtime().create_run_definition(&request).unwrap();
    assert!(runtime().snapshot(&run_id).unwrap().agent_bindings.len() == 4);
}

#[test]
fn changing_global_binding_does_not_change_existing_run() {
    let run_id = runtime().create_run_definition(&RunRequest::new(repo(), "goal").unwrap()).unwrap();
    bind("tester", "codex-work");
    assert_eq!(runtime().snapshot(&run_id).unwrap().agent_bindings["tester"].agent_instance_id,
               "codex-default");
}
```

- [ ] **第 2 步：运行 RED**

运行： `cargo test -p archmage-core --test runtime repository_and_goal_are_sufficient_to_define_a_run changing_global_binding_does_not_change_existing_run`

预期：编译失败，因为 `RunRequest` 仍需要检查并且快照缺少代理绑定。

- [ ] **步骤 3：实施新的 Run 定义**

从新的 `RunRequest` 和 `RunSnapshot` 中删除 `checks`。将 `FrozenRunDefinition.checks` 替换为 `agent_bindings: BTreeMap<String, FrozenAgentBinding>`。在持续运行之前解析并验证所有四个绑定。通过忽略该字段，为包含旧版 `checks` 的定义保持向后兼容的反序列化。

更改调度以从每个节点的冻结绑定解析 Codex 工具和模型，而不是为整个运行调用一个全局 `codex::inspect()`。

- [ ] **第 4 步：运行绿色**

运行： `cargo test -p archmage-core --test runtime`

预期：所有运行时测试都通过仅目标请求。

- [ ] **第 5 步：承诺**

```bash
git add crates/archmage-core/src/runtime/mod.rs crates/archmage-core/src/runtime/dispatch.rs crates/archmage-core/tests/runtime.rs
git commit -m "feat: freeze agent bindings into goal driven runs"
```

### 任务 4：给测试人员一个自主任务，而不是冻结检查

**文件：**
- 修改：`crates/archmage-core/capabilities/v1/tester.md`
- 修改：`crates/archmage-core/capabilities/v1/tester.schema.json`
- 修改：`crates/archmage-core/capabilities/v1/reviewer.md`
- 修改：`crates/archmage-core/src/contract.rs`
- 修改：`crates/archmage-core/src/runtime/dispatch.rs`
- 修改：`crates/archmage-core/tests/contracts.rs`
- 修改：`crates/archmage-core/tests/runtime.rs`

- [ ] **第 1 步：编写失败的测试器合约测试**

```rust
#[test]
fn tester_reports_commands_it_selected_without_required_check_ids() {
    let output = json!({
        "schema_version": "1",
        "summary_markdown": "Tests passed",
        "status": "passed",
        "commands": [{"command":"cargo test","exit_code":0,"status":"passed","summary":"12 passed"}],
        "limitations": []
    });
    assert!(validate_output(CapabilityKind::Tester, output.to_string().as_bytes(), &Default::default()).is_ok());
}

#[test]
fn reviewer_cannot_approve_failed_or_blocked_testing() {
    let context = ContractContext { tester_accepted: false };
    assert!(validate_output(CapabilityKind::Reviewer, approved_review(), &context).is_err());
}
```

- [ ] **第 2 步：运行 RED**

运行： `cargo test -p archmage-core --test contracts tester_reports_commands_it_selected_without_required_check_ids reviewer_cannot_approve_failed_or_blocked_testing`

预期：测试器架构拒绝自主命令结果并且 `tester_accepted` 不存在。

- [ ] **步骤 3：实施自主测试人员调度**

更新测试仪提示和架构以要求 `status`、选择 `commands` 和 `limitations`。从内置调度中删除 `invoke_tester`、Unix 套接字注入、冻结检查查找和必需检查合约汇编。通过正常的工作区写入 Codex 路径调用 Tester。

将 `ContractContext.required_checks/checks_failed` 替换为 `tester_accepted` 以进行审阅者验证。从持久的测试器结果状态中得出它。保持 `check_runner.rs`、内部 CLI 帮助程序和迁移 v5 作为休眠的较低级别基础设施保持不变。

- [ ] **第 4 步：运行绿色**

运行： `cargo test -p archmage-core --test contracts --test runtime --test checks`

预期：合同和运行时通行证；旧版 CheckRunner 单元测试保持绿色。

- [ ] **第 5 步：承诺**

```bash
git add crates/archmage-core/capabilities/v1/tester.md crates/archmage-core/capabilities/v1/tester.schema.json crates/archmage-core/capabilities/v1/reviewer.md crates/archmage-core/src/contract.rs crates/archmage-core/src/runtime/dispatch.rs crates/archmage-core/tests/contracts.rs crates/archmage-core/tests/runtime.rs
git commit -m "feat: delegate testing to the bound agent"
```

### 任务 5：通过 Tauri 公开 Agent 管理

**文件：**
- 修改：`apps/archmage-desktop/src-tauri/src/lib.rs`
- 修改：`apps/archmage-desktop/src/api.ts`
- 修改：`apps/archmage-desktop/src/model.ts`

- [ ] **第 1 步：在命令之前添加类型化的前端合约**

```ts
export type AgentManagementSnapshot = {
  adapters: AdapterDescriptor[];
  instances: AgentInstance[];
  bindings: CapabilityBinding[];
};

export type RunRequest = { repository: string; goal: string };
```

运行：`apps/archmage-desktop` 中的 `npm test -- --run`。

预期：现有请求序列化测试失败，因为它需要检查。

- [ ] **第 2 步：添加 Tauri 命令**

公开 `get_agent_management`、`save_agent_instance`、`verify_agent_instance` 和 `bind_capability`。命令仅调用 Core，返回安全 DTO，并且从不公开探针 stderr。更改 `start_run` 以接受仅目标请求。

- [ ] **步骤 3：添加类型化 API 方法**

```ts
getAgentManagement: () => invoke<AgentManagementSnapshot>("get_agent_management"),
saveAgentInstance: (input) => invoke<AgentManagementSnapshot>("save_agent_instance", { input }),
verifyAgentInstance: (agentInstanceId) => invoke<AgentManagementSnapshot>("verify_agent_instance", { agentInstanceId }),
bindCapability: (capability, agentInstanceId) => invoke<AgentManagementSnapshot>("bind_capability", { capability, agentInstanceId }),
```

- [ ] **第四步：编译**

运行： `cargo check -p archmage-desktop`

预期：Tauri 命令层编译。

- [ ] **第 5 步：承诺**

```bash
git add apps/archmage-desktop/src-tauri/src/lib.rs apps/archmage-desktop/src/api.ts apps/archmage-desktop/src/model.ts
git commit -m "feat: expose agent management to desktop"
```

### 任务6：构建Agent管理页面并简化New Run

**文件：**
- Create: `apps/archmage-desktop/src/AgentManagement.tsx`
- 修改：`apps/archmage-desktop/src/App.tsx`
- 修改：`apps/archmage-desktop/src/model.ts`
- 修改：`apps/archmage-desktop/src/model.test.ts`
- 修改：`apps/archmage-desktop/src/styles.css`

- [ ] **第 1 步：编写失败的模型测试**

```ts
it("serializes a run from repository and goal only", () => {
  expect(toRunRequest(" /repo ", " goal ")).toEqual({ repository: "/repo", goal: "goal" });
});

it("does not call an unverified Codex connection ready", () => {
  expect(agentStatusLabel({ connectionState: "unverified" })).toBe("连接未验证");
});
```

- [ ] **第 2 步：运行 RED**

运行：`apps/archmage-desktop` 中的 `npm test -- --run`。

预期：请求签名和状态帮助程序测试失败。

- [ ] **步骤 3：实现纯模型行为**

从前端运行 DTO 中删除 CheckSpec。实施仅目标 `toRunRequest`、安全连接标签以及总结功能绑定准备情况的帮助程序。

- [ ] **步骤 4：运行 GREEN 进行模型测试**

运行：`apps/archmage-desktop` 中的 `npm test -- --run`。

预期：前端测试通过。

- [ ] **Step 5: Implement the page**

Add Runs/Agents sidebar navigation.使用 Codex 配置卡、在线验证操作、不支持的适配器卡、功能绑定选择器和高级附加 Codex 实例表单构建 `AgentManagement`。从“新运行”中删除检查命令状态和控件。Show frozen Agent identity on each Run node.

- [ ] **Step 6: Build and commit**

运行：`apps/archmage-desktop` 中的 `npm run build`。

预期：TypeScript 和 Vite 完成，退出代码为 0。

```bash
git add apps/archmage-desktop/src/AgentManagement.tsx apps/archmage-desktop/src/App.tsx apps/archmage-desktop/src/model.ts apps/archmage-desktop/src/model.test.ts apps/archmage-desktop/src/styles.css
git commit -m "feat: add local Agent management page"
```

### 任务 7：更新使用文档并运行最终验证

**文件：**
- 修改：`docs/guide.md`
- 修改：`docs/runbooks/desktop-s0.md`
- 修改：`docs/architecture/runtime-and-state.md`

- [ ] **第 1 步：记录真实的工作流程**

记录 `./scripts/dev-desktop.sh`，打开代理，验证真实的 Codex 连接，读取功能绑定，添加另一个 Codex 实例，并开始使用存储库和目标运行。明确声明 Claude、OpenClaw 和 Hermes 未实现，测试人员选择自己的验证命令。

- [ ] **第 2 步：运行重点验证**

运行：

```bash
cargo test -p archmage-core --test agents --test runtime --test contracts --test codex_protocol --test checks
(cd apps/archmage-desktop && npm test -- --run && npm run build)
cargo check -p archmage-desktop
```

预期：所有命令退出 0。

- [ ] **步骤 3：运行工作区回归验证**

运行： `cargo test --workspace`

预期：所有不可忽略的工作区测试均通过。

- [ ] **步骤 4：当凭证可用时运行真正的 Codex 验证**

运行： `cargo test -p archmage-core --test codex_live real_readonly_probe -- --ignored --nocapture`

预期：实际探测以退出代码 0 和非空线程 ID 完成。如果无法提供，请单独报告，而不是要求在线验证。

- [ ] **步骤 5：根据批准的设计和提交文档进行自我审查**

确认运行没有检查命令输入，所有四个绑定都被冻结，不受支持的提供程序是真实的，不存在每次运行覆盖，并且没有添加Fake Agent。

```bash
git add docs/guide.md docs/runbooks/desktop-s0.md docs/architecture/runtime-and-state.md docs/superpowers/specs/2026-09-28-agent-management-and-goal-runs-design.md docs/superpowers/plans/2026-09-28-agent-management-and-goal-runs.md
git commit -m "docs: explain Agent management and goal driven runs"
```
