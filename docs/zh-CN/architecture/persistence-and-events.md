简体中文 | [English](../../en/architecture/persistence-and-events.md)

# 持久化与事件

## 存储分工

SQLite 保存事务状态和可搜索元数据，本地文件系统保存可能较大或以流形式产生的内容。

```text
<archmage-data>/
├── archmage.db
├── workspaces/<run-id>/
└── runs/<run-id>/
    ├── workflow.json
    ├── nodes/<node-id>/attempt-<n>/
    │   ├── context.json
    │   ├── prompt.txt
    │   ├── events.jsonl
    │   ├── result.json
    │   └── summary.md
    └── artifacts/
```

数据根目录默认使用操作系统的应用数据目录，也可以通过 `ARCHMAGE_HOME` 覆盖。

## SQLite 职责

S0 至少持久化 Project、Provider、Capability Snapshot、Workflow Run、Node Run、Agent Run、Workspace、Context Snapshot 元数据、Artifact 元数据与 Event。

数据库启用 WAL 和外键，在 Schema 表中跟踪迁移，并设置有界 busy timeout。状态变更及其 Event 在同一个事务中提交。

## Artifact 持久性

Artifact 字节先写入目标目录中的临时文件，flush 后通过原子 rename 生效，之后才插入元数据。没有被已提交元数据引用的文件属于孤儿，不会参与 Context 解析。

Store 在读取已引用 Artifact 时校验内容长度与 hash。清理是显式操作，不属于 S0 正常 Run 完成路径。

## Event 模型

Event 只追加写入，在单个 Run 内有序，并包含稳定类型、Run 标识、可选节点和 Attempt 标识、时间戳、Schema 版本与类型化 payload。

S0 Event 家族包括：

- `workflow.created`、`workflow.started`、`workflow.completed`、`workflow.failed`、`workflow.cancelled`
- `workspace.created`、`workspace.changed`
- `node.ready`、`node.started`、`node.succeeded`、`node.failed`、`node.interrupted`、`node.cancelled`
- `agent.started`、`agent.event`、`agent.exited`
- `context.snapshotted`、`artifact.created`

进程内订阅者只在事务提交后接收 Event。持久化 Event 日志是重启、CLI 状态与未来桌面 Timeline 的权威来源。
