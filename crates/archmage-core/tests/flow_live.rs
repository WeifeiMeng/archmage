use archmage_core::{
    config::Config,
    runtime::{RunRequest, Runtime},
    team::{TeamEdge, TeamGraph, TeamMode, TeamNode},
};
use std::{fs, path::Path, process::Command};

fn git(repo: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(repo)
            .status()
            .unwrap()
            .success()
    );
}

#[tokio::test]
#[ignore = "requires real authenticated Codex; invokes two or more paid model turns"]
async fn real_codex_coder_reviewer_flow_completes() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("source");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    git(
        &repo,
        &["config", "user.email", "archmage-flow@example.invalid"],
    );
    git(&repo, &["config", "user.name", "Archmage Flow Test"]);
    fs::write(repo.join("README.md"), "# Flow acceptance fixture\n").unwrap();
    git(&repo, &["add", "README.md"]);
    git(&repo, &["commit", "-qm", "fixture"]);

    let mut config = Config::resolve(Some(&temp.path().join("data")), Some(&repo)).unwrap();
    config.node_timeout_seconds = 300;
    let runtime = Runtime::open(config).unwrap();
    let graph = TeamGraph {
        mode: TeamMode::Flow,
        entry: Some("coder".into()),
        terminal: Some("reviewer".into()),
        nodes: vec![
            TeamNode {
                capability: "coder".into(),
                x: 100,
                y: 120,
            },
            TeamNode {
                capability: "reviewer".into(),
                x: 520,
                y: 120,
            },
        ],
        edges: vec![
            TeamEdge {
                from: "coder".into(),
                to: "reviewer".into(),
            },
            TeamEdge {
                from: "reviewer".into(),
                to: "coder".into(),
            },
        ],
    };
    let team = runtime
        .team_service()
        .create("Flow live fixture", &graph)
        .unwrap();
    let request = RunRequest::new(&repo,
        "Create hello.txt containing exactly hello followed by a newline. Send the result to Reviewer. Reviewer should inspect the worktree and approve only if the file is correct; otherwise send concrete feedback to Coder.")
        .unwrap().with_team(team.id);
    let run_id = runtime.start_run(&request).await.unwrap();
    let result = runtime.run_to_terminal(&run_id).await;
    let snapshot = runtime.snapshot(&run_id).unwrap();
    println!(
        "run={} state={} deliveries={:?}",
        run_id, snapshot.state, snapshot.flow_deliveries
    );
    assert!(result.is_ok(), "flow failed: {:?}", result.err());
    assert_eq!(snapshot.state, "completed");
    assert_eq!(snapshot.flow_deliveries.first().unwrap().target, "coder");
    assert_eq!(snapshot.flow_deliveries.last().unwrap().target, "reviewer");
    assert_eq!(
        fs::read_to_string(snapshot.workspace.unwrap().join("hello.txt")).unwrap(),
        "hello\n"
    );
}
