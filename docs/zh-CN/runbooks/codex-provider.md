简体中文 | [English](../../en/runbooks/codex-provider.md)

# Codex Provider 操作手册

## 目的

本手册定义 S0 真实本地 Codex CLI Provider 的 preflight 与诊断预期。在 `archmage doctor` 实现之前，它是一份设计阶段契约。

## 支持的调用接口

S0 依赖 Codex 非交互模式，并验证实际 CLI，而不是假定固定安装路径：

```bash
codex --version
codex exec --help
```

必需 flag 为 `--cd`、`--sandbox`、`--json`、`--output-schema` 与 `--output-last-message`。参见官方 [Codex 非交互模式文档](https://learn.chatgpt.com/docs/non-interactive-mode)。

## Preflight 检查

`archmage doctor` 必须报告：

1. 解析后的 Codex executable 路径与版本。
2. 每个必需 `codex exec` flag 是否可用。
3. Git executable 与版本。
4. Archmage 数据根目录路径及可写性。
5. SQLite 数据库打开与迁移状态。
6. 通过有界、不修改代码的 Codex probe 判断认证是否可用。

Probe 不得输出或持久化凭证材料。Probe 失败时应报告类型化 Provider 错误以及诊断所需的脱敏 stderr。

## 调用策略

- 使用 `-` sentinel 通过 stdin 传入完整 prompt。
- 用 `--cd` 显式设置 worktree。
- 根据 Capability 权限选择 `read-only` 或 `workspace-write`。
- 启用 `--json` 并将 stdout 持久化为 JSONL。
- 使用 `--output-schema` 传入带版本的 Capability Schema。
- 使用 `--output-last-message` 将最终响应写入 Attempt 所属路径。
- S0 绝不使用 `danger-full-access`。
- 正常 Handoff 或 Archmage 恢复不使用 `codex exec resume`。

## 诊断分类

| 症状 | Archmage 分类 | 操作员行动 |
|---|---|---|
| 找不到 executable | `preflight_failed` | 安装 Codex CLI 并重试 `doctor` |
| 缺少必需 flag | `preflight_failed` | 升级本地 Codex CLI |
| 登录或 entitlement 失败 | `auth_failed` | 完成 Codex 认证并重新运行有界 probe |
| 节点非零退出 | `cli_failed` | 检查 Attempt stderr 与 JSONL Event |
| 没有有效最终 JSON | `contract_invalid` | 检查原始最终消息、Schema 和 prompt 版本 |
| Runtime 在运行中被终止 | 对账后为 `interrupted` | 运行 `archmage resume <run-id>` |

## 版本证据

每个 Agent Run 都记录 Codex CLI 版本。验收报告包含该版本，以便复现协议回归。
