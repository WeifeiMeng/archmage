pub mod control;
mod dispatch;
mod recovery;
pub mod scheduler;

use crate::{
    agent::{AgentManagementSnapshot, AgentService, ProviderKind},
    capability::{CapabilityConfiguration, CapabilityService},
    config::Config,
    contract::validate_builtin_schemas,
    error::{Error, Result},
    store::{RunDefinitionRecord, Store},
    workspace::inspect_repository,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

const BUILTIN_NODES: [&str; 4] = ["designer", "coder", "tester", "reviewer"];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckSpec {
    pub check_id: String,
    pub command: String,
}

impl CheckSpec {
    pub fn new(check_id: impl Into<String>, command: impl Into<String>) -> Result<Self> {
        let check = Self {
            check_id: check_id.into(),
            command: command.into(),
        };
        if check.check_id.trim().is_empty() || check.command.trim().is_empty() {
            return Err(Error::Preflight(
                "check ID and command must be nonblank".into(),
            ));
        }
        Ok(check)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    pub repository: PathBuf,
    pub goal: String,
}

impl RunRequest {
    pub fn new(repository: impl AsRef<Path>, goal: impl Into<String>) -> Result<Self> {
        let goal = goal.into();
        if goal.trim().is_empty() {
            return Err(Error::Preflight("goal is required".into()));
        }
        Ok(Self {
            repository: repository.as_ref().to_path_buf(),
            goal,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrozenAgentBinding {
    pub agent_instance_id: String,
    pub display_name: String,
    pub provider_kind: ProviderKind,
    pub executable_path: Option<PathBuf>,
    pub model: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub id: String,
    pub goal: String,
    pub state: String,
    pub outcome: Option<String>,
    pub created_at: String,
    pub last_event_sequence: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSnapshot {
    pub id: String,
    pub key: String,
    pub state: String,
    pub revision: i64,
    pub result: Option<NodeResultSnapshot>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeResultSnapshot {
    pub attempt_id: String,
    pub thread_id: String,
    pub result_artifact_id: String,
    pub changeset_artifact_id: Option<String>,
    pub output: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSnapshot {
    pub id: String,
    pub goal: String,
    pub repository: PathBuf,
    pub checks: Vec<CheckSpec>,
    pub agent_bindings: BTreeMap<String, FrozenAgentBinding>,
    pub capability_configurations: BTreeMap<String, CapabilityConfiguration>,
    pub state: String,
    pub outcome: Option<String>,
    pub revision: i64,
    pub created_at: String,
    pub last_event_sequence: i64,
    pub nodes: Vec<NodeSnapshot>,
    pub workspace: Option<PathBuf>,
}

#[derive(Clone)]
pub struct Runtime {
    pub(crate) config: Config,
}

#[derive(Serialize, Deserialize)]
struct FrozenRunDefinition {
    pub(crate) version: u32,
    pub(crate) repository: PathBuf,
    #[serde(default)]
    pub(crate) checks: Vec<CheckSpec>,
    #[serde(default)]
    pub(crate) agent_bindings: BTreeMap<String, FrozenAgentBinding>,
    #[serde(default)]
    pub(crate) capability_configurations: BTreeMap<String, CapabilityConfiguration>,
    pub(crate) nodes: Vec<String>,
}

impl Runtime {
    pub fn open(config: Config) -> Result<Self> {
        config.prepare()?;
        validate_builtin_schemas()?;
        Store::open(&config.data_root.join("archmage.db"))?;
        Ok(Self { config })
    }

    pub(crate) fn store(&self) -> Result<Store> {
        Store::open(&self.config.data_root.join("archmage.db"))
    }

    pub fn create_run_definition(&self, request: &RunRequest) -> Result<String> {
        let repository = inspect_repository(&request.repository)?.root;
        let (bindings, configurations) = self.frozen_capability_settings()?;
        self.create_run_definition_with_bindings(request, repository, bindings, configurations)
    }

    fn create_run_definition_with_bindings(
        &self,
        request: &RunRequest,
        repository: PathBuf,
        agent_bindings: BTreeMap<String, FrozenAgentBinding>,
        capability_configurations: BTreeMap<String, CapabilityConfiguration>,
    ) -> Result<String> {
        let definition = FrozenRunDefinition {
            version: 3,
            repository,
            checks: Vec::new(),
            agent_bindings,
            capability_configurations,
            nodes: BUILTIN_NODES.iter().map(|key| (*key).to_owned()).collect(),
        };
        let value = serde_json::to_value(definition)
            .map_err(|error| Error::Preflight(error.to_string()))?;
        self.store()?
            .create_run_definition(&request.goal, &BUILTIN_NODES, value)
    }

    pub async fn start_run(&self, request: &RunRequest) -> Result<String> {
        let repository = inspect_repository(&request.repository)?.root;
        let (mut bindings, configurations) = self.frozen_capability_settings()?;
        for (capability, binding) in &mut bindings {
            match binding.provider_kind {
                ProviderKind::Codex => {
                    let tools =
                        crate::provider::codex::inspect_at(binding.executable_path.as_deref())
                            .await
                            .map_err(|error| {
                                Error::Preflight(format!(
                                    "capability {capability} uses Agent {}: {error}",
                                    binding.agent_instance_id
                                ))
                            })?;
                    binding.executable_path = Some(tools.codex_binary);
                }
                kind => {
                    return Err(Error::Preflight(format!(
                        "capability {capability} uses unsupported {kind} provider"
                    )));
                }
            }
        }
        let run_id = self.create_run_definition_with_bindings(
            request,
            repository,
            bindings,
            configurations,
        )?;
        let mut store = self.store()?;
        crate::workspace::provision(
            &mut store,
            &run_id,
            &request.repository,
            &self.config.data_root.join("worktrees"),
        )?;
        Ok(run_id)
    }

    pub fn list_runs(&self) -> Result<Vec<RunSummary>> {
        self.store()?
            .run_definitions()?
            .into_iter()
            .map(summary_from_record)
            .collect()
    }

    pub fn snapshot(&self, run_id: &str) -> Result<RunSnapshot> {
        let store = self.store()?;
        let record = store.run_definition(run_id)?;
        let definition: FrozenRunDefinition = serde_json::from_value(record.definition.clone())
            .map_err(|error| Error::Preflight(format!("invalid run definition: {error}")))?;
        let nodes = store
            .nodes(run_id)?
            .into_iter()
            .map(|node| {
                let result = store
                    .optional_node_result(run_id, &node.key)?
                    .map(|result| NodeResultSnapshot {
                        attempt_id: result.attempt_id,
                        thread_id: result.thread_id,
                        result_artifact_id: result.result_artifact_id,
                        changeset_artifact_id: result.changeset_artifact_id,
                        output: result.output,
                    });
                Ok(NodeSnapshot {
                    id: node.id,
                    key: node.key,
                    state: node.state,
                    revision: node.revision,
                    result,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let workspace = store
            .workspace_for_run(run_id)
            .ok()
            .map(|item| item.path.into());
        let configurations = if definition.capability_configurations.is_empty() {
            definition
                .agent_bindings
                .iter()
                .map(|(key, binding)| {
                    (
                        key.clone(),
                        CapabilityConfiguration::legacy_default(
                            key,
                            binding.agent_instance_id.clone(),
                        ),
                    )
                })
                .collect()
        } else {
            definition.capability_configurations
        };
        Ok(RunSnapshot {
            id: record.id,
            goal: record.goal,
            repository: definition.repository,
            checks: definition.checks,
            agent_bindings: definition.agent_bindings,
            capability_configurations: configurations,
            state: record.state,
            outcome: record.outcome,
            revision: record.revision,
            created_at: record.created_at,
            last_event_sequence: record.last_event_sequence,
            nodes,
            workspace,
        })
    }

    pub fn events(&self, run_id: &str, after_sequence: i64) -> Result<Vec<crate::event::Event>> {
        self.store()?.events(run_id, after_sequence)
    }

    pub fn agent_management(&self) -> Result<AgentManagementSnapshot> {
        AgentService::new(self.config.data_root.clone()).snapshot()
    }

    pub fn agent_service(&self) -> AgentService {
        AgentService::new(self.config.data_root.clone())
    }

    pub fn capability_service(&self) -> CapabilityService {
        CapabilityService::new(self.config.data_root.clone())
    }

    fn frozen_capability_settings(
        &self,
    ) -> Result<(
        BTreeMap<String, FrozenAgentBinding>,
        BTreeMap<String, CapabilityConfiguration>,
    )> {
        let store = self.store()?;
        let configurations = store.capability_configurations()?;
        let mut frozen = BTreeMap::new();
        let mut frozen_configurations = BTreeMap::new();
        for capability in BUILTIN_NODES {
            let configuration = configurations
                .iter()
                .find(|configuration| configuration.key == capability)
                .ok_or_else(|| {
                    Error::Preflight(format!("capability {capability} has no configuration"))
                })?;
            let instance = store.agent_instance(&configuration.agent_instance_id)?;
            if !instance.enabled || !instance.provider_kind.supported() {
                return Err(Error::Preflight(format!(
                    "capability {capability} is bound to an unavailable Agent"
                )));
            }
            frozen.insert(
                capability.to_owned(),
                FrozenAgentBinding {
                    agent_instance_id: instance.id,
                    display_name: instance.display_name,
                    provider_kind: instance.provider_kind,
                    executable_path: instance.executable_path,
                    model: instance.model,
                },
            );
            frozen_configurations.insert(capability.to_owned(), configuration.clone());
        }
        Ok((frozen, frozen_configurations))
    }
}

fn summary_from_record(record: RunDefinitionRecord) -> Result<RunSummary> {
    Ok(RunSummary {
        id: record.id,
        goal: record.goal,
        state: record.state,
        outcome: record.outcome,
        created_at: record.created_at,
        last_event_sequence: record.last_event_sequence,
    })
}
