# B 批次：存储基础检查点

2026-09-17，feat/s0-codex。本检查点不是 B 批次完成或 S0 完成声明。

## 已实现

- SQLite schema v3、事务迁移、WAL、foreign_keys、FULL synchronous、5 秒 busy timeout。
- Run、NodeRun、attempt 分离；Runtime 生成 UUID；唯一 active attempt。
- revision/state CAS；状态和事件在同一事务提交。
- 按 Run 单调事件序列、cursor 查询；active attempt 查询只读。
- Artifact 私有文件、sync 后发布、SHA-256/长度校验、元数据与 artifact.created 同事务。
- 仅成功 attempt 的 Artifact 可读作 committed input；未登记文件不可引用。
- doctor（在线和离线）打开 archmage.db 并返回真实数据库检查结果。
- 每 Run 的 Workspace provisioning intent；Git 创建失败时保留 intent，不删除碰撞路径。
- 从 committed baseline 创建真实分支/worktree，dirty source 不进入 Run Worktree。
- 内容级 Snapshot 覆盖 index diff、working diff、tracked/untracked/binary 文件和 symlink target。
- provisioning 重启对账只接受 path、branch、baseline、common-dir 全部匹配的已登记 Worktree。
- ChangeSet 将 tracked binary patch、每个 untracked 内容和 canonical manifest 发布为 attempt-owned Artifact。
- Unix attempt 使用独立 session/process group；取消验证 PID/PGID/启动标识，并按 TERM、宽限、KILL、wait 确认执行。
- 每 Run OS owner lock 互斥调度器；数据库 owner generation fencing 拒绝旧 owner 登记进程或提交终态。

## 验证范围

使用临时真实 SQLite，而非模拟存储。覆盖重开、重复迁移、错误 schema、
并发领取、重复完成、外键失败、事件注入失败时回滚，以及 Artifact 篡改、
丢失、越界 ID、symlink root、登记失败的孤儿文件。真实临时 Git 仓库测试还覆盖
dirty source 隔离、空格路径、两个 Run、unborn HEAD、碰撞 intent、partial provisioning
恢复和 ChangeSet Artifact。普通 shell 测试覆盖双流输出、超限、超时、错误身份拒绝、
TERM 宽限和忽略 TERM 后整个进程树 KILL。

## 明确尚未实现

- Project/Provider/Capability/Context/Handoff/check/control 的完整持久化模型。
- 终态、Contract 校验输出及 Handoff 的整体提交；当前 finish_attempt 只是内部 ledger API，
  调用方必须先做 Contract 校验，不是用户可直接使用的工作流入口。
- 跨节点调度和 Runtime 恢复编排。
- 原子文件发布前后的进程崩溃注入矩阵及敌对文件系统 TOCTOU 加固。
- source 前后不变量检查，以及非精确 partial provisioning 的人工诊断入口。

只有 SQL 查询和持久化，不会在打开数据库时自动恢复或启动 Agent。
run/status/logs/resume/cancel 的 CLI 业务仍未接线。未自动合并 main 或 push。
