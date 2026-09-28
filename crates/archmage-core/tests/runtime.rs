use archmage_core::{
    config::Config,
    runtime::{CheckSpec, RunRequest, Runtime},
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
    let request = RunRequest::new(
        &repo,
        "add health endpoint",
        vec![CheckSpec::new("check-1", "cargo test").unwrap()],
    )
    .unwrap();

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
    assert_eq!(snapshot.checks[0].command, "cargo test");
    assert_eq!(runtime.list_runs().unwrap()[0].id, run_id);
}
