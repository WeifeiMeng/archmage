简体中文 | [English](../en/guide-online-doctor.md)

# 在线 Doctor：2026-09-17 更新

本次更新位于 `codex/s0-runtime-fencing` Worktree，尚未自动合入 main。主目录仍是上一版。

```bash
cd /Users/mengweifei/Desktop/code/github/archmage/.worktrees/s0-runtime-fencing
ARCHMAGE_HOME="$HOME/Library/Application Support/archmage-dev" cargo run -p archmage-cli -- doctor
```

这会实际调用你当前模型配置下的 Codex，使用本机登录、消耗使用额度，sandbox 为 read-only，deadline 为 120 秒。stderr 会先提示；stdout 成功输出 JSON：

- authentication=verified
- probe.thread_id：真实会话 ID
- probe.exit_code=0
- probe.result={"ok":true}
- probe.diagnostics_dir：本次诊断目录
- database：真实数据库检查对象，包含 schema_version=5、foreign_keys=true、
  synchronous=2、journal_mode=wal、busy_timeout_ms=5000。这不是整体工作流已可用。

每次在线调用创建独立 probe-* 目录，保留 events.jsonl、stderr.log、result.json、metadata.json、Schema 与探针仓库。事件在进程运行时增量解码；失败时也保留已收到的日志并显示目录路径。Unix 上目录 0700，日志文件 0600。诊断可能包含敏感内容，请勿直接上传或提交 Git。

不调用模型时继续使用 doctor --offline。在线和离线诊断都会在数据根创建或打开
archmage.db 并执行迁移；不会创建 Workflow Run。原有 run/status/logs/resume/cancel 仍未实现。

## 验证

```bash
cargo test --workspace
cargo test -p archmage-cli --test doctor_live -- --ignored --nocapture
```

第二条是真实 CLI 接入与日志权限测试，不是 Fake Agent。Core 的 codex_live 测试仍可运行，其临时文件会在测试结束后清理。CLI doctor 的诊断目录不会自动删除。

## 已知边界

Unix subprocess 已使用独立 process group，超时和显式取消会校验启动身份并执行 TERM/KILL；数据库 owner generation 和完整崩溃恢复编排仍待后续 Runtime。Core 已有真实 Git Worktree provisioning、精确恢复对账、ChangeSet Artifact 与内容级 Snapshot，但尚未接入 `run`。当前只读探针不是通用 Agent invocation，也不代表四个能力协作已完成。
