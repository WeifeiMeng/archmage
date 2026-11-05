# Agent 安装指引设计

## 目标

当 Archmage 检测不到 Codex CLI 或 DeepSeek Harness Python SDK 时，在 Agent 管理页提供与当前操作系统匹配的安装指引。Archmage 不执行安装命令，不申请管理员权限，也不自动安装 Node.js、Python 或其他系统运行时。

## 展示位置

- Provider 分组没有可用安装时，在分组标题操作区显示“安装指引”。
- 已有实例检查失败且错误属于 CLI/SDK 缺失时，在实例卡片操作区显示“查看安装指引”。
- 两个入口打开同一 Provider 安装指引面板，避免重复维护文案和命令。

## 指引面板

面板显示 Provider、当前系统、支持状态、前置条件、安装命令和后续操作。

- 支持的平台提供“复制命令”“打开官方文档”“重新检查”三个操作。
- 复制只写入剪贴板，不执行命令。
- “重新检查”复用现有发现能力；实例入口检查当前实例，Provider 分组入口检查默认可执行文件或 SDK。
- 失败信息保留在面板内，不用全局错误横幅替代具体反馈。

## 平台内容

### Codex

- macOS：展示官方独立安装脚本 `curl -fsSL https://chatgpt.com/codex/install.sh | sh`。
- Windows：展示官方 PowerShell 安装脚本 `powershell -ExecutionPolicy ByPass -c "irm https://chatgpt.com/codex/install.ps1 | iex"`。
- 安装完成后提示重新打开 Archmage；若新 PATH 已在当前进程可见，也允许直接重新检查。

### DeepSeek Harness

- 前置条件为 Python 3.10 或更高版本。
- macOS 14 或更高版本且为 Apple Silicon 时，展示 `python3 -m pip install deepseek-harness-sdk`。
- Windows、Intel Mac 或低于 macOS 14 时，明确显示“当前 DeepSeek Harness Python SDK 暂不支持此系统”，不展示无效安装命令。
- 安装后重新检查 Python SDK；Provider route、Model、凭证和 endpoint 仍在实例配置阶段完成。

## 系统识别与数据边界

- Tauri 后端返回只读的安装指引描述：Provider、操作系统、CPU 架构、支持状态、前置条件、命令和官方文档地址。
- 前端只负责呈现、复制、打开官方页面和触发现有检查，不在 JSX 中自行推导平台支持矩阵。
- 浏览器预览不伪造系统结果，显示“请在桌面窗口查看安装指引”。
- 安装指引不写入数据库，不改变 Agent 实例和能力绑定。

## 安全与错误处理

- 命令必须来自代码内固定白名单，不接收网页、用户输入或服务端动态内容。
- 外部链接固定指向 Codex 官方仓库/文档与 DeepSeek Harness 官方仓库文档。
- 剪贴板失败、打开链接失败和重新检查失败分别显示可恢复提示。
- 不记录凭证、环境变量内容或用户目录中的敏感路径。

## 验收

- macOS 与 Windows 的 Codex 分组显示正确安装指引。
- 支持的 Apple Silicon macOS 显示 DSH SDK 安装命令；Windows 和不支持的 Mac 明确阻止并解释原因。
- 缺失实例可以从 Provider 标题和实例卡片进入同一指引。
- 复制命令不执行安装；重新检查复用现有发现逻辑。
- 浏览器预览、暗色主题、键盘焦点和窄窗口布局可用。
- Rust、前端测试及生产构建通过。
