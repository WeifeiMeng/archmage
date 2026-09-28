pub mod control;

use crate::{
    config::Config,
    contract::validate_builtin_schemas,
    error::{Error, Result},
    store::{RunDefinitionRecord, Store},
    workspace::inspect_repository,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
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
    pub checks: Vec<CheckSpec>,
}

impl RunRequest {
    pub fn new(
        repository: impl AsRef<Path>,
        goal: impl Into<String>,
        checks: Vec<CheckSpec>,
    ) -> Result<Self> {
        let goal = goal.into();
        if goal.trim().is_empty() || checks.is_empty() {
            return Err(Error::Preflight(
                "goal and at least one check are required".into(),
            ));
        }
        let mut ids = BTreeSet::new();
        if checks
            .iter()
            .any(|check| !ids.insert(check.check_id.as_str()))
        {
            return Err(Error::Preflight("check IDs must be unique".into()));
        }
        Ok(Self {
            repository: repository.as_ref().to_path_buf(),
            goal,
            checks,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub id: String,
    pub goal: String,
    pub state: String,
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSnapshot {
    pub id: String,
    pub goal: String,
    pub repository: PathBuf,
    pub checks: Vec<CheckSpec>,
    pub state: String,
    pub revision: i64,
    pub created_at: String,
    pub last_event_sequence: i64,
    pub nodes: Vec<NodeSnapshot>,
    pub workspace: Option<PathBuf>,
}

#[derive(Clone)]
pub struct Runtime {
    config: Config,
}

#[derive(Serialize, Deserialize)]
struct FrozenRunDefinition {
    version: u32,
    repository: PathBuf,
    checks: Vec<CheckSpec>,
    nodes: Vec<String>,
}

impl Runtime {
    pub fn open(config: Config) -> Result<Self> {
        config.prepare()?;
        validate_builtin_schemas()?;
        Store::open(&config.data_root.join("archmage.db"))?;
        Ok(Self { config })
    }

    fn store(&self) -> Result<Store> {
        Store::open(&self.config.data_root.join("archmage.db"))
    }

    pub fn create_run_definition(&self, request: &RunRequest) -> Result<String> {
        let repository = inspect_repository(&request.repository)?.root;
        let definition = FrozenRunDefinition {
            version: 1,
            repository,
            checks: request.checks.clone(),
            nodes: BUILTIN_NODES.iter().map(|key| (*key).to_owned()).collect(),
        };
        let value = serde_json::to_value(definition)
            .map_err(|error| Error::Preflight(error.to_string()))?;
        self.store()?
            .create_run_definition(&request.goal, &BUILTIN_NODES, value)
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
            .map(|node| NodeSnapshot {
                id: node.id,
                key: node.key,
                state: node.state,
                revision: node.revision,
            })
            .collect();
        let workspace = store
            .workspace_for_run(run_id)
            .ok()
            .map(|item| item.path.into());
        Ok(RunSnapshot {
            id: record.id,
            goal: record.goal,
            repository: definition.repository,
            checks: definition.checks,
            state: record.state,
            revision: record.revision,
            created_at: record.created_at,
            last_event_sequence: record.last_event_sequence,
            nodes,
            workspace,
        })
    }
}

fn summary_from_record(record: RunDefinitionRecord) -> Result<RunSummary> {
    Ok(RunSummary {
        id: record.id,
        goal: record.goal,
        state: record.state,
        created_at: record.created_at,
        last_event_sequence: record.last_event_sequence,
    })
}
