use crate::{
    artifact::ArtifactStore,
    capability::{CapabilityGuidance, CapabilityKind, GUIDANCE_NOTE},
    error::{Error, Result},
    store::Store,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContextRef {
    WorkflowInput { value: Value },
    CommittedNodeOutput { artifact_id: String },
    Artifact { artifact_id: String },
    WorkspaceSnapshot { value: Value },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolvedHandoff {
    pub input_key: String,
    pub artifact_id: String,
    pub content_hash: String,
    pub producer_attempt_id: String,
}

#[derive(Clone, Debug)]
pub struct ContextSnapshot {
    pub hash: String,
    pub prompt_version: String,
    pub rendered: Vec<u8>,
    pub manifest: Value,
    pub handoffs: Vec<ResolvedHandoff>,
}

fn required(kind: CapabilityKind) -> &'static [&'static str] {
    match kind {
        CapabilityKind::Designer => &["goal", "repo_view"],
        CapabilityKind::Coder => &["goal", "design"],
        CapabilityKind::Tester => &[
            "goal",
            "design",
            "implementation",
            "changeset",
            "workspace_manifest",
        ],
        CapabilityKind::Reviewer => &[
            "goal",
            "design",
            "implementation",
            "changeset",
            "test_result",
        ],
    }
}

fn canonical(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted: BTreeMap<_, _> = map.into_iter().map(|(k, v)| (k, canonical(v))).collect();
            Value::Object(sorted.into_iter().collect::<Map<_, _>>())
        }
        Value::Array(values) => Value::Array(values.into_iter().map(canonical).collect()),
        other => other,
    }
}

pub fn resolve_context(
    kind: CapabilityKind,
    prompt_version: &str,
    inputs: HashMap<String, ContextRef>,
    artifacts: &ArtifactStore,
    store: &Store,
) -> Result<ContextSnapshot> {
    resolve_context_with_instructions(kind, prompt_version, "", inputs, artifacts, store)
}

pub fn resolve_context_with_instructions(
    kind: CapabilityKind,
    prompt_version: &str,
    extra_instructions: &str,
    inputs: HashMap<String, ContextRef>,
    artifacts: &ArtifactStore,
    store: &Store,
) -> Result<ContextSnapshot> {
    resolve_context_with_feedback(
        kind,
        prompt_version,
        extra_instructions,
        &[],
        inputs,
        artifacts,
        store,
    )
}

pub fn resolve_context_with_feedback(
    kind: CapabilityKind,
    prompt_version: &str,
    extra_instructions: &str,
    revision_feedback: &[String],
    inputs: HashMap<String, ContextRef>,
    artifacts: &ArtifactStore,
    store: &Store,
) -> Result<ContextSnapshot> {
    resolve_context_with_revision(
        kind,
        prompt_version,
        extra_instructions,
        &[],
        &[],
        revision_feedback,
        None,
        inputs,
        artifacts,
        store,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn resolve_context_with_revision(
    kind: CapabilityKind,
    prompt_version: &str,
    extra_instructions: &str,
    skills: &[CapabilityGuidance],
    tools: &[CapabilityGuidance],
    revision_feedback: &[String],
    previous_output: Option<&Value>,
    inputs: HashMap<String, ContextRef>,
    artifacts: &ArtifactStore,
    store: &Store,
) -> Result<ContextSnapshot> {
    if prompt_version.trim().is_empty() {
        return Err(Error::Preflight("context prompt version is blank".into()));
    }
    let expected = required(kind);
    if inputs.len() != expected.len() || expected.iter().any(|key| !inputs.contains_key(*key)) {
        return Err(Error::Preflight(
            "context bindings are incomplete or unexpected".into(),
        ));
    }
    let mut bindings = Map::new();
    let mut handoffs = Vec::new();
    for (key, reference) in inputs {
        let resolved = match reference {
            ContextRef::WorkflowInput { value } => canonical(value),
            ContextRef::WorkspaceSnapshot { value } => canonical(value),
            ContextRef::CommittedNodeOutput { artifact_id }
            | ContextRef::Artifact { artifact_id } => {
                let content = artifacts.read_committed_info(store, &artifact_id)?;
                handoffs.push(ResolvedHandoff {
                    input_key: key.clone(),
                    artifact_id: artifact_id.clone(),
                    content_hash: content.sha256.clone(),
                    producer_attempt_id: content.producer_attempt_id,
                });
                let value: Value = serde_json::from_slice(&content.bytes).unwrap_or_else(|_| {
                    String::from_utf8_lossy(&content.bytes).into_owned().into()
                });
                json!({"artifact_id":artifact_id,"sha256":content.sha256,"content":value})
            }
        };
        bindings.insert(key, resolved);
    }
    handoffs.sort_by(|a, b| a.input_key.cmp(&b.input_key));
    let mut manifest = json!({"schema_version":1,"capability":format!("{kind:?}").to_lowercase(),"prompt_version":prompt_version,"instructions":kind.prompt(),"bindings":bindings});
    if !extra_instructions.is_empty() {
        manifest["extra_instructions"] = extra_instructions.into();
    }
    manifest["skills"] = json!(skills);
    manifest["tools"] = json!(tools);
    manifest["guidance_note"] = GUIDANCE_NOTE.into();
    if !revision_feedback.is_empty() {
        manifest["revision_feedback"] = json!(revision_feedback);
    }
    if let Some(previous_output) = previous_output {
        manifest["previous_output"] = canonical(previous_output.clone());
    }
    let manifest = canonical(manifest);
    let rendered = serde_json::to_vec(&manifest).map_err(|e| Error::Preflight(e.to_string()))?;
    let hash = format!("{:x}", Sha256::digest(&rendered));
    Ok(ContextSnapshot {
        hash,
        prompt_version: prompt_version.into(),
        rendered,
        manifest,
        handoffs,
    })
}
