# 在 macOS 上安装 Archmage

Archmage 1.0.0 是面向运行 **macOS 14 或更高版本**的 **Apple Silicon** Mac 的预览版，不兼容 Intel Mac。由于项目目前没有 Apple Developer 身份，当前安装包**未签名**、未公证，也不包含自动更新器。

## 1. 下载并校验

打开 [GitHub Releases](https://github.com/WeifeiMeng/archmage/releases)，同时下载：

- `Archmage_1.0.0_aarch64.dmg`
- `Archmage_1.0.0_aarch64.dmg.sha256`

在终端中进入下载目录，并在打开 DMG 前执行：

```bash
shasum -a 256 -c Archmage_1.0.0_aarch64.dmg.sha256
```

仅在结果显示 `OK` 时继续。如果校验失败，请删除两个文件并重新下载。

## 2. 安装和首次打开

1. 打开 `Archmage_1.0.0_aarch64.dmg`。
2. 将 `Archmage.app` 拖入“应用程序”。
3. 在 Finder 的“应用程序”中右键 Archmage，选择“打开”，然后再次确认“打开”。
4. 如果 macOS 仍然阻止启动，请前往“系统设置 → 隐私与安全性”，找到 Archmage 提示并选择“仍要打开”。

这是未签名预览版的预期首次启动流程。使用 `xattr` 删除隔离属性不是常规安装方式：它会移除操作系统提供的安全信号，也更容易误开未经验证的下载。应先校验 SHA-256，再通过 Finder 或“隐私与安全性”打开。

## 3. 完成首次启动并配置 Provider

Archmage 不会内置 Codex 或其他 Agent Provider。Provider 配置仍是独立、由用户控制的过程：

1. 通过 Provider 自己的可信发布渠道完成安装；
2. 如有需要，在 Archmage 之外完成 Provider 认证；
3. 打开 Archmage 的 Agent 管理，发现或添加 Provider；
4. 执行验证，并确认界面显示已验证后，再将其绑定到 Capability。

“已安装”“已认证”和“已验证”是不同状态。Provider 缺失或未认证时应显示配置指引；重新安装 Archmage DMG 不会修复 Provider 本身。

## 4. 手动升级和数据保留

Archmage 当前仅支持**手动升级**：

1. 停止正在运行的 Run 并退出 Archmage；
2. 备份 `~/Library/Application Support/dev.archmage.archmage`；
3. 下载并校验新版 DMG；
4. 替换“应用程序”中的 `Archmage.app`。

替换应用不会删除 Application Support 目录，其中的 Run、Team、Capability 配置、Agent 记录、Artifact 和托管工作区应继续保留。不要把数据目录移动到 `.app` bundle 内。

降级前应另存一份备份。新版可能将数据库或 Artifact 迁移到旧版无法读取的 schema。项目不保证原地降级兼容；恢复与旧版对应的备份才是安全的回滚边界。

## 5. 卸载

如果只是**仅卸载应用**，请退出 Archmage，并将 `/Applications/Archmage.app` 移到废纸篓。这样会保留 Application Support 数据，之后重新安装仍可使用。

删除 `~/Library/Application Support/dev.archmage.archmage` 是另一项不可恢复的数据清理操作，会删除 Archmage 数据库、Run、Artifact、配置和托管工作区。请先备份，确认没有 Run 或 Agent 进程仍在运行，并且仅在明确要清除全部本地 Archmage 数据时删除它。Provider 应用及其凭据需单独管理。

## 6. 故障排查

- **架构不匹配：** 执行 `uname -m`；支持的结果是 `arm64`。此版本不能在 `x86_64` Intel Mac 上运行。
- **系统版本不支持：** 在“关于本机”中确认系统为 macOS 14 或更高版本。
- **Gatekeeper 阻止启动：** 重新校验 SHA-256，然后通过 Finder 右键“打开”，或选择“隐私与安全性 → 仍要打开”。
- **提示 DMG“已损坏”或校验失败：** 删除 DMG 和校验文件，然后从 GitHub Releases 重新下载；不要关闭 Gatekeeper。
- **Provider 缺失或未验证：** 单独安装并认证该 Provider，再回到 Agent 管理执行验证。
- **应用可打开但旧数据缺失：** 退出应用，检查启动前是否设置了指向其他目录的 `ARCHMAGE_HOME`。
