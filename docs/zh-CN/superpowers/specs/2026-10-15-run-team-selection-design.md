简体中文 | [English](../../../en/superpowers/specs/2026-10-15-run-team-selection-design.md)

# Run Team 选择设计

## 目标

创建 Run 必须提供 Git 仓库、目标和一个 Team 选择。所选 Team 是表单中唯一展示的协作配置。

## 表单

Team 选择器下方紧接一个紧凑只读预览，展示成员 Capability 与有向边。消息流 Team 还要标明入口与终止 Capability；历史 Team 标注其旧咨询语义。切换选择会更新预览，但不修改 Team。

移除长期显示的 Tester、全 Capability 绑定和 Runtime Ready 卡片。不得承诺每个 Run 都包含全部四个内置 Capability。只有在没有 Team、所选 Team 成员绑定不可用或本地 Runtime 不可用时，才显示小型可操作消息。Team 问题链接到 Team 管理，绑定问题链接到 Capability 管理。后端 preflight 仍是最终权威，不新增执行选项。

## 启动行为

表单绑定检查只考虑所选 Team 成员。只要所选 Team、仓库与目标有效，即使未使用的 Capability 未配置也能启动。连接状态本身不作为启动阻塞；现有后端 preflight 负责实时校验。新 Run 继续在创建时冻结 Team 图和 Capability 设置。

从 Team 管理返回后，启用 Start 前重新加载 Team 列表与 Agent 绑定。加载期间隐藏缓存 Team 详情；Team 列表请求失败必须禁用 Start 并提供重试，不能伪装成空 Team 列表。纯浏览器预览不得声称 Team 仍在加载。

## 验证

使用纯 TypeScript 测试覆盖 Team 预览详情与所选成员绑定过滤。运行桌面测试套件与生产构建，并检查选择、空 Team 与阻塞状态下的最终表单。
