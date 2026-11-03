简体中文 | [English](../../../en/acceptance/s0/batch-a-progress.md)

# A 批次实施检查点

## 2026-09-17 后续

在线 Doctor 现在会调用真实 Codex、增量解码 stdout，并保留包括失败在内的私有诊断。
真实 CLI 集成测试已通过，验证了最终 JSON 与 0600 诊断文件。
手工真实调用：Codex 0.154.0，thread 01a0ad29-6623-7d92-a80d-b78e28c404f6，exit 0，result {"ok":true}。
已将脱敏的实际 Event 形态作为解析器 fixture 提交；没有添加 Fake Provider。
下面的剩余列表记录的是早期检查点；在线接线、流式解码和诊断保留现已实现。
数据库与受控 process-group 恢复仍属于 B 批次；本文不宣称 S0 已完成。

日期：2026-09-16。分支：feat/s0-codex。

## 已完成

- Task 01：双 crate Rust workspace、CLI 参数校验、领域访问兼容性。
- Task 02 本地部分：数据根目录校验、私有目录创建、executable/version/flag 检查、离线诊断。
- Task 03 部分：JSONL decoder、有界 subprocess capture、真实只读 Codex probe 和显式 opt-in 测试。

## 验证

- `cargo test --workspace`：20 passed；1 个真实 Codex 测试默认显式忽略。
- 显式 `real_readonly_probe`：失败（Codex exit 1）。
- `codex login status`：已使用 ChatGPT 登录。
- 已安装 Codex：0.142.4。
- 直接 CLI 复现返回 HTTP 400 `invalid_request_error`：配置的 `gpt-5.6-sol` 模型要求更新版本的 Codex。

最初失败未使用 model override、permission bypass 或 Fake Provider。

## 已授权升级与复测

经用户批准，`/Users/mengweifei/.nvm/versions/node/v22.11.0` 下已有 npm 安装从 0.142.4 升级至 0.154.0。没有更改模型或登录配置；登录状态仍为 ChatGPT。

显式 `real_readonly_probe` 复测：PASS（1 passed，0 failed，0 ignored）。
Codex exit 0，并精确返回 `{"ok":true}`。
观察到的 thread ID：`01a0a9a5-034a-7451-91bf-6b6c8f467c39`。
CLI/模型兼容性不再阻塞 probe。

## A 批次完成前的剩余工作

- 已验证已安装 CLI/模型兼容性与真实只读 probe；继续完成下列集成工作。
- 将在线 probe 接入 Doctor（当时在线 Doctor 仍明确报告未实现）。
- 把增量 Event 解码接入存活 subprocess drain 并持久化诊断日志；当时 probe 先捕获有界输出再解码。
- 捕获脱敏的真实 Event fixture 与成功调用证据。
- 完整 process-group ownership/cancellation 属于 Task 07；当时 capture 只控制直接 child。
- 数据库诊断明确推迟到 B 批次。

不要仅依据离线测试把 A 批次或 S0 标记为已接受。
