use archmage_core::{
    artifact::ArtifactStore,
    capability::{CapabilityGuidance, CapabilityKind},
    context::{
        ContextRef, resolve_context, resolve_context_for_flow, resolve_context_for_team,
        resolve_context_with_feedback, resolve_context_with_instructions,
        resolve_context_with_revision,
    },
    store::{AttemptOutcome, Store},
};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn flow_context_accepts_reviewer_feedback_without_designer_handoff() {
    let (_dir, store, files, _, _, artifact) = committed_artifact();
    let context = resolve_context_for_flow(
        CapabilityKind::Coder,
        "v1",
        "Follow the feedback",
        &[],
        &[],
        Some(&json!({"status":"implemented"})),
        HashMap::from([
            (
                "goal".into(),
                ContextRef::WorkflowInput {
                    value: json!("build"),
                },
            ),
            (
                "repo_view".into(),
                ContextRef::WorkspaceSnapshot {
                    value: json!({"files":[]}),
                },
            ),
            (
                "incoming".into(),
                ContextRef::CommittedNodeOutput {
                    artifact_id: artifact.clone(),
                },
            ),
            (
                "previous_result".into(),
                ContextRef::CommittedNodeOutput {
                    artifact_id: artifact,
                },
            ),
        ]),
        &files,
        &store,
    )
    .unwrap();
    assert!(context.manifest["bindings"]["incoming"].is_object());
    assert_eq!(context.manifest["previous_output"]["status"], "implemented");
    assert_eq!(context.handoffs.len(), 2);
    assert!(context.manifest["bindings"]["previous_result"].is_object());
    assert!(context.manifest["bindings"].get("design").is_none());
}

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
fn selected_team_resolves_coder_without_designer_and_reviewer_without_tester() {
    let (_dir, store, files, _, _, artifact) = committed_artifact();
    let coder = resolve_context_for_team(
        CapabilityKind::Coder,
        "v1",
        "",
        &[],
        &[],
        &[],
        None,
        HashMap::from([
            (
                "goal".into(),
                ContextRef::WorkflowInput {
                    value: json!("build"),
                },
            ),
            (
                "repo_view".into(),
                ContextRef::WorkspaceSnapshot {
                    value: json!({"files":[]}),
                },
            ),
        ]),
        &files,
        &store,
    )
    .unwrap();
    assert!(coder.manifest["bindings"]["repo_view"].is_object());
    assert!(coder.manifest["bindings"].get("design").is_none());
    let reviewer = resolve_context_for_team(
        CapabilityKind::Reviewer,
        "v1",
        "",
        &[],
        &[],
        &[],
        None,
        HashMap::from([
            (
                "goal".into(),
                ContextRef::WorkflowInput {
                    value: json!("build"),
                },
            ),
            (
                "implementation".into(),
                ContextRef::CommittedNodeOutput {
                    artifact_id: artifact.clone(),
                },
            ),
            (
                "changeset".into(),
                ContextRef::Artifact {
                    artifact_id: artifact,
                },
            ),
        ]),
        &files,
        &store,
    )
    .unwrap();
    assert!(reviewer.manifest["bindings"]["implementation"].is_object());
    assert!(reviewer.manifest["bindings"].get("test_result").is_none());
}

#[test]
fn capability_guidance_is_rendered_as_advisory_context() {
    let (_dir, store, files, _, _, artifact) = committed_artifact();
    let guidance = CapabilityGuidance {
        name: "brainstorming".into(),
        purpose: "Explore options".into(),
    };
    let tool = CapabilityGuidance {
        name: "web_search".into(),
        purpose: "Research uncertain facts".into(),
    };
    let context = resolve_context_with_revision(
        CapabilityKind::Coder,
        "v1",
        "Prefer clear code",
        &[guidance],
        &[tool],
        &[],
        None,
        coder_inputs(&artifact),
        &files,
        &store,
    )
    .unwrap();
    assert_eq!(context.manifest["skills"][0]["name"], "brainstorming");
    assert_eq!(context.manifest["tools"][0]["name"], "web_search");
    assert!(
        context.manifest["guidance_note"]
            .as_str()
            .unwrap()
            .contains("not an availability or permission guarantee")
    );
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
fn revision_feedback_is_a_separate_hashed_context_field() {
    let (_dir, store, files, _, _, artifact) = committed_artifact();
    let result = resolve_context_with_feedback(
        CapabilityKind::Coder,
        "v1",
        "",
        &["Handle errors".into()],
        coder_inputs(&artifact),
        &files,
        &store,
    )
    .unwrap();
    assert_eq!(
        result.manifest["revision_feedback"],
        json!(["Handle errors"])
    );
    assert_eq!(
        result.manifest["instructions"],
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
