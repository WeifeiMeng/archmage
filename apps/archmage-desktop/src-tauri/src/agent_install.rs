use archmage_core::{
    agent::ProviderKind,
    provider::{codex, deepseek_harness},
};
use serde::Serialize;

const CODEX_DOCS: &str = "https://github.com/openai/codex#quickstart";
const DSH_DOCS: &str =
    "https://github.com/deepseek-ai/deepseek-harness/blob/master/python/sdk/README.md";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInstallGuidance {
    pub provider_kind: ProviderKind,
    pub platform: String,
    pub architecture: String,
    pub supported: bool,
    pub detected: bool,
    pub prerequisite: Option<String>,
    pub command: Option<String>,
    pub documentation_url: String,
    pub unsupported_reason: Option<String>,
}

pub fn resolve(
    provider_kind: ProviderKind,
    platform: &str,
    architecture: &str,
    macos_major: Option<u32>,
    detected: bool,
) -> AgentInstallGuidance {
    let (supported, prerequisite, command, documentation_url, unsupported_reason) =
        match provider_kind {
            ProviderKind::Codex => match platform {
                "macos" => (
                    true,
                    None,
                    Some("curl -fsSL https://chatgpt.com/codex/install.sh | sh".into()),
                    CODEX_DOCS,
                    None,
                ),
                "windows" => (
                    true,
                    None,
                    Some("powershell -ExecutionPolicy ByPass -c \"irm https://chatgpt.com/codex/install.ps1 | iex\"".into()),
                    CODEX_DOCS,
                    None,
                ),
                _ => (
                    false,
                    None,
                    None,
                    CODEX_DOCS,
                    Some("当前安装指引仅覆盖 macOS 和 Windows。".into()),
                ),
            },
            ProviderKind::DeepseekHarness => {
                let supported = platform == "macos"
                    && architecture == "aarch64"
                    && macos_major.is_some_and(|major| major >= 14);
                let reason = if supported {
                    None
                } else if platform == "windows" {
                    Some("当前 DeepSeek Harness Python SDK 暂不支持 Windows Agent。".into())
                } else if platform == "macos" && architecture != "aarch64" {
                    Some("当前 DeepSeek Harness Python SDK 仅支持 Apple Silicon Mac。".into())
                } else if platform == "macos" {
                    Some("DeepSeek Harness Python SDK 需要 macOS 14 或更高版本。".into())
                } else {
                    Some("当前安装指引仅覆盖受支持的 Apple Silicon Mac。".into())
                };
                (
                    supported,
                    Some("Python 3.10 或更高版本".into()),
                    supported.then(|| "python3 -m pip install deepseek-harness-sdk".into()),
                    DSH_DOCS,
                    reason,
                )
            }
            _ => (
                false,
                None,
                None,
                "https://github.com/",
                Some("该 Provider 尚未提供安装指引。".into()),
            ),
        };

    AgentInstallGuidance {
        provider_kind,
        platform: platform.into(),
        architecture: architecture.into(),
        supported,
        detected,
        prerequisite,
        command,
        documentation_url: documentation_url.into(),
        unsupported_reason,
    }
}

pub async fn list() -> Vec<AgentInstallGuidance> {
    let platform = std::env::consts::OS;
    let architecture = std::env::consts::ARCH;
    let macos_major = macos_major_version();
    let codex_detected = codex::find_executable("codex").is_ok();
    let dsh_supported = platform == "macos"
        && architecture == "aarch64"
        && macos_major.is_some_and(|major| major >= 14);
    let dsh_detected = if dsh_supported {
        deepseek_harness::inspect_at(None).await.is_ok()
    } else {
        false
    };

    vec![
        resolve(
            ProviderKind::Codex,
            platform,
            architecture,
            macos_major,
            codex_detected,
        ),
        resolve(
            ProviderKind::DeepseekHarness,
            platform,
            architecture,
            macos_major,
            dsh_detected,
        ),
    ]
}

#[cfg(target_os = "macos")]
fn macos_major_version() -> Option<u32> {
    let output = std::process::Command::new("/usr/bin/sw_vers")
        .arg("-productVersion")
        .output()
        .ok()?;
    output.status.success().then_some(())?;
    String::from_utf8(output.stdout)
        .ok()?
        .trim()
        .split('.')
        .next()?
        .parse()
        .ok()
}

#[cfg(not(target_os = "macos"))]
fn macos_major_version() -> Option<u32> {
    None
}

#[cfg(test)]
mod tests {
    use super::resolve;
    use archmage_core::agent::ProviderKind;

    #[test]
    fn resolves_codex_installers_for_macos_and_windows() {
        assert_eq!(
            resolve(ProviderKind::Codex, "macos", "aarch64", Some(14), false)
                .command
                .as_deref(),
            Some("curl -fsSL https://chatgpt.com/codex/install.sh | sh")
        );
        assert_eq!(
            resolve(ProviderKind::Codex, "windows", "x86_64", None, false)
                .command
                .as_deref(),
            Some(
                "powershell -ExecutionPolicy ByPass -c \"irm https://chatgpt.com/codex/install.ps1 | iex\""
            )
        );
    }

    #[test]
    fn limits_deepseek_harness_to_supported_macs() {
        let supported = resolve(
            ProviderKind::DeepseekHarness,
            "macos",
            "aarch64",
            Some(14),
            false,
        );
        assert!(supported.supported);
        assert_eq!(
            supported.command.as_deref(),
            Some("python3 -m pip install deepseek-harness-sdk")
        );
        assert!(
            !resolve(
                ProviderKind::DeepseekHarness,
                "windows",
                "x86_64",
                None,
                false
            )
            .supported
        );
        assert!(
            !resolve(
                ProviderKind::DeepseekHarness,
                "macos",
                "x86_64",
                Some(14),
                false
            )
            .supported
        );
        assert!(
            !resolve(
                ProviderKind::DeepseekHarness,
                "macos",
                "aarch64",
                Some(13),
                false
            )
            .supported
        );
    }
}
