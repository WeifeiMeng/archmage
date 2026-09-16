# Agent Collaboration Desktop MVP

## 里程碑与需求拆分 v0.1

## 1. 项目目标

构建一个 **Local-first 的多 Agent 协作桌面应用**。

产品核心不是提供新的 AI Agent，而是将已有 Agent 的能力组织起来，使多个 Agent 可以围绕同一个代码 Workspace 自动完成协作。

首个目标场景：

```text
Feature Design
      ↓
Coding
      ↓
Test Validation
      ↓
Code Review
      │
      ├── Approved → Done
      │
      └── Changes Requested
                ↓
              Bug Fix
                ↓
              Test
                ↓
              Review
```

典型 Agent：

```text
Feature Design   → Codex
Coding           → Qoder
Test Validation  → Claude
Code Review      → Claude
Bug Fix          → Qoder
```

目标体验：

> 用户选择一个本地 Repository，输入开发目标并点击 Run 后，在无需人工复制 Agent 输出的情况下，由多个 Agent 自动完成 Design → Coding → Test → Review → Fix 的协作闭环。

---

# 2. 核心产品原则

## 2.1 Local-first

Agent 执行、代码访问、Git、Terminal、Workspace 等能力优先运行于用户本地。

云端不是 MVP 前置依赖。

---

## 2.2 Capability-first

产品核心抽象不是 Agent，而是 Capability。

```text
Agent = 执行者
Capability = 能力
Workflow = 能力的协作过程
```

例如：

```text
Code Review
```

是一种 Capability。

当前可以由：

```text
Claude Code
```

执行。

未来可以由：

```text
Codex
Internal Agent
Remote Agent
```

执行。

---

## 2.3 Context 是一等公民

Agent 不应该简单继承完整 Conversation History。

每一个 Workflow Node 应获得明确的 Context View。

例如 Review：

```text
Requirement
Technical Design
Current Git Diff
Latest Test Result
```

---

## 2.4 Contract 驱动 Agent 协作

Agent 输出必须同时考虑：

```text
Human-readable Output
+
Machine-readable Output
```

Workflow Runtime 依赖结构化 Output Contract 推进状态。

---

## 2.5 Workspace 是 Coding 场景的共享现实

Agent 围绕：

```text
Repository
Branch
Worktree
Files
Git Diff
Commit
Terminal
```

进行协作。

---

# 3. 技术基线

建议技术栈：

```text
Desktop
Tauri 2

Frontend
React + TypeScript + Vite

Core Runtime
Rust

Database
SQLite + WAL

Workspace Isolation
Git Worktree

Agent Integration
Adapter Pattern

First Transport
CLI / subprocess / PTY

Future Transport
A2A

Contract
JSON Schema

Artifact Storage
Local Filesystem + SQLite Metadata

Event
In-process Event Bus + SQLite Event Log
```

---

# 4. 领域模型

MVP 重点围绕以下对象构建：

```text
Project
Agent
Capability
WorkflowDefinition
WorkflowNode
WorkflowRun
NodeRun
Context
Artifact
Handoff
Workspace
Event
```

关系：

```text
Agent
  │
  │ provides
  ▼
Capability
  │
  │ instantiated as
  ▼
Workflow Node
  │
  │ executes as
  ▼
Node Run

Workflow Definition
  │
  ▼
Workflow Run
  │
  ├── Node Run
  ├── Workspace
  ├── Context
  ├── Artifact
  └── Event
```

---

# 5. 总体里程碑

建议拆成：

```text
M0  Domain & Runtime Foundation
M1  Agent Execution Layer
M2  Workspace Runtime
M3  Capability & Contract
M4  Workflow Engine
M5  Coding Collaboration Loop
M6  Desktop Product MVP
M7  Stability & Alpha
```

依赖关系：

```text
M0
 ↓
M1
 ↓
M2
 ↓
M3
 ↓
M4
 ↓
M5
 ↓
M6
 ↓
M7
```

其中：

> M0-M4 是 Runtime，M5 是价值验证，M6 才是产品化。

---

# 6. M0：Domain & Runtime Foundation

## 目标

先建立整个产品的底层领域模型和 Runtime 基础设施。

不接真实 Agent。

---

## 需求

### 6.1 Core Runtime

创建独立 Rust Core：

```text
collab-runtime
```

建议模块：

```text
runtime/
├── domain
├── agent
├── capability
├── workflow
├── context
├── artifact
├── workspace
├── event
└── storage
```

要求 Runtime 不依赖 Tauri UI。

未来可以被：

```text
Desktop
CLI
Daemon
Server
```

共同调用。

---

### 6.2 Persistence

SQLite：

需要支持至少：

```text
Project
Agent
Capability
WorkflowDefinition
WorkflowRun
NodeRun
Artifact
Event
```

要求：

* migration
* WAL
* transaction
* crash-safe state persistence

---

### 6.3 Event Model

建立基本事件模型：

```text
workflow.created
workflow.started
workflow.completed
workflow.failed

node.ready
node.started
node.completed
node.failed
node.cancelled

artifact.created

workspace.created
workspace.changed
```

Event 同时：

```text
publish to runtime
+
persist to SQLite
```

---

## M0 验收

能够通过测试代码：

```text
create WorkflowRun
→ create NodeRun
→ state transition
→ write event
→ restart runtime
→ restore state
```

不需要 UI。

---

# 7. M1：Agent Execution Layer

## 目标

统一调用不同类型 Agent。

---

## 7.1 Agent Definition

Agent 基础配置：

```text
id
name
type
command / endpoint
status
env
auth
metadata
```

---

## 7.2 AgentAdapter

建立统一接口。

概念接口：

```text
start()
send()
status()
cancel()
resume()
stream_events()
collect_result()
```

不要让 Workflow Runtime 感知 Claude、Codex 或 Qoder 的细节。

---

## 7.3 CLI Agent Adapter

第一阶段优先：

```text
CLI
subprocess
PTY
```

支持：

```text
stdout
stderr
exit code
timeout
cancel
working directory
environment variables
```

---

## 7.4 第一批 Agent Adapter

优先验证：

```text
Codex
Claude Code
```

Qoder / Hermes 后续补。

原因：

先证明 Adapter Architecture，而不是一开始接四个平台。

---

## 7.5 Agent Run Log

保存：

```text
command
start time
end time
exit status
stdout
stderr
structured result
```

---

## M1 验收

Runtime 可以执行：

```text
invoke(agent, instruction, workspace)
```

并获得：

```text
Running
→ Completed / Failed
```

以及完整执行日志。

---

# 8. M2：Workspace Runtime

## 目标

建立 Coding 场景的共享 Workspace。

---

## 8.1 Project

用户可以注册本地 Git Repository。

读取：

```text
repo path
current branch
remote
git status
```

---

## 8.2 Workflow Workspace

每一个 Workflow Run 创建独立：

```text
Git Worktree
```

例如：

```text
repo:
~/code/project

workspace:
~/.collab/workspaces/run_123

branch:
collab/run-123
```

---

## 8.3 Workspace API

至少提供：

```text
list files
read file
git diff
git status
current commit
run command
```

---

## 8.4 Workspace Lifecycle

支持：

```text
create
reuse
cleanup
archive
```

---

## 8.5 Concurrency Rule

MVP 先定义：

```text
不同 Workflow Run
→ 不同 Worktree

同一个 Workflow Run
→ 共享同一个 Worktree
```

暂时不支持 Node-level worktree。

---

## M2 验收

可以创建两个 Workflow Run：

```text
run_A
run_B
```

两者可以同时修改代码而互不影响。

---

# 9. M3：Capability & Contract

## 目标

建立产品最核心的抽象。

---

# 9.1 Capability

Capability 表示可复用的专业能力。

MVP 第一批：

```text
Feature Design
Coding
Test Validation
Code Review
Bug Fix
```

---

## 9.2 Capability Definition

基本结构：

```text
id
name
description
version

executor
instruction

input_contract
output_contract

permissions
default_config
```

---

## 9.3 Executor Binding

MVP：

```text
Capability
→ fixed Agent
```

例如：

```text
Code Review
→ Claude Code
```

架构必须允许未来：

```text
Capability
→ multiple providers
```

---

# 9.4 Input Contract

声明 Capability 需要什么。

例如：

```text
Code Review requires:

Requirement
Technical Design
Workspace Diff
Test Result
```

---

# 9.5 Output Contract

统一使用 JSON Schema。

例如：

```text
ReviewResult
```

包含：

```text
status:
approved | changes_requested

summary

issues[]
```

---

## 9.6 Dual Output

Agent 执行结果允许：

```text
Markdown
+
JSON
```

其中：

```text
Markdown → UI / 人
JSON → Runtime
```

---

## M3 验收

可以调用：

```text
invoke_capability("code_review")
```

而不是：

```text
invoke_agent("claude")
```

Runtime 能验证其输出是否满足 Contract。

---

# 10. M4：Context + Workflow Engine

这是第一个真正形成 Collaboration Runtime 的阶段。

---

# 10.1 Context Binding

Capability 只声明：

```text
我需要什么
```

Workflow Node 声明：

```text
这些东西从哪里来
```

例如：

```text
requirement
← workflow.input.requirement

design
← nodes.feature_design.output

workspace
← workflow.workspace

test_result
← nodes.test.output
```

---

## 10.2 Context Resolver

负责：

```text
Reference
→ Resolve
→ Context View
→ Agent Input
```

MVP 暂时不做：

```text
RAG
Embedding
Semantic Retrieval
Memory
```

只支持明确引用。

---

# 10.3 Workflow Definition

支持：

```text
Node
Edge
Condition
Loop
Retry
```

---

## 10.4 Node State

至少：

```text
pending
ready
running
succeeded
failed
waiting_human
cancelled
```

---

## 10.5 Workflow Control

支持：

```text
start
pause
resume
cancel
retry
```

---

## 10.6 Condition

允许：

```text
review.output.status == approved
```

跳转：

```text
Done
```

否则：

```text
Bug Fix
```

---

## 10.7 Loop Protection

必须有：

```text
max_iterations
```

例如：

```text
Review/Fix 最大 3 次
```

超过：

```text
waiting_human
```

---

## M4 验收

用 Fake Agent 跑通：

```text
A
↓
B
↓
Review
├── approved → Done
└── rejected → Fix → Review
```

并支持 Runtime 重启后 resume。

---

# 11. M5：Coding Collaboration Loop

这是 MVP 最关键的业务里程碑。

## 目标

真正实现：

> 多个真实 Agent 自动协作完成开发闭环。

---

## 11.1 Feature Design

输入：

```text
User Goal
Repository Context
```

输出：

```text
feature_design.md
feature_design.json
```

---

## 11.2 Coding

输入：

```text
Goal
Feature Design
Workspace
```

执行真实代码修改。

输出：

```text
ImplementationResult
Git Diff
Commit / Workspace State
```

---

## 11.3 Test Validation

输入：

```text
Goal
Design
Workspace
```

执行：

```text
existing tests
build
lint
custom validation
```

输出：

```text
TestResult
```

---

## 11.4 Code Review

输入：

```text
Goal
Feature Design
Git Diff
Test Result
```

输出：

```text
ReviewResult
```

---

## 11.5 Bug Fix

输入：

```text
Goal
Design
Workspace
Review Issues
```

修改代码。

然后重新：

```text
Test
→ Review
```

---

## M5 验收

用户只提供：

```text
Repository
+
Goal
```

例如：

```text
“给项目增加一个 /health API”
```

系统能够自动完成：

```text
Design
Coding
Test
Review
Fix
Review
```

全过程不需要人工 Copy/Paste。

这应该作为 MVP 最重要的技术 Demo。

---

# 12. M6：Desktop Product MVP

只有 M5 跑通之后再重点做 UI。

---

## 12.1 Project Page

展示：

```text
Project Name
Repository
Branch
Git Status
```

---

## 12.2 Agent Manager

展示：

```text
Codex        Connected
Claude Code  Connected
Qoder        Not Configured
```

支持：

```text
detect
configure
test connection
```

---

## 12.3 Capability Library

这是核心页面之一。

展示：

```text
Feature Design
Coding
Test Validation
Code Review
Bug Fix
```

每个 Capability 可以查看：

```text
Executor
Prompt
Input
Output
Permissions
```

---

## 12.4 Workflow

MVP 不需要自由 Canvas。

用纵向流程即可：

```text
Feature Design
↓
Coding
↓
Testing
↓
Review
↓
Fix
```

---

## 12.5 Create Run

输入：

```text
Goal

Repository

Workflow
```

按钮：

```text
Run
```

---

## 12.6 Run Overview

展示：

```text
✓ Feature Design
✓ Coding
✓ Test
● Review
○ Fix
```

---

## 12.7 Node Detail

最重要的是四个区域：

```text
Input
Context
Execution
Output
```

特别要展示：

> What did this agent see?

即：

```text
Context View
```

---

## 12.8 Timeline

例如：

```text
11:01 Workflow Started
11:01 Workspace Created

11:02 Codex Started
11:05 Feature Design Completed

11:06 Qoder Started
11:15 Coding Completed

11:16 Claude Test Started
11:18 Test Passed

11:19 Claude Review Started
11:21 Changes Requested

11:22 Qoder Fix Started
```

---

## 12.9 Workspace

展示：

```text
Files
Git Diff
Commits
Logs
Artifacts
```

同时提供：

```text
Open in VS Code
Open in Cursor
Open Folder
```

---

# 13. M7：Stability & Alpha

这一阶段不是继续加功能，而是让系统真正可用。

---

## 13.1 Failure Recovery

覆盖：

```text
Agent Crash
Agent Timeout
Malformed Output
CLI Exit
Runtime Restart
Desktop Restart
```

---

## 13.2 Workflow Resume

桌面应用关闭重新打开后：

```text
running
waiting_human
failed
```

状态必须可以恢复。

---

## 13.3 Observability

每一个 Run 可以回答：

```text
谁执行的？

为什么执行？

输入是什么？

看到了哪些 Context？

产生了什么 Artifact？

为什么走到下一个 Node？

失败在哪里？
```

---

## 13.4 Human Intervention

用户至少可以：

```text
Pause
Retry
Skip
Stop
Edit Context
Continue
```

---

## 13.5 Safety

Agent 权限至少区分：

```text
Read Files
Write Files
Git Operations
Shell Execution
Network
```

---

# 14. MVP P0 / P1 / P2 优先级

## P0：必须实现

```text
Local Repository
Git Worktree

Agent Adapter
CLI Execution

Capability
Prompt
Input Contract
Output Contract

Context Binding

Workflow State Machine
Condition
Loop
Retry

Workflow Run
Node Run

Artifact
Event Log

Coding → Test → Review → Fix

Run Timeline
Context View
```

---

## P1：MVP 后立即补

```text
Agent 自动发现
Capability 编辑器
Workflow 编辑
Human Gate
Resume
Permission
Cost / Token Tracking
Open in IDE
```

---

## P2：明确后置

```text
Cloud SaaS
Organization
Multi-user

Dynamic Orchestrator

Automatic Agent Selection

Capability Marketplace

Workflow Marketplace

A2A-native Remote Agent

Memory
RAG
Semantic Context

Remote Sandbox

Complex DAG Canvas
```

---

# 15. 推荐实现顺序

不要按 UI → Backend 的顺序。

推荐：

```text
① Domain Model

② SQLite / Event / State

③ Agent Adapter

④ Workspace

⑤ Capability

⑥ Contract

⑦ Context Resolver

⑧ Workflow Engine

⑨ Coding Workflow

⑩ CLI Demo

⑪ Desktop Integration

⑫ Desktop UI

⑬ Reliability
```

强烈建议首先提供一个 CLI：

```bash
collab run \
  --repo ./project \
  --workflow feature-development \
  --goal "Implement health check API"
```

只有 CLI 完整跑通之后，再将 Runtime 接入 Tauri。

---

# 16. Codex 技术设计重点

在设计实现时，请重点回答以下问题。

### Runtime

* Core Runtime 如何保持与 Tauri 解耦？
* Workflow Runtime 采用什么状态机设计？
* 如何保证 Runtime crash 后可以恢复？

### Agent

* CLI Agent 如何统一抽象？
* 如何管理长生命周期 Agent Process？
* PTY 与普通 subprocess 如何取舍？
* 如何支持 interrupt / cancel / resume？

### Capability

* Capability Schema 应如何定义？
* Capability version 如何处理？
* Agent Provider 如何解耦？

### Context

* Context Ref 数据结构如何设计？
* Node 的 Context View 如何构建？
* Context Snapshot 如何保证可复现？

### Contract

* JSON Schema 在 Runtime 中如何校验？
* Agent 输出结构化结果失败时怎么处理？

### Workspace

* Git Worktree lifecycle 如何管理？
* workflow run 与 branch/worktree 的关系是什么？
* 用户已有 dirty working tree 如何处理？

### Workflow

* Definition 和 Run 如何分离？
* Node 和 NodeRun 如何分离？
* loop / retry / condition 如何建模？
* Event-driven 还是 Scheduler-driven？

### Persistence

* 哪些状态进入 SQLite？
* 哪些 Artifact 存 filesystem？
* DB 和 filesystem 如何保持引用一致？

### Desktop

* Tauri command/event API 如何划分？
* 前端如何订阅 Runtime event stream？
* Runtime 是否应独立 daemon 化？

---

# 17. MVP 完成定义

MVP 不以“页面做完”作为完成标准。

MVP 的 Definition of Done：

> 在一个真实 Git Repository 中，用户输入一个开发目标并启动 Workflow 后，Codex/Qoder/Claude 等多个本地 Agent 可以在隔离 Workspace 中自动完成 Feature Design、Coding、Test、Review、Fix 和再次 Review；所有 Context、Artifact、状态和 Handoff 都可追踪，用户无需人工复制不同 Agent 之间的信息。

最终 Demo 应能够清楚展示：

```text
一个 Goal
↓
多个 Capability
↓
多个 Agent
↓
同一个 Workspace
↓
自动 Handoff
↓
完整开发闭环
```

这才是产品第一阶段真正需要验证的核心价值。
