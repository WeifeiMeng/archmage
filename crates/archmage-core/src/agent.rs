use crate::{
    error::{Error, Result},
    provider::{codex, deepseek_harness, qoder},
    store::{AgentObservation, Store},
};
use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf, str::FromStr};

pub const BUILTIN_CAPABILITIES: [&str; 4] = ["designer", "coder", "tester", "reviewer"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Codex,
    #[serde(rename = "deepseek_harness")]
    DeepseekHarness,
    Qoder,
    Claude,
    OpenClaw,
    Hermes,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::DeepseekHarness => "deepseek_harness",
            Self::Qoder => "qoder",
            Self::Claude => "claude",
            Self::OpenClaw => "openclaw",
            Self::Hermes => "hermes",
        }
    }

    pub fn supported(self) -> bool {
        matches!(self, Self::Codex | Self::DeepseekHarness | Self::Qoder)
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ProviderKind {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "codex" => Ok(Self::Codex),
            "deepseek_harness" => Ok(Self::DeepseekHarness),
            "qoder" => Ok(Self::Qoder),
            "claude" => Ok(Self::Claude),
            "openclaw" => Ok(Self::OpenClaw),
            "hermes" => Ok(Self::Hermes),
            _ => Err(Error::Preflight(format!("unknown provider kind: {value}"))),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Unverified,
    Ready,
    Failed,
}

impl ConnectionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unverified => "unverified",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }
}

impl FromStr for ConnectionState {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "unverified" => Ok(Self::Unverified),
            "ready" => Ok(Self::Ready),
            "failed" => Ok(Self::Failed),
            _ => Err(Error::Preflight(format!(
                "unknown connection state: {value}"
            ))),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInstanceInput {
    pub id: String,
    pub provider_kind: ProviderKind,
    pub display_name: String,
    pub executable_path: Option<PathBuf>,
    #[serde(default)]
    pub provider_route: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub agent_name: Option<String>,
    #[serde(default = "empty_provider_config")]
    pub provider_config: serde_json::Value,
    pub enabled: bool,
}

fn empty_provider_config() -> serde_json::Value {
    serde_json::json!({})
}

impl AgentInstanceInput {
    pub fn validate(&self) -> Result<()> {
        let valid_id = !self.id.is_empty()
            && self.id.len() <= 80
            && self
                .id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
        if !valid_id || self.display_name.trim().is_empty() {
            return Err(Error::Preflight(
                "agent ID and display name must be valid and nonblank".into(),
            ));
        }
        if !self.provider_kind.supported() {
            return Err(Error::Preflight(format!(
                "{} provider is not supported yet",
                self.provider_kind
            )));
        }
        if self
            .model
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(Error::Preflight("agent model cannot be blank".into()));
        }
        if self
            .provider_route
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(Error::Preflight(
                "agent provider route cannot be blank".into(),
            ));
        }
        if self
            .agent_name
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(Error::Preflight("agent name cannot be blank".into()));
        }
        if !self.provider_config.is_object() {
            return Err(Error::Preflight(
                "agent provider config must be a JSON object".into(),
            ));
        }
        match self.provider_kind {
            ProviderKind::Codex | ProviderKind::Qoder if self.provider_route.is_some() => {
                return Err(Error::Preflight(format!(
                    "{} does not use a provider route",
                    self.provider_kind
                )));
            }
            ProviderKind::DeepseekHarness
                if self.provider_route.is_none() || self.model.is_none() =>
            {
                return Err(Error::Preflight(
                    "DeepSeek Harness provider route and model are required".into(),
                ));
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInstance {
    pub id: String,
    pub provider_kind: ProviderKind,
    pub display_name: String,
    pub executable_path: Option<PathBuf>,
    pub provider_route: Option<String>,
    pub model: Option<String>,
    pub agent_name: Option<String>,
    pub provider_config: serde_json::Value,
    pub enabled: bool,
    pub connection_state: ConnectionState,
    pub discovered_version: Option<String>,
    pub last_verified_at: Option<String>,
    pub last_error: Option<String>,
    pub verification_thread_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityBinding {
    pub capability: String,
    pub agent_instance_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterDescriptor {
    pub kind: ProviderKind,
    pub display_name: String,
    pub supported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentManagementSnapshot {
    pub adapters: Vec<AdapterDescriptor>,
    pub instances: Vec<AgentInstance>,
    pub bindings: Vec<CapabilityBinding>,
}

pub fn adapter_descriptors() -> Vec<AdapterDescriptor> {
    [
        (ProviderKind::Codex, "Codex"),
        (ProviderKind::DeepseekHarness, "DeepSeek Harness"),
        (ProviderKind::Qoder, "Qoder"),
        (ProviderKind::Claude, "Claude"),
        (ProviderKind::OpenClaw, "OpenClaw"),
        (ProviderKind::Hermes, "Hermes"),
    ]
    .into_iter()
    .map(|(kind, name)| AdapterDescriptor {
        kind,
        display_name: name.into(),
        supported: kind.supported(),
    })
    .collect()
}

#[derive(Clone)]
pub struct AgentService {
    data_root: PathBuf,
}

impl AgentService {
    pub fn new(data_root: PathBuf) -> Self {
        Self { data_root }
    }

    fn store(&self) -> Result<Store> {
        Store::open(&self.data_root.join("archmage.db"))
    }

    pub fn snapshot(&self) -> Result<AgentManagementSnapshot> {
        self.store()?.agent_management_snapshot()
    }

    pub fn save(&self, input: &AgentInstanceInput) -> Result<AgentManagementSnapshot> {
        let mut store = self.store()?;
        store.save_agent_instance(input)?;
        store.agent_management_snapshot()
    }

    pub fn delete(&self, id: &str) -> Result<AgentManagementSnapshot> {
        let mut store = self.store()?;
        store.delete_agent_instance(id)?;
        store.agent_management_snapshot()
    }

    pub fn bind(
        &self,
        capability: &str,
        agent_instance_id: &str,
    ) -> Result<AgentManagementSnapshot> {
        let mut store = self.store()?;
        store.bind_capability(capability, agent_instance_id)?;
        store.agent_management_snapshot()
    }

    pub async fn discover(&self, id: &str) -> Result<AgentManagementSnapshot> {
        let instance = self.store()?.agent_instance(id)?;
        if !instance.enabled {
            return Err(Error::Preflight("agent instance is disabled".into()));
        }
        let discovered = match instance.provider_kind {
            ProviderKind::Codex => codex::inspect_at(instance.executable_path.as_deref())
                .await
                .map(|tools| tools.codex_version),
            ProviderKind::DeepseekHarness => {
                deepseek_harness::inspect_at(instance.executable_path.as_deref())
                    .await
                    .map(|tools| deepseek_harness::python_version_label(&tools))
            }
            ProviderKind::Qoder => qoder::inspect_at(instance.executable_path.as_deref())
                .await
                .map(|tools| tools.version),
            kind => {
                return Err(Error::Preflight(format!(
                    "{kind} provider is not supported yet"
                )));
            }
        };
        match discovered {
            Ok(version) => {
                self.store()?.record_agent_observation_if_current(
                    &instance,
                    AgentObservation::Discovered(&version),
                )?;
            }
            Err(_) => {
                let message = match instance.provider_kind {
                    ProviderKind::Codex => "Codex executable inspection failed",
                    ProviderKind::DeepseekHarness => {
                        "DeepSeek Harness Python SDK inspection failed"
                    }
                    ProviderKind::Qoder => "Qoder executable inspection failed",
                    _ => "Agent inspection failed",
                };
                self.store()?.record_agent_observation_if_current(
                    &instance,
                    AgentObservation::DiscoveryFailed(message),
                )?;
            }
        }
        self.snapshot()
    }

    pub async fn verify(&self, id: &str) -> Result<AgentManagementSnapshot> {
        let instance = self.store()?.agent_instance(id)?;
        if !instance.enabled {
            return Err(Error::Preflight("agent instance is disabled".into()));
        }
        if instance.provider_kind == ProviderKind::DeepseekHarness {
            return self.verify_deepseek_harness(&instance).await;
        }
        if instance.provider_kind == ProviderKind::Qoder {
            return self.verify_qoder(&instance).await;
        }
        if instance.provider_kind != ProviderKind::Codex {
            return Err(Error::Preflight(format!(
                "{} provider is not supported yet",
                instance.provider_kind
            )));
        }

        let tools = match codex::inspect_at(instance.executable_path.as_deref()).await {
            Ok(tools) => tools,
            Err(_) => {
                self.store()?.record_agent_observation_if_current(
                    &instance,
                    AgentObservation::Failed("Codex executable inspection failed"),
                )?;
                return self.snapshot();
            }
        };
        let probes = self.data_root.join("probes").join(id);
        private_directory(&probes)?;
        match codex::readonly_probe_logged_with_model(&tools, &probes, instance.model.as_deref())
            .await
        {
            Ok(probe) => {
                self.store()?.record_agent_observation_if_current(
                    &instance,
                    AgentObservation::Ready {
                        version: &tools.codex_version,
                        thread_id: &probe.thread_id,
                    },
                )?;
                self.snapshot()
            }
            Err(_) => {
                self.store()?.record_agent_observation_if_current(
                    &instance,
                    AgentObservation::Failed("Codex connection verification failed"),
                )?;
                self.snapshot()
            }
        }
    }

    async fn verify_deepseek_harness(
        &self,
        instance: &AgentInstance,
    ) -> Result<AgentManagementSnapshot> {
        let tools = match deepseek_harness::inspect_at(instance.executable_path.as_deref()).await {
            Ok(tools) => tools,
            Err(_) => {
                self.store()?.record_agent_observation_if_current(
                    instance,
                    AgentObservation::Failed("DeepSeek Harness executable inspection failed"),
                )?;
                return self.snapshot();
            }
        };
        let root = self.data_root.join("probes").join(&instance.id);
        private_directory(&root)?;
        let probe = tempfile::Builder::new()
            .prefix("probe-")
            .tempdir_in(&root)?;
        let session_id = format!("archmage-probe-{}", uuid::Uuid::new_v4());
        let config = deepseek_harness::SidecarConfig {
            python_binary: tools.python_binary.clone(),
            data_root: self.data_root.clone(),
            repository: probe.path().to_path_buf(),
            provider_route: instance.provider_route.clone().ok_or_else(|| {
                Error::Preflight("DeepSeek Harness provider route is required".into())
            })?,
            model: instance
                .model
                .clone()
                .ok_or_else(|| Error::Preflight("DeepSeek Harness model is required".into()))?,
            session_id: session_id.clone(),
            permission_mode: deepseek_harness::PermissionMode::ReadOnly,
            session_root: probe.path().join("sessions"),
            diagnostics_dir: probe.path().join("diagnostics"),
            deadline: std::time::Duration::from_secs(60),
            max_line_bytes: 1024 * 1024,
            max_stream_bytes: 4 * 1024 * 1024,
            max_result_bytes: 1024 * 1024,
            environment: std::collections::BTreeMap::new(),
        };
        let result = async {
            let mut sidecar = deepseek_harness::DeepseekHarnessSidecar::start(config).await?;
            let outcome = match sidecar
                .invoke(
                    "verification",
                    b"Return the JSON object {\"ok\":true}. Do not edit files or run commands.",
                    &serde_json::json!({"type":"object","properties":{"ok":{"type":"boolean"}},"required":["ok"],"additionalProperties":false}),
                )
                .await
            {
                Ok(outcome) => outcome,
                Err(error) => {
                    let _ = sidecar.cancel().await;
                    return Err(error);
                }
            };
            sidecar.close().await?;
            Ok::<_, Error>(outcome)
        }
        .await;
        match result {
            Ok(outcome) if outcome.result["ok"] == true => {
                self.store()?.record_agent_observation_if_current(
                    instance,
                    AgentObservation::Ready {
                        version: &deepseek_harness::python_version_label(&tools),
                        thread_id: &session_id,
                    },
                )?;
            }
            _ => {
                self.store()?.record_agent_observation_if_current(
                    instance,
                    AgentObservation::Failed("DeepSeek Harness connection verification failed"),
                )?;
            }
        }
        self.snapshot()
    }

    async fn verify_qoder(&self, instance: &AgentInstance) -> Result<AgentManagementSnapshot> {
        let tools = match qoder::inspect_at(instance.executable_path.as_deref()).await {
            Ok(tools) => tools,
            Err(_) => {
                self.store()?.record_agent_observation_if_current(
                    instance,
                    AgentObservation::Failed("Qoder executable inspection failed"),
                )?;
                return self.snapshot();
            }
        };
        let root = self.data_root.join("probes").join(&instance.id);
        private_directory(&root)?;
        let probe = tempfile::Builder::new()
            .prefix("probe-")
            .tempdir_in(&root)?;
        let diagnostics = tempfile::Builder::new()
            .prefix("diagnostics-")
            .tempdir_in(&root)?;
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(probe.path())
            .status()
            .map_err(Error::Io)?;
        let config = qoder::QoderSessionConfig {
            binary: tools.binary.clone(),
            repository: probe.path().to_path_buf(),
            model: instance.model.clone(),
            agent_name: instance.agent_name.clone(),
            provider_config: instance.provider_config.clone(),
            access: crate::provider::acp::AcpAccess::ReadOnly,
            diagnostics_dir: diagnostics.path().to_path_buf(),
            deadline: std::time::Duration::from_secs(60),
            max_line_bytes: 1024 * 1024,
            max_stream_bytes: 4 * 1024 * 1024,
            max_result_bytes: 1024 * 1024,
            environment: std::collections::BTreeMap::new(),
        };
        let result = async {
            let mut session = qoder::QoderSession::start(config).await?;
            let outcome = match session
                .prompt(
                    "verification",
                    b"Return exactly the JSON object {\"ok\":true}. Do not edit files or run commands.",
                )
                .await
            {
                Ok(outcome) => outcome,
                Err(error) => {
                    let _ = session.cancel().await;
                    return Err(error);
                }
            };
            session.close().await?;
            Ok::<_, Error>(outcome)
        }
        .await;
        match result {
            Ok(outcome) if outcome.result == serde_json::json!({"ok": true}) => {
                self.store()?.record_agent_observation_if_current(
                    instance,
                    AgentObservation::Ready {
                        version: &tools.version,
                        thread_id: &outcome.conversation_id,
                    },
                )?;
            }
            _ => {
                self.store()?.record_agent_observation_if_current(
                    instance,
                    AgentObservation::Failed("Qoder connection verification failed"),
                )?;
            }
        }
        self.snapshot()
    }
}

fn private_directory(path: &std::path::Path) -> Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    Ok(())
}
