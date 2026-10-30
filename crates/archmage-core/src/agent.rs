use crate::{
    error::{Error, Result},
    provider::codex,
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
    Claude,
    OpenClaw,
    Hermes,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::DeepseekHarness => "deepseek_harness",
            Self::Claude => "claude",
            Self::OpenClaw => "openclaw",
            Self::Hermes => "hermes",
        }
    }

    pub fn supported(self) -> bool {
        matches!(self, Self::Codex | Self::DeepseekHarness)
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
    pub model: Option<String>,
    pub enabled: bool,
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
        if self.provider_kind == ProviderKind::DeepseekHarness && self.model.is_some() {
            return Err(Error::Preflight(
                "DeepSeek Harness uses the model selected by its inherited DSH_HOME".into(),
            ));
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
    pub model: Option<String>,
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
        if instance.provider_kind != ProviderKind::Codex {
            return Err(Error::Preflight(format!(
                "{} provider is not supported yet",
                instance.provider_kind
            )));
        }
        match codex::inspect_at(instance.executable_path.as_deref()).await {
            Ok(tools) => {
                self.store()?.record_agent_observation_if_current(
                    &instance,
                    AgentObservation::Discovered(&tools.codex_version),
                )?;
            }
            Err(_) => {
                self.store()?.record_agent_observation_if_current(
                    &instance,
                    AgentObservation::DiscoveryFailed,
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
