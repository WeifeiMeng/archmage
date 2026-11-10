use archmage_core::{
    capability::{CapabilityConfigurationInput, CapabilityGuidance},
    config::Config,
    process::ProcessIdentity,
    runtime::{
        NodeSnapshot, RunRequest, Runtime,
        scheduler::{
            ReviewGate, TerminalDecision, ready_keys, ready_keys_for_team, review_gate,
            terminal_decision,
        },
    },
    store::{AttemptOutcome, Store},
    team::{TeamEdge, TeamGraph, TeamNode},
};
use std::{path::Path, process::Command};

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn review_gate_requires_every_outgoing_consultation_and_aggregates_rejections() {
    let graph = TeamGraph::builtin();
    let edges: Vec<_> = graph.outgoing("designer").collect();
    assert_eq!(
        review_gate(&edges, &[]),
        ReviewGate::Awaiting(vec!["coder".into(), "tester".into(), "reviewer".into()])
    );
    let decisions = vec![
        ("coder", "approved", vec![]),
        (
            "tester",
            "changes_requested",
            vec!["Cover failure path".into()],
        ),
        (
            "reviewer",
            "changes_requested",
            vec!["Clarify acceptance".into()],
        ),
    ];
    assert_eq!(
        review_gate(&edges, &decisions),
        ReviewGate::ChangesRequested(vec![
            "Cover failure path".into(),
            "Clarify acceptance".into()
        ])
    );
}

fn node(key: &str, state: &str) -> NodeSnapshot {
    NodeSnapshot {
        id: key.to_owned(),
        key: key.to_owned(),
        state: state.to_owned(),
        revision: 0,
        result: None,
    }
}

#[test]
fn ready_set_advances_from_persisted_dependencies() {
    let initial = [
        node("designer", "pending"),
        node("coder", "pending"),
        node("tester", "pending"),
        node("reviewer", "pending"),
    ];
    assert_eq!(ready_keys(&initial), ["designer"]);

    let after_design = [
        node("designer", "succeeded"),
        node("coder", "pending"),
        node("tester", "pending"),
        node("reviewer", "pending"),
    ];
    assert_eq!(ready_keys(&after_design), ["coder"]);

    let failed = [
        node("designer", "failed"),
        node("coder", "pending"),
        node("tester", "pending"),
        node("reviewer", "pending"),
    ];
    assert!(ready_keys(&failed).is_empty());
    assert_eq!(terminal_decision(&failed), TerminalDecision::Failed);

    let complete = [
        node("designer", "succeeded"),
        node("coder", "succeeded"),
        node("tester", "succeeded"),
        node("reviewer", "succeeded"),
    ];
    assert_eq!(terminal_decision(&complete), TerminalDecision::Completed);
}

#[test]
fn two_member_scheduler_starts_coder_then_reviewer() {
    let graph = TeamGraph {
        nodes: vec![
            TeamNode {
                capability: "coder".into(),
                x: 100,
                y: 120,
            },
            TeamNode {
                capability: "reviewer".into(),
                x: 600,
                y: 120,
            },
        ],
        edges: vec![TeamEdge {
            from: "coder".into(),
            to: "reviewer".into(),
        }],
        ..TeamGraph::default()
    };
    assert_eq!(
        ready_keys_for_team(
            &[node("coder", "pending"), node("reviewer", "pending")],
            &graph
        ),
        ["coder"]
    );
    assert_eq!(
        ready_keys_for_team(
            &[node("coder", "succeeded"), node("reviewer", "pending")],
            &graph
        ),
        ["reviewer"]
    );
    assert_eq!(
        terminal_decision(&[node("coder", "succeeded"), node("reviewer", "succeeded")]),
        TerminalDecision::Completed
    );
}

#[test]
fn persisted_run_only_completes_after_all_nodes_succeed() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let run = store
        .create_run(
            "finish the workflow",
            &["designer", "coder", "tester", "reviewer"],
        )
        .unwrap();
    store.start_run(&run, 0).unwrap();
    assert!(store.complete_run(&run, 1).is_err());

    for node in store.nodes(&run).unwrap() {
        store.ready_node(&node.id, node.revision).unwrap();
        let attempt = store.begin_attempt(&node.id, node.revision + 1).unwrap();
        store.mark_running(&attempt.id, attempt.revision).unwrap();
        store
            .finish_attempt(
                &attempt.id,
                attempt.revision + 1,
                AttemptOutcome::Succeeded(serde_json::json!({"ok": true})),
            )
            .unwrap();
    }

    store.complete_run(&run, 1).unwrap();
    assert_eq!(store.run(&run).unwrap().state, "completed");
    assert_eq!(
        store.events(&run, 0).unwrap().last().unwrap().event_type,
        "workflow.completed"
    );
}

#[test]
fn reconciliation_marks_an_abandoned_attempt_interrupted() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    let runtime = Runtime::open(Config::resolve(Some(&data), None).unwrap()).unwrap();
    let mut store = Store::open(&data.join("archmage.db")).unwrap();
    let run = store.create_run("recover", &["designer"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let node = store.nodes(&run).unwrap().remove(0);
    store.ready_node(&node.id, node.revision).unwrap();
    store.begin_attempt(&node.id, node.revision + 1).unwrap();
    drop(store);

    runtime.reconcile_interrupted().unwrap();

    assert_eq!(
        Store::open(&data.join("archmage.db"))
            .unwrap()
            .run(&run)
            .unwrap()
            .state,
        "interrupted"
    );
}

#[test]
fn reconciliation_marks_an_abandoned_consultation_run_interrupted() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    let runtime = Runtime::open(Config::resolve(Some(&data), None).unwrap()).unwrap();
    let mut store = Store::open(&data.join("archmage.db")).unwrap();
    let run = store
        .create_run("recover consultation", &["designer", "coder"])
        .unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    let node = store.nodes(&run).unwrap().remove(0);
    store.ready_node(&node.id, node.revision).unwrap();
    let attempt = store.begin_attempt(&node.id, node.revision + 1).unwrap();
    store.mark_running(&attempt.id, attempt.revision).unwrap();
    store
        .finish_attempt(
            &attempt.id,
            attempt.revision + 1,
            AttemptOutcome::Succeeded(serde_json::json!({"design":"v1"})),
        )
        .unwrap();
    store
        .register_consultation_process(
            "consult-1",
            &run,
            &attempt.id,
            "coder",
            generation,
            &ProcessIdentity {
                pid: 999_999,
                pgid: 999_999,
                started: "absent".into(),
            },
        )
        .unwrap();
    drop(store);

    runtime.reconcile_interrupted().unwrap();
    let store = Store::open(&data.join("archmage.db")).unwrap();
    assert_eq!(store.run(&run).unwrap().state, "interrupted");
    assert!(
        store
            .active_consultation_processes(&run)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn start_definition_is_queryable_as_one_four_node_run() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "--quiet"]);
    git(&repo, &["config", "user.email", "archmage@example.invalid"]);
    git(&repo, &["config", "user.name", "Archmage Test"]);
    std::fs::write(repo.join("README.md"), "fixture\n").unwrap();
    git(&repo, &["add", "README.md"]);
    git(&repo, &["commit", "--quiet", "-m", "fixture"]);

    let config = Config::resolve(Some(&temp.path().join("data")), Some(&repo)).unwrap();
    let runtime = Runtime::open(config).unwrap();
    let team = runtime
        .team_service()
        .create("Custom Team", &archmage_core::team::TeamGraph::builtin())
        .unwrap();
    let request = RunRequest::new(&repo, "add health endpoint")
        .unwrap()
        .with_team(team.id.clone());

    let run_id = runtime.create_run_definition(&request).unwrap();
    let snapshot = runtime.snapshot(&run_id).unwrap();

    assert_eq!(snapshot.goal, "add health endpoint");
    assert_eq!(snapshot.repository, repo.canonicalize().unwrap());
    assert_eq!(
        snapshot
            .nodes
            .iter()
            .map(|node| node.key.as_str())
            .collect::<Vec<_>>(),
        ["designer", "coder", "tester", "reviewer"]
    );
    assert_eq!(snapshot.agent_bindings.len(), 4);
    assert_eq!(snapshot.team.as_ref().unwrap().id, team.id);
    assert_eq!(snapshot.team.as_ref().unwrap().name, "Custom Team");
    runtime
        .team_service()
        .save(&team.id, team.revision, "Changed Team", &team.graph)
        .unwrap();
    assert_eq!(
        runtime.snapshot(&run_id).unwrap().team.unwrap().name,
        "Custom Team"
    );
    assert_eq!(
        snapshot.agent_bindings["tester"].agent_instance_id,
        "codex-default"
    );
    assert_eq!(runtime.list_runs().unwrap()[0].id, run_id);
}

#[test]
fn two_member_team_freezes_only_selected_run_nodes() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "--quiet"]);
    git(&repo, &["config", "user.email", "archmage@example.invalid"]);
    git(&repo, &["config", "user.name", "Archmage Test"]);
    std::fs::write(repo.join("README.md"), "fixture\n").unwrap();
    git(&repo, &["add", "README.md"]);
    git(&repo, &["commit", "--quiet", "-m", "fixture"]);
    let runtime =
        Runtime::open(Config::resolve(Some(&temp.path().join("data")), Some(&repo)).unwrap())
            .unwrap();
    let graph = TeamGraph {
        nodes: vec![
            TeamNode {
                capability: "coder".into(),
                x: 100,
                y: 120,
            },
            TeamNode {
                capability: "reviewer".into(),
                x: 600,
                y: 120,
            },
        ],
        edges: vec![TeamEdge {
            from: "coder".into(),
            to: "reviewer".into(),
        }],
        ..TeamGraph::default()
    };
    let team = runtime
        .team_service()
        .create("Code Review", &graph)
        .unwrap();
    let id = runtime
        .create_run_definition(&RunRequest::new(&repo, "goal").unwrap().with_team(team.id))
        .unwrap();
    let run = runtime.snapshot(&id).unwrap();
    assert_eq!(
        run.nodes
            .iter()
            .map(|node| node.key.as_str())
            .collect::<Vec<_>>(),
        vec!["coder", "reviewer"]
    );
    assert_eq!(run.agent_bindings.len(), 2);
    assert_eq!(run.capability_configurations.len(), 2);
}

#[test]
fn changing_global_binding_does_not_change_existing_run() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "--quiet"]);
    git(&repo, &["config", "user.email", "archmage@example.invalid"]);
    git(&repo, &["config", "user.name", "Archmage Test"]);
    std::fs::write(repo.join("README.md"), "fixture\n").unwrap();
    git(&repo, &["add", "README.md"]);
    git(&repo, &["commit", "--quiet", "-m", "fixture"]);

    let data = temp.path().join("data");
    let runtime = Runtime::open(Config::resolve(Some(&data), Some(&repo)).unwrap()).unwrap();
    let run_id = runtime
        .create_run_definition(&RunRequest::new(&repo, "goal").unwrap())
        .unwrap();
    let mut store = Store::open(&data.join("archmage.db")).unwrap();
    store
        .save_agent_instance(&archmage_core::agent::AgentInstanceInput {
            id: "codex-work".into(),
            provider_kind: archmage_core::agent::ProviderKind::Codex,
            display_name: "Codex Work".into(),
            executable_path: None,
            provider_route: None,
            model: None,
            agent_name: None,
            provider_config: serde_json::json!({}),
            enabled: true,
        })
        .unwrap();
    store.bind_capability("tester", "codex-work").unwrap();
    let current = store.capability_configurations().unwrap().remove(2);
    store
        .save_capability_configuration(&CapabilityConfigurationInput {
            key: "tester".into(),
            display_name: "QA".into(),
            description: "New description".into(),
            extra_instructions: "Run focused checks first".into(),
            skills: vec![CapabilityGuidance {
                name: "test-design".into(),
                purpose: "Select checks".into(),
            }],
            tools: vec![CapabilityGuidance {
                name: "web_search".into(),
                purpose: "Research behavior".into(),
            }],
            agent_instance_id: "codex-work".into(),
            revision: current.revision,
        })
        .unwrap();

    let frozen = runtime.snapshot(&run_id).unwrap();
    assert_eq!(
        frozen.agent_bindings["tester"].agent_instance_id,
        "codex-default"
    );
    assert_eq!(
        frozen.capability_configurations["tester"].display_name,
        "Tester"
    );
    assert!(
        frozen.capability_configurations["tester"]
            .extra_instructions
            .is_empty()
    );
    assert_eq!(
        frozen.capability_configurations["tester"].prompt_version,
        "v1"
    );
    assert!(frozen.capability_configurations["tester"].skills.is_empty());
    assert!(frozen.capability_configurations["tester"].tools.is_empty());
    let next_run = runtime
        .create_run_definition(&RunRequest::new(&repo, "next goal").unwrap())
        .unwrap();
    let next = runtime.snapshot(&next_run).unwrap();
    assert_eq!(
        next.capability_configurations["tester"].skills[0].name,
        "test-design"
    );
    assert_eq!(
        next.capability_configurations["tester"].tools[0].name,
        "web_search"
    );

    rusqlite::Connection::open(data.join("archmage.db")).unwrap().execute(
        "UPDATE runs SET definition_json=json_remove(definition_json,'$.capabilityConfigurations') WHERE id=?1",
        [&run_id],
    ).unwrap();
    let legacy = runtime.snapshot(&run_id).unwrap();
    assert_eq!(
        legacy.capability_configurations["tester"].display_name,
        "Tester"
    );
    assert!(
        legacy.capability_configurations["tester"]
            .extra_instructions
            .is_empty()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn start_run_freezes_a_deepseek_harness_executable() {
    use archmage_core::agent::{AgentInstanceInput, ProviderKind};
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "--quiet"]);
    git(&repo, &["config", "user.email", "archmage@example.invalid"]);
    git(&repo, &["config", "user.name", "Archmage Test"]);
    std::fs::write(repo.join("README.md"), "fixture\n").unwrap();
    git(&repo, &["add", "README.md"]);
    git(&repo, &["commit", "--quiet", "-m", "fixture"]);
    let binary = temp.path().join("python-test");
    std::fs::write(
        &binary,
        "#!/bin/sh\nprintf '%s\\n' '{\"sdkVersion\":\"0.1.0\",\"runtimeVersion\":\"0.1.0\"}'\n",
    )
    .unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    let data = temp.path().join("data");
    let runtime = Runtime::open(Config::resolve(Some(&data), Some(&repo)).unwrap()).unwrap();
    let agents = runtime.agent_service();
    agents
        .save(&AgentInstanceInput {
            id: "dsh-local".into(),
            provider_kind: ProviderKind::DeepseekHarness,
            display_name: "DSH Local".into(),
            executable_path: Some(binary.canonicalize().unwrap()),
            provider_route: Some("deepseek-official".into()),
            model: Some("deepseek-v4-flash".into()),
            agent_name: None,
            provider_config: serde_json::json!({}),
            enabled: true,
        })
        .unwrap();
    for capability in ["designer", "coder", "tester", "reviewer"] {
        agents.bind(capability, "dsh-local").unwrap();
    }

    let run_id = runtime
        .start_run(&RunRequest::new(&repo, "goal").unwrap())
        .await
        .unwrap();
    let run = runtime.snapshot(&run_id).unwrap();
    let expected_binary = binary.canonicalize().unwrap();
    assert!(run.agent_bindings.values().all(|binding| {
        binding.provider_kind == ProviderKind::DeepseekHarness
            && binding.executable_path.as_deref() == Some(expected_binary.as_path())
            && binding.provider_route.as_deref() == Some("deepseek-official")
            && binding.model.as_deref() == Some("deepseek-v4-flash")
    }));
}
