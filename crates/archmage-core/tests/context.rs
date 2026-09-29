use archmage_core::{
    artifact::ArtifactStore,
    capability::CapabilityKind,
    context::{ContextRef, resolve_context, resolve_context_with_instructions},
    store::{AttemptOutcome, Store},
};
use serde_json::json;
use std::collections::HashMap;

fn committed_artifact() -> (
    tempfile::TempDir,
    Store,
    ArtifactStore,
    String,
    String,
    String,
) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("state.db")).unwrap();
    let run = store.create_run("goal", &["design", "code"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let nodes = store.nodes(&run).unwrap();
    store.ready_node(&nodes[0].id, 0).unwrap();
    let attempt = store.begin_attempt(&nodes[0].id, 1).unwrap();
    store.mark_running(&attempt.id, 0).unwrap();
    let files = ArtifactStore::open(&dir.path().join("artifacts")).unwrap();
    let artifact = files
        .publish(&mut store, &attempt.id, b"design body", "text/markdown")
        .unwrap();
    store
        .finish_attempt(&attempt.id, 1, AttemptOutcome::Succeeded(json!({})))
        .unwrap();
    (dir, store, files, run, nodes[1].id.clone(), artifact)
}

fn coder_inputs(artifact: &str) -> HashMap<String, ContextRef> {
    HashMap::from([
        (
            "goal".into(),
            ContextRef::WorkflowInput {
                value: json!({"task":"build","nested":{"b":2,"a":1}}),
            },
        ),
        (
            "design".into(),
            ContextRef::CommittedNodeOutput {
                artifact_id: artifact.into(),
            },
        ),
    ])
}

#[test]
fn map_order_is_canonical_but_prompt_version_changes_hash() {
    let (_dir, store, files, _, _, artifact) = committed_artifact();
    let first = resolve_context(
        CapabilityKind::Coder,
        "coder-v1",
        coder_inputs(&artifact),
        &files,
        &store,
    )
    .unwrap();
    let mut reversed = HashMap::new();
    reversed.insert(
        "design".into(),
        ContextRef::CommittedNodeOutput {
            artifact_id: artifact.clone(),
        },
    );
    reversed.insert(
        "goal".into(),
        ContextRef::WorkflowInput {
            value: json!({"nested":{"a":1,"b":2},"task":"build"}),
        },
    );
    let second =
        resolve_context(CapabilityKind::Coder, "coder-v1", reversed, &files, &store).unwrap();
    assert_eq!(first.hash, second.hash);
    let newer = resolve_context(
        CapabilityKind::Coder,
        "coder-v2",
        coder_inputs(&artifact),
        &files,
        &store,
    )
    .unwrap();
    assert_ne!(first.hash, newer.hash);
}

#[test]
fn capability_config_instructions_are_rendered_and_hashed_separately() {
    let (_dir, store, files, _, _, artifact) = committed_artifact();
    let base = resolve_context(
        CapabilityKind::Coder,
        "v1",
        coder_inputs(&artifact),
        &files,
        &store,
    )
    .unwrap();
    let custom = resolve_context_with_instructions(
        CapabilityKind::Coder,
        "v1",
        "Prefer small commits",
        coder_inputs(&artifact),
        &files,
        &store,
    )
    .unwrap();
    assert_ne!(base.hash, custom.hash);
    assert_eq!(
        custom.manifest["extra_instructions"],
        "Prefer small commits"
    );
    assert_eq!(
        custom.manifest["instructions"],
        CapabilityKind::Coder.prompt()
    );
}

#[test]
fn missing_predecessor_blocks_resolution() {
    let (_dir, store, files, _, _, _) = committed_artifact();
    let only_goal = HashMap::from([(
        "goal".into(),
        ContextRef::WorkflowInput { value: json!("x") },
    )]);
    assert!(resolve_context(CapabilityKind::Coder, "v1", only_goal, &files, &store).is_err());
}

#[test]
fn uncommitted_or_tampered_artifact_is_rejected() {
    let (dir, mut store, files, run, node, committed) = committed_artifact();
    std::fs::write(dir.path().join("artifacts").join(&committed), b"tampered").unwrap();
    assert!(
        resolve_context(
            CapabilityKind::Coder,
            "v1",
            coder_inputs(&committed),
            &files,
            &store
        )
        .is_err()
    );
    store.ready_node(&node, 0).unwrap();
    let attempt = store.begin_attempt(&node, 1).unwrap();
    store.mark_running(&attempt.id, 0).unwrap();
    let raw = files
        .publish(&mut store, &attempt.id, b"raw", "text/plain")
        .unwrap();
    assert!(files.read_committed(&store, &raw).is_err());
    assert!(!run.is_empty());
}

#[test]
fn context_and_handoff_are_persisted_and_bound_to_attempt() {
    let (_dir, mut store, files, run, node, artifact) = committed_artifact();
    let snapshot = resolve_context(
        CapabilityKind::Coder,
        "coder-v1",
        coder_inputs(&artifact),
        &files,
        &store,
    )
    .unwrap();
    let context_id = store.persist_context(&run, &node, &snapshot).unwrap();
    store.ready_node(&node, 0).unwrap();
    let attempt = store.begin_attempt(&node, 1).unwrap();
    store
        .bind_attempt_context(&attempt.id, &context_id)
        .unwrap();

    assert_eq!(store.attempt_context(&attempt.id).unwrap(), context_id);
    let handoffs = store.handoffs_for_context(&context_id).unwrap();
    assert_eq!(handoffs.len(), 1);
    assert_eq!(handoffs[0].input_key, "design");
    assert_eq!(handoffs[0].artifact_id, artifact);
}
