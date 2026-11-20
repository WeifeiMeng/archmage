# Archmage macOS arm64 下载发行设计

**日期：** 2026-11-20

**状态：** 已确认，等待实施计划

**首个发行版本：** `1.0.0`

**首个发行标签：** `v1.0.0`

## 目标

让没有 Archmage 源码、Rust、Node.js 或 npm 的普通用户，可以从 GitHub Releases 下载 Archmage，在 Apple Silicon Mac 上通过 DMG 安装并运行。

首版只支持 Apple Silicon（arm64）和 macOS 14 或更高版本。由于目前没有 Apple Developer Program 账号和 Developer ID Application 证书，`v1.0.0` 作为未签名、未公证的 Preview 发行。版本号使用 `1.0.0`，GitHub Release 标记为 Pre-release，标题为 `Archmage 1.0.0 — macOS Apple Silicon Preview`，避免把它表述为已经获得 Apple 信任链验证的正式安装包。

本设计参考 xopc 的标签驱动桌面发行和按架构提供 DMG 的方式，但继续使用 Archmage 现有的 Tauri 2 架构，不引入 Electron，也不复制 xopc 的应用运行时或自动更新实现。

## 已确认的产品决策

- 仅发布 `aarch64-apple-darwin` DMG，不发布 Intel 或 Universal 构建。
- 最低支持 macOS 14。
- 使用 GitHub 标签触发 GitHub Actions 构建和发布。
- 首版没有 Developer ID 签名、Apple 公证或 stapling。
- 首版不提供应用内自动更新；用户手动下载新 DMG 覆盖升级。
- Archmage 不自动安装 Codex、Qoder、DeepSeek Harness 或系统运行时；继续检测本地 Provider 并展示现有安装指引。
- 首版不建设独立下载站，不进入 Mac App Store。

## 当前差距

Archmage 已经是 Tauri 2 桌面应用，并具有 macOS ICNS 图标和内嵌 Rust Runtime，因此不需要更换桌面技术栈。但当前仍是开发态：

- `apps/archmage-desktop/src-tauri/tauri.conf.json` 的 `bundle.active` 为 `false`；
- 仓库没有 GitHub Actions 发布工作流；
- 没有版本同步、标签校验、DMG 命名和 SHA-256 生成流程；
- 没有面向下载用户的安装、首次打开、升级和卸载说明；
- 没有在无源码、无开发工具链的干净 Mac 上进行安装验收的发布门禁。

## 发行架构

```text
main 上的已验证发布提交
          │
          ├─ annotated tag: v1.0.0
          ▼
GitHub Actions / macos-15 arm64 runner
          │
          ├─ 校验 tag 与版本文件
          ├─ Rust format / clippy / tests
          ├─ Desktop tests / production build
          ├─ Tauri aarch64 DMG build
          ├─ DMG、Bundle、架构与内容检查
          └─ SHA-256
          ▼
Draft GitHub Release
          │
          ├─ Archmage_1.0.0_aarch64.dmg
          ├─ Archmage_1.0.0_aarch64.dmg.sha256
          └─ 双语 Release Notes / 安装说明
          ▼
全部门禁通过后发布为 Pre-release
```

`workflow_dispatch` 可以用于构建和保留短期 Actions Artifact，但不会创建公开 Release。只有从受保护的 `main` 发布提交创建并推送的 `v*` 标签可以进入 GitHub Release 发布路径。PR、普通分支和普通 `main` push 都不能发布安装包。

## 版本合同

以下版本必须完全一致：

- 根目录 `Cargo.toml` 的 `[workspace.package].version`；
- `apps/archmage-desktop/package.json` 的 `version`；
- `apps/archmage-desktop/src-tauri/tauri.conf.json` 的 `version`；
- `Cargo.lock` 中受 workspace version 影响的 Archmage package 版本；
- Git 标签去掉前缀 `v` 后的值。

新增一个发布辅助脚本统一设置或检查版本。脚本必须拒绝空值、非 SemVer、脏工作区、非 `main` 分支、已经存在的标签和版本不一致。首个版本直接将当前 `0.1.0` 更新为 `1.0.0`。

标签使用 annotated tag：

```bash
git tag -a v1.0.0 -m "Archmage 1.0.0"
git push origin main --follow-tags
```

只有发布实现合并、完整本地验证通过并确认远端 `main` 是预期提交后才创建标签；设计或实现过程中不能提前占用 `v1.0.0`。

## Tauri 打包配置

Tauri 配置需要：

- 开启 bundle；
- 仅启用 DMG target；
- 继续使用产品名 `Archmage` 和现有应用图标；
- 继续使用稳定标识 `dev.archmage.workbench`；
- 将 macOS 最低系统版本设为 `14.0`；
- 只构建 `aarch64-apple-darwin`；
- 不伪造签名 identity，不把 ad-hoc 签名描述为 Developer ID 签名；
- 不配置 updater endpoint 或 updater 签名密钥。

CI 对产物统一重命名为 `Archmage_1.0.0_aarch64.dmg`，不依赖 Tauri 默认文件名作为公开下载合同。

## CI 发布门禁

发布工作流固定使用 `macos-15` Apple Silicon Runner，而不是随时间变化的 `macos-latest`。使用仓库已有的 `rust-toolchain.toml` 安装 Rust 1.95.0、rustfmt 和 Clippy；使用固定 Node 22 版本及 `npm ci` 安装桌面依赖。

门禁依次执行：

1. 校验标签、workspace、Tauri 和 npm 版本一致；
2. `cargo fmt --check`；
3. `cargo clippy --workspace --all-targets -- -D warnings`；
4. `cargo test --workspace`，保留现有需要真实认证或付费调用的 ignored live tests；
5. `npm test -- --run`；
6. `npm run build`；
7. `npm run tauri build -- --target aarch64-apple-darwin --bundles dmg`；
8. 使用 `hdiutil verify` 校验 DMG，并只读挂载检查 `.app`；
9. 使用 `file` 和 `lipo -archs` 确认主程序只有 `arm64`；
10. 使用 `plutil` 检查 Bundle ID、版本号和最低系统版本；
11. 检查安装包中不存在数据库、凭证、`.env`、开发 worktree、测试 fixture 或构建缓存；
12. 记录当前没有 Developer ID 签名和公证；这属于已知发行属性，不伪装成 Gatekeeper 通过；
13. 生成 DMG 的 SHA-256 文件；
14. 创建 Draft Release 并上传两个资产；
15. 所有步骤成功后，将 Draft 转为公开 Pre-release。

任何失败都不能发布部分成功的 Release。可保留 Draft 和 Actions 日志用于诊断，修复后从新的发布提交重新构建；不能手工替换同一公开资产而不留下可审计记录。

## 权限和供应链边界

- 工作流默认 `contents: read`，仅 Release job 获得 `contents: write`。
- 不从 fork PR 或任意分支执行发布。
- GitHub Actions 固定到已审查的版本或 commit SHA，并通过后续维护更新。
- Release 资产只能来自当前 workflow 的构建目录，不能从开发机或旧运行复用。
- CI 日志不得打印 Provider 凭证、环境变量内容或本地用户数据。
- 首版不需要 Apple、Tauri updater 或第三方发布服务 Secret。
- 缓存仅用于加速依赖和编译，不能作为 Release 资产，也不能跳过产物验证。

## 安装和首次启动体验

用户从 GitHub Release 下载 DMG，打开后将 Archmage 拖入 Applications。由于首版没有 Developer ID 签名，正常文档优先引导用户：

1. 在 Finder 中右键 Archmage 并选择“打开”；或
2. 在“系统设置 → 隐私与安全性”中确认“仍要打开”。

删除 `com.apple.quarantine` 属性的终端命令不作为正常安装步骤，只能放在故障排查中，并解释它会绕过下载文件的隔离保护。

首次启动后，Archmage 使用现有 Agent 管理能力检查本机 Provider：

- 应用安装完成不代表 Provider 已安装；
- Provider 已安装不代表已经认证；
- Provider 已认证也不代表真实连接验证已经通过；
- 缺少 Provider 时只展示平台匹配的安装指引，不自动执行系统修改。

因此，用户不需要 Rust、Node.js 或源码仓库来运行 Archmage，但至少需要自行安装并认证一个受支持的 Provider 才能完成真实 Run。

## 数据、升级、降级和卸载

运行数据继续由 Archmage 写入 macOS Application Support 下的现有数据根目录，不放入 `.app` bundle。用新 DMG 覆盖 Applications 中旧版本时，Run、团队、Capability 和 Agent 配置应继续保留。

文档要求升级前退出 Archmage。Preview 阶段建议用户在重要升级前备份数据目录。数据库迁移保持只向前承诺：只有明确验证了 schema 兼容的版本才能降级，不能承诺任意版本覆盖回滚。

卸载分为两个明确动作：

- 删除 `/Applications/Archmage.app` 只卸载应用；
- 删除 Application Support 数据是独立且不可恢复的数据清理操作，必须由用户明确执行。

首版没有自动更新。设置页或 README 可以链接 GitHub Releases，但应用不静默检查、下载或替换自身。

## Release Notes 和双语文档

每个 Release 顶部明确写出：

- Preview / Pre-release；
- 未进行 Developer ID 签名和 Apple 公证；
- 仅支持 Apple Silicon；
- 最低要求 macOS 14；
- Provider 需要独立安装和认证。

README 增加清晰的 macOS 下载入口。新增英文与简体中文安装文档，并保持系统要求、Gatekeeper 指引、数据目录、升级语义和已知限制一致。Release Notes 同时提供简洁中文与英文段落。

## 验收

### 构建验收

- 所有非 live Rust 测试、Clippy、桌面测试和生产构建通过；
- DMG 校验、挂载、Bundle metadata 和 arm64 检查通过；
- SHA-256 可以验证下载文件；
- GitHub Release 的资产来自与 `v1.0.0` 指向相同提交的 workflow。

### 干净环境验收

在一台没有 Archmage 源码、Rust、Node.js 和 npm 的 Apple Silicon、macOS 14+ 机器上：

- DMG 可以挂载，应用可以拖入 Applications；
- 按未签名 Preview 指引可以首次打开；
- 应用创建并重用自己的数据目录；
- Agent 管理页能准确展示缺少 Provider 的状态和安装指引；
- 退出并重启应用后设置仍然存在。

### 真实功能验收

在安装并认证至少一个受支持 Provider 后，完成一个最小真实 Run，并验证：

- Workspace 创建成功；
- Agent 实时输出和结构化结果可见；
- Artifact 和运行状态持久化；
- 应用重启后仍可读取该 Run。

真实功能验收单独记录；不能用单元测试、模拟 Provider 或“应用能够打开”替代。

## 失败处理

- 版本不一致：构建前失败，不创建 Release。
- 测试或打包失败：保留日志和临时 Artifact，不公开 Release。
- DMG 无法验证或架构错误：视为发布阻断。
- 安装包含敏感或开发文件：视为发布阻断并调查来源。
- GitHub 上传失败：Release 保持 Draft，不能宣称版本已发布。
- 干净机无法启动：即使 CI 全绿也不能把发行标为验收完成。
- Provider 不可用：应用仍应正常启动并展示指引；这不是安装包损坏。

## 后续签名升级

获得 Apple Developer Program 账号后，保留相同的版本、DMG 和 GitHub Release 架构，增加：

- Developer ID Application 签名；
- Hardened Runtime 和最小 entitlements；
- Apple notarization；
- stapling；
- `codesign --verify`、`spctl --assess` 和在线/离线 Gatekeeper 验证；
- CI Secret 的最小权限、轮换和泄漏响应流程。

签名升级完成后取消 Pre-release/unsigned 文案。应用内自动更新作为独立设计处理，需要单独的更新签名密钥、清单、失败恢复和回滚策略，不与本次首发打包混在一起。

## 非目标

- Intel Mac 或 Universal Binary；
- Windows、Linux、iOS 或 Mac App Store；
- Developer ID 签名、公证和 stapling；
- Tauri updater 或后台自动更新；
- 自动安装或捆绑第三方 Provider；
- 独立下载服务、CDN 或下载统计；
- 改变 Archmage Runtime、Run 状态机或 Provider 协议。
