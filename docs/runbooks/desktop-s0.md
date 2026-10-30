# Archmage 桌面客户端使用手册

桌面客户端是当前 Runtime 的本地工作台。它直接调用 `archmage-core`，不是解析 CLI stdout，也没有额外后台 daemon。

## 当前可以做什么

- 选择本地 Git 仓库；
- 配置本地 Codex 或 DeepSeek Harness Agent 实例并执行真实连通验证；
- 在独立的 Capability 管理页为四个职责分别选择 Agent 实例；
- 在团队管理页自行选择至少两个能力、指定入口与终止能力，并在画布中绘制消息流向；
- 选择团队、仓库和目标创建 Run；
- 启动所选能力的真实 Provider Run；默认团队仍是 Designer → Coder → Tester → Reviewer；
- 查看最近 Run、所选能力节点状态和持久化事件时间线；
- 查看节点相关事件；
- 打开隔离工作区；
- 取消正在运行的 Run；
- 客户端重启时清理失去 owner 的 Provider 进程并收敛遗留 Run。

当前不包含图形化 resume、任意节点类型和后台常驻执行。团队画布限于四个内置能力，但消息流连线可双向循环。

## 启动开发客户端

前置检查：

```bash
git --version
cargo --version
node --version
npm --version
codex --version
codex login status
python3 -m pip show deepseek-harness-sdk deepseek-harness-runtime-bin
python3 -c 'import deepseek_harness; print("DeepSeek Harness SDK ready")'
```

在 Archmage 仓库根目录一键启动：

```bash
./scripts/dev-desktop.sh
```

脚本会检查 Git、Cargo、Node.js、npm 和 Codex CLI；仅当 `node_modules` 不存在时安装前端依赖，然后启动 Tauri。Tauri 会同时启动 Vite 界面和内嵌 `archmage-core` 的 Rust Runtime，不需要单独启动后端 daemon。

第一次启动会安装或编译依赖，因此会比后续启动慢。看到 Vite local URL 和 `Running .../archmage-desktop` 后，桌面窗口会打开。

如果需要隔离数据，在启动前设置专用目录：

```bash
export ARCHMAGE_HOME="$HOME/Library/Application Support/archmage-desktop-dev"
./scripts/dev-desktop.sh
```

脚本会原样传递 `ARCHMAGE_HOME`、当前 Codex 配置和 `DSH_HOME`。Archmage 不复制 DeepSeek Harness 的凭证或 endpoint；Provider route 和 model 由 Agent 实例明确配置。

如果需要排查 npm 或 Tauri 启动问题，可以分步执行底层命令：

```bash
npm install --prefix apps/archmage-desktop
npm run tauri dev --prefix apps/archmage-desktop
```

桌面端和 CLI 使用相同的数据格式。只要 `ARCHMAGE_HOME` 相同，就能看到相同的 Run。

## 配置并验证本地 Agent

1. 点击侧栏“Agent 管理”。可添加 Codex 或 DeepSeek Harness 实例。
2. 默认的 `codex-default` 会从当前 `PATH` 自动寻找 `codex`；如需使用指定安装，填写 Codex CLI 的绝对路径并保存。
3. 模型覆盖可以留空，此时沿用 Codex 自身配置；填写后仅该 Agent 实例使用指定模型。
4. 点击“检查 CLI”，只检查可执行文件、版本、必需参数和 Git；此时状态是“CLI 可用 · 未验证”，不能据此判断账号已连通。
5. 点击“验证连接”。该操作会使用当前实例的可执行文件和模型覆盖发起一次真实、只读的 Codex 请求，并展示版本、可用状态和上次验证时间。
6. 点击侧栏“Capability 管理”，逐个选择 Designer、Coder、Tester、Reviewer，确认其“执行 Agent”指向预期实例。可在“角色补充提示词”填写该能力的工作侧重点，再分别在 Skills 和 Tools 中点击“添加”，手动填写名称与用途（例如 Designer 的 `brainstorming`、`web_search`）；不需要的条目可移除。最后点击“保存 Capability”。Runtime 内置职责与输出契约仍固定。

DeepSeek Harness 实例默认从 `PATH` 查找 `python3`，也可填写安装了 `deepseek-harness-sdk` 的 Python 绝对路径。SDK 与同版本 `deepseek-harness-runtime-bin` 必须同时可用；实例还需填写 Provider route 和 model。它继承启动 Archmage 时的凭证、endpoint 和 `DSH_HOME`，“检查 SDK”只验证安装与版本，“验证连接”会启动临时只读 sidecar 并发起一次真实请求，可能消耗额度。Archmage 通过官方 Python SDK 和自身 Cordis 配置集成，不要求修改 DeepSeek Harness 仓库。

“添加 Agent 实例”允许创建多个 Codex 或 DeepSeek Harness 配置。能力绑定是全局设置，不在单个 Run 中临时覆盖；Run 启动时会冻结当时的绑定及能力配置。修改只影响之后创建的 Run。

能力页面不展示拓扑，也不设置团队关系。Skill/Tool 列表是传给该能力所绑定真实 Agent 的提示词指引；它不代表本机已安装、已验证可用，也不是权限白名单。执行和只读咨询都会使用对应能力在 Run 创建时冻结的配置。

Capability 修改未保存时，切换能力、离开页面或关闭窗口都会先询问是否放弃；若另一处已更新同一能力，保存会提示冲突，可点击“重新加载”后重新编辑。

未绑定的实例可以禁用或删除；已绑定的实例需要先把相关能力切换到其他实例。新建 Run 页面会显示缺失或不可用的能力绑定，并提示尚未通过真实连接验证的实例。

同一 Run 中，绑定 DeepSeek Harness 的每个 Capability 分别保有一个长生命周期 sidecar 和 DSH 对话；该能力因反馈或消息流再次执行时继续原对话，不会与其他能力共享。Designer、Reviewer 和咨询使用只读权限，Coder、Tester 使用工作区写权限，所有交互式 approval 自动拒绝。历史咨询调用使用独立临时对话，不写入主能力会话；Run 完成、失败或取消时 sidecar 会被回收。Archmage 重启后不恢复该对话，遗留 Run 按 `interrupted` 收敛。

Claude、OpenClaw 和 Hermes 在当前页面只显示“尚未支持”，不会执行假探测或被绑定到能力。

## 配置团队拓扑

1. 点击侧栏“团队管理”，再点“创建团队”。新团队从空的消息流画布开始；不会自动创建 Coder–Reviewer 组合。
2. 勾选至少两个能力。例如选 Coder 和 Reviewer，把“入口能力”设为 Coder，把“可发出完成信号的能力”设为 Reviewer。
3. 从 Coder 右侧的圆点拖到 Reviewer 左侧圆点，画出 Coder → Reviewer；再从 Reviewer 右侧拖到 Coder 左侧，画出反馈线。也可聚焦输出圆点按 Enter，再聚焦目标输入圆点按 Enter。拖到空处或按 Escape 会取消连线。
4. 点击一条线可选中并删除。节点主体可拖动或用方向键移动。修改名称并保存；图中每个能力都必须从入口可达，也必须有通向终止能力的路径。
5. 回到“新建 Run”选择该团队。未保存修改时离开会提示；版本冲突时刷新后重新编辑。

箭头代表交付和反馈，不是“咨询许可”。每次交付都让目标能力执行一次真实 Codex；收到两条消息就执行两次，执行仍按队列顺序串行。能力的结构化输出会选择一个或多个已连通下游，也可由指定终止能力发出完成信号。以 Coder ↔ Reviewer 为例：Coder 实现并交给 Reviewer；Reviewer 给出修改意见并回交 Coder；Coder 修订后再次交付；Reviewer 认可后完成流程。最多执行 24 次，超过后 Run 失败并保留历史；终止时其他待执行消息标记为跳过。没有 Tester 的团队不能声称 Tester 已执行验收。

默认团队及以前保存的团队仍按旧“咨询”语义运行。页面会标明“历史咨询”；点击“复制为消息流团队”只会创建未保存副本，需重新指定入口与终止能力，不会修改原团队。历史模式仍要求下游只读咨询认可，修改意见回给上游；不要把其边理解为新消息流。

## 创建第一个 Run

1. 点击“新建 Run”。
2. 点击“选择”，选一个至少已有一次 commit 的本地 Git 仓库。
3. 在“目标”中写清希望最终实现和验证的结果。
4. 选择执行团队，点击“启动真实 Run”。Run 只冻结所选能力、拓扑和对应 Agent 绑定。若包含 Tester，它会根据实际可用的目标、设计、实现、ChangeSet 与工作区信息，自主选择验证方式。

启动后界面会切换到 Run 工作台：

- 顶部显示 Run state、仓库、冻结团队和 Run ID；
- 能力状态条显示所选节点当前执行到哪一环；
- 每个能力节点显示该 Run 冻结的 Agent 实例；
- 时间线显示 SQLite 中的持久化事件；
- 点击能力节点可在右侧检查结果和相关事件；新消息流团队另有逐条交付及路由记录，历史团队继续显示咨询进度；冻结的团队边也在工作台显示；
- “工作区”在 Finder 中定位该 Run 的隔离 worktree；
- “取消”终止当前真实执行并持久化 cancelled 状态。

## 关闭与恢复

只要还有本客户端启动的 Run 在执行，关闭窗口时会询问是否取消这些 Run。选择继续关闭后，客户端先发出取消，再退出。

Archmage 当前不在客户端退出后继续调度。若进程崩溃或被强制结束，下次启动会终止失去 owner 的主任务或咨询进程，并把对应 Run 标记为 `interrupted`。图形化 resume 尚未提供。

## 与 CLI 配合

查看桌面端创建的 Run：

```bash
archmage status <run-id> --json
archmage logs <run-id>
```

从另一个终端取消：

```bash
archmage cancel <run-id>
```

如果 CLI 查不到桌面端的 Run，通常是两边的 `ARCHMAGE_HOME` 不一致。

## 简单验证

前端单测和生产构建：

```bash
npm test --prefix apps/archmage-desktop
npm run build --prefix apps/archmage-desktop
```

Rust/Tauri 编译检查：

```bash
cargo check -p archmage-desktop
```

完整命令说明见 [CLI 命令手册](../cli-reference.md)。
