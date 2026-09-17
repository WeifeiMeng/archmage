#![cfg(unix)]

use archmage_core::{
    artifact::ArtifactStore,
    check_runner::{CheckRunner, FrozenCheck, request_check, serve_check_once},
    process::ProcessIdentity,
    store::{AttemptOutcome, Store},
    workspace::inspect_repository,
};
use std::{fs, path::Path, process::Command, time::Duration};

fn git(cwd: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .unwrap()
            .success()
    );
}

fn setup() -> (
    tempfile::TempDir,
    Store,
    ArtifactStore,
    String,
    i64,
    archmage_core::workspace::GitRepository,
) {
    let dir = tempfile::tempdir().unwrap();
    let worktree = dir.path().join("run");
    fs::create_dir(&worktree).unwrap();
    git(&worktree, &["init", "-q"]);
    git(&worktree, &["config", "user.name", "Archmage Test"]);
    git(
        &worktree,
        &["config", "user.email", "archmage@example.invalid"],
    );
    fs::write(worktree.join("base.txt"), "base").unwrap();
    git(&worktree, &["add", "base.txt"]);
    git(&worktree, &["commit", "-qm", "base"]);
    let mut store = Store::open(&dir.path().join("state.db")).unwrap();
    let run = store.create_run("checks", &["test"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    let node = store.nodes(&run).unwrap()[0].id.clone();
    store.ready_node(&node, 0).unwrap();
    let attempt = store.begin_attempt(&node, 1).unwrap();
    store
        .attach_attempt_process(
            &attempt.id,
            0,
            generation,
            &ProcessIdentity {
                pid: 1,
                pgid: 1,
                started: "test".into(),
            },
        )
        .unwrap();
    let artifacts = ArtifactStore::open(&dir.path().join("artifacts")).unwrap();
    let repo = inspect_repository(&worktree).unwrap();
    (dir, store, artifacts, attempt.id, generation, repo)
}

#[tokio::test]
async fn success_and_nonzero_exit_both_produce_canonical_evidence() {
    let (dir, mut store, artifacts, attempt, generation, repo) = setup();
    let checks = vec![
        FrozenCheck::new(
            "pass",
            "printf pass; touch generated.txt",
            Duration::from_secs(5),
        ),
        FrozenCheck::new("fail", "printf fail >&2; exit 7", Duration::from_secs(5)),
    ];
    let mut runner =
        CheckRunner::new(&mut store, &artifacts, &attempt, generation, &repo, checks).unwrap();
    let passed = runner.run("pass").await.unwrap();
    let failed = runner.run("fail").await.unwrap();
    assert_eq!(passed.exit_code, Some(0));
    assert_eq!(failed.exit_code, Some(7));
    assert_ne!(passed.workspace_before, passed.workspace_after);
    assert_eq!(passed.cwd, repo.root.to_string_lossy());
    assert_eq!(passed.timeout_ms, 5_000);
    assert!(!passed.stdout_hash.is_empty());
    assert!(!failed.stderr_hash.is_empty());
    assert_eq!(
        runner.run("pass").await.unwrap().evidence_id,
        passed.evidence_id
    );
    drop(runner);
    let connection = rusqlite::Connection::open(dir.path().join("state.db")).unwrap();
    let event_counts: (i64, i64) = connection
        .query_row(
            "SELECT SUM(event_type='check.started'),SUM(event_type='check.completed') FROM events WHERE attempt_id=?1",
            [&attempt],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(event_counts, (2, 2));
}

#[tokio::test]
async fn unknown_check_and_expired_attempt_are_rejected() {
    let (_dir, mut store, artifacts, attempt, generation, repo) = setup();
    let mut runner = CheckRunner::new(
        &mut store,
        &artifacts,
        &attempt,
        generation,
        &repo,
        vec![FrozenCheck::new("known", "true", Duration::from_secs(2))],
    )
    .unwrap();
    assert!(runner.run("unknown").await.is_err());
    drop(runner);
    store
        .finish_attempt_owned(
            &attempt,
            1,
            generation,
            AttemptOutcome::Failed("expired".into()),
        )
        .unwrap();
    let mut expired = CheckRunner::new(
        &mut store,
        &artifacts,
        &attempt,
        generation,
        &repo,
        vec![FrozenCheck::new("known", "true", Duration::from_secs(2))],
    )
    .unwrap();
    assert!(expired.run("known").await.is_err());
}

#[tokio::test]
async fn timeout_is_evidence_and_sibling_source_is_not_writable() {
    let (dir, mut store, artifacts, attempt, generation, repo) = setup();
    fs::create_dir(dir.path().join("source")).unwrap();
    let checks = vec![
        FrozenCheck::new("timeout", "sleep 5", Duration::from_millis(100)),
        FrozenCheck::new("escape", "touch ../source/escape", Duration::from_secs(5)),
    ];
    let mut runner =
        CheckRunner::new(&mut store, &artifacts, &attempt, generation, &repo, checks).unwrap();
    assert!(runner.run("timeout").await.unwrap().timed_out);
    assert_ne!(runner.run("escape").await.unwrap().exit_code, Some(0));
    assert!(!dir.path().join("source/escape").exists());
}

#[tokio::test]
async fn stale_generation_cannot_publish_completed_evidence() {
    let (dir, mut store, artifacts, attempt, generation, repo) = setup();
    let database = dir.path().join("state.db");
    let mut runner = CheckRunner::new(
        &mut store,
        &artifacts,
        &attempt,
        generation,
        &repo,
        vec![FrozenCheck::new(
            "slow",
            "sleep 0.5",
            Duration::from_secs(5),
        )],
    )
    .unwrap();
    let fence = async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let connection = rusqlite::Connection::open(database).unwrap();
        assert_eq!(
            connection
                .execute(
                    "UPDATE runs SET owner_generation=owner_generation+1 WHERE id=(SELECT n.run_id FROM attempts a JOIN node_runs n ON n.id=a.node_run_id WHERE a.id=?1)",
                    [&attempt],
                )
                .unwrap(),
            1
        );
    };
    let (result, ()) = tokio::join!(runner.run("slow"), fence);
    assert!(result.is_err());
    drop(runner);
    let connection = rusqlite::Connection::open(dir.path().join("state.db")).unwrap();
    let state: String = connection
        .query_row(
            "SELECT state FROM check_runs WHERE attempt_id=?1 AND check_id='slow'",
            [&attempt],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(state, "in_progress");
}

#[tokio::test]
async fn unix_helper_sends_only_attempt_and_frozen_check_id() {
    let (dir, mut store, artifacts, attempt, generation, repo) = setup();
    let mut runner = CheckRunner::new(
        &mut store,
        &artifacts,
        &attempt,
        generation,
        &repo,
        vec![FrozenCheck::new(
            "unit",
            "printf socket",
            Duration::from_secs(5),
        )],
    )
    .unwrap();
    let socket = dir.path().join("check.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let (served, rejected) = tokio::join!(
        serve_check_once(&listener, &mut runner),
        request_check(&socket, "other-attempt", "unit")
    );
    served.unwrap();
    assert!(rejected.is_err());
    let (served, evidence) = tokio::join!(
        serve_check_once(&listener, &mut runner),
        request_check(&socket, &attempt, "unit")
    );
    served.unwrap();
    assert_eq!(evidence.unwrap().exit_code, Some(0));
}
