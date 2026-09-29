use crate::{
    agent::AgentInstance,
    error::{Error, Result},
    store::Store,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const CAPABILITY_KEYS: [&str; 4] = ["designer", "coder", "tester", "reviewer"];
pub const BUILTIN_PROMPT_VERSION: &str = "v1";

fn default_prompt_version() -> String {
    BUILTIN_PROMPT_VERSION.into()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityConfiguration {
    pub key: String,
    pub display_name: String,
    pub description: String,
    pub extra_instructions: String,
    pub agent_instance_id: String,
    pub revision: i64,
    #[serde(default = "default_prompt_version")]
    pub prompt_version: String,
}

impl CapabilityConfiguration {
    pub fn legacy_default(key: &str, agent_instance_id: String) -> Self {
        let (display_name, description) = match key {
            "designer" => ("Designer", "分析目标，设计实现方案与验收标准"),
            "coder" => ("Coder", "根据设计完成代码实现"),
            "tester" => ("Tester", "自主选择测试方式并验证结果"),
            "reviewer" => ("Reviewer", "审查实现与测试结果"),
            _ => (key, ""),
        };
        Self {
            key: key.into(),
            display_name: display_name.into(),
            description: description.into(),
            extra_instructions: String::new(),
            agent_instance_id,
            revision: 0,
            prompt_version: default_prompt_version(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityConfigurationInput {
    pub key: String,
    pub display_name: String,
    pub description: String,
    pub extra_instructions: String,
    pub agent_instance_id: String,
    pub revision: i64,
}

impl CapabilityConfigurationInput {
    pub fn validate(&self) -> Result<()> {
        if !CAPABILITY_KEYS.contains(&self.key.as_str()) {
            return Err(Error::Preflight(format!(
                "unknown capability: {}",
                self.key
            )));
        }
        if self.display_name.trim().is_empty() || self.display_name.chars().count() > 80 {
            return Err(Error::Preflight(
                "capability display name must be 1–80 characters".into(),
            ));
        }
        if self.description.chars().count() > 500 {
            return Err(Error::Preflight(
                "capability description exceeds 500 characters".into(),
            ));
        }
        if self.extra_instructions.chars().count() > 8_000 {
            return Err(Error::Preflight(
                "capability extra instructions exceed 8000 characters".into(),
            ));
        }
        if self.agent_instance_id.trim().is_empty() || self.revision < 0 {
            return Err(Error::Preflight(
                "capability binding or revision is invalid".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyEdge {
    pub from: &'static str,
    pub to: &'static str,
    pub kind: &'static str,
}

pub fn topology_edges() -> Vec<TopologyEdge> {
    vec![
        TopologyEdge {
            from: "designer",
            to: "coder",
            kind: "execution",
        },
        TopologyEdge {
            from: "coder",
            to: "tester",
            kind: "execution",
        },
        TopologyEdge {
            from: "tester",
            to: "reviewer",
            kind: "execution",
        },
        TopologyEdge {
            from: "designer",
            to: "tester",
            kind: "context",
        },
        TopologyEdge {
            from: "designer",
            to: "reviewer",
            kind: "context",
        },
        TopologyEdge {
            from: "coder",
            to: "reviewer",
            kind: "context",
        },
    ]
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityManagementSnapshot {
    pub configurations: Vec<CapabilityConfiguration>,
    pub instances: Vec<AgentInstance>,
    pub edges: Vec<TopologyEdge>,
}

#[derive(Clone)]
pub struct CapabilityService {
    data_root: PathBuf,
}

impl CapabilityService {
    pub fn new(data_root: PathBuf) -> Self {
        Self { data_root }
    }

    fn store(&self) -> Result<Store> {
        Store::open(&self.data_root.join("archmage.db"))
    }

    pub fn snapshot(&self) -> Result<CapabilityManagementSnapshot> {
        let store = self.store()?;
        Ok(CapabilityManagementSnapshot {
            configurations: store.capability_configurations()?,
            instances: store.agent_instances()?,
            edges: topology_edges(),
        })
    }

    pub fn save(
        &self,
        input: &CapabilityConfigurationInput,
    ) -> Result<CapabilityManagementSnapshot> {
        self.store()?.save_capability_configuration(input)?;
        self.snapshot()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityKind {
    Designer,
    Coder,
    Tester,
    Reviewer,
}

impl CapabilityKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Designer => "designer",
            Self::Coder => "coder",
            Self::Tester => "tester",
            Self::Reviewer => "reviewer",
        }
    }

    pub fn prompt(self) -> &'static str {
        match self {
            Self::Designer => include_str!("../capabilities/v1/designer.md"),
            Self::Coder => include_str!("../capabilities/v1/coder.md"),
            Self::Tester => include_str!("../capabilities/v1/tester.md"),
            Self::Reviewer => include_str!("../capabilities/v1/reviewer.md"),
        }
    }
    pub fn schema(self) -> &'static str {
        match self {
            Self::Designer => include_str!("../capabilities/v1/designer.schema.json"),
            Self::Coder => include_str!("../capabilities/v1/coder.schema.json"),
            Self::Tester => include_str!("../capabilities/v1/tester.schema.json"),
            Self::Reviewer => include_str!("../capabilities/v1/reviewer.schema.json"),
        }
    }
}
