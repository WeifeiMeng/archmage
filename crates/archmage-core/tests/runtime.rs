use archmage_core::{
    capability::CapabilityConfigurationInput,
    config::Config,
    runtime::{
        NodeSnapshot, RunRequest, Runtime,
        scheduler::{TerminalDecision, ready_keys, terminal_decision},
    },
    store::{AttemptOutcome, Store},
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
    let request = RunRequest::new(&repo, "add health endpoint").unwrap();

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
    assert_eq!(
        snapshot.agent_bindings["tester"].agent_instance_id,
        "codex-default"
    );
    assert_eq!(runtime.list_runs().unwrap()[0].id, run_id);
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
            model: None,
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
