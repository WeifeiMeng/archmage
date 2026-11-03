# Agent 管理按 Provider 分组设计

简体中文 | [English](../../../en/superpowers/specs/2026-10-30-agent-provider-grouping-design.md)

## 目标

让 Agent 管理页的添加入口与 Provider 建立直接归属关系，降低顶部操作的视觉权重，并让 Codex 与 DeepSeek Harness 的实例管理能力保持对等。

## 页面结构

- 顶部 Hero 删除 `LOCAL AGENT CONNECTIONS` 眉题和全局“添加 Agent 实例”操作。
- Hero 保留页面标题、说明和实例状态统计。
- 本地连接区域固定展示两个受支持 Provider 分组：Codex 和 DeepSeek Harness；分组不因实例为空而消失。
- 每个分组包含名称、简短说明、低强调添加按钮和该 Provider 的实例列表。
- 未支持的 Provider 继续显示在受支持分组之后，并保持“尚未支持”状态。

## 添加交互

- Codex 分组提供“添加 Codex”按钮，DeepSeek Harness 分组提供“添加 DeepSeek Harness”按钮。
- 点击按钮后，新实例表单显示在对应分组中，Provider 类型由入口确定，不再显示 Provider 下拉框。
- 同一时间最多展开一个新实例表单；点击另一个 Provider 的添加按钮会切换表单，点击当前按钮或取消会收起表单。
- 创建成功后关闭表单并刷新实例列表；保存、验证、删除等既有行为不变。

## 视觉层级

- Provider 添加按钮使用普通次级按钮样式，不使用 Hero 强调色样式。
- Provider 分组标题承担页面的主要分类层级，实例卡片保持现有编辑和状态展示结构。
- 空 Provider 分组显示简短空状态，确保用户仍能找到添加入口。

## 测试与验收

- 页面不再渲染 `LOCAL AGENT CONNECTIONS` 或 Hero 全局添加按钮。
- Codex 和 DeepSeek Harness 分组及各自添加入口始终存在。
- 两个入口分别打开绑定到正确 Provider 的新实例表单。
- 同一时间只显示一个新实例表单，取消和创建成功会关闭表单。
- 现有桌面端测试与生产构建保持通过。

## 非目标

- 不修改 Agent 数据模型、Rust 服务、Provider 执行逻辑或数据库结构。
- 不改变未支持 Provider 的可用状态。
- 不重新设计现有 Agent 实例编辑表单。
