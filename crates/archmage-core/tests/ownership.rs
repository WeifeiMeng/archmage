use archmage_core::{
    process::ProcessIdentity,
    runtime::control::RunOwnerLock,
    store::{AttemptOutcome, Store},
};
use serde_json::json;

fn active_attempt() -> (tempfile::TempDir, Store, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("state.db")).unwrap();
    let run = store.create_run("ownership", &["code"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let node = store.nodes(&run).unwrap()[0].id.clone();
    store.ready_node(&node, 0).unwrap();
    let attempt = store.begin_attempt(&node, 1).unwrap().id;
    (dir, store, run, attempt)
}

#[test]
fn only_one_os_owner_lock_exists_per_run() {
    let dir = tempfile::tempdir().unwrap();
    let first = RunOwnerLock::acquire(dir.path(), "run-one").unwrap();
    assert!(RunOwnerLock::acquire(dir.path(), "run-one").is_err());
    assert!(RunOwnerLock::acquire(dir.path(), "run-two").is_ok());
    drop(first);
    assert!(RunOwnerLock::acquire(dir.path(), "run-one").is_ok());
}

#[test]
fn stale_owner_cannot_commit_attempt_result() {
    let (_dir, mut store, run, attempt) = active_attempt();
    let first = store.claim_run_owner(&run).unwrap();
    store
        .attach_attempt_process(
            &attempt,
            0,
            first,
            &ProcessIdentity {
                pid: 123,
                pgid: 123,
                started: "identity-one".into(),
            },
        )
        .unwrap();
    let second = store.claim_run_owner(&run).unwrap();
    assert!(second > first);
    let before = store.events(&run, 0).unwrap();

    assert!(
        store
            .finish_attempt_owned(
                &attempt,
                1,
                first,
                AttemptOutcome::Succeeded(json!({"ok":true}))
            )
            .is_err()
    );

    assert_eq!(store.attempt(&attempt).unwrap().state, "running");
    assert_eq!(store.events(&run, 0).unwrap(), before);
}

#[test]
fn current_owner_can_commit_attempt_result() {
    let (_dir, mut store, run, attempt) = active_attempt();
    let generation = store.claim_run_owner(&run).unwrap();
    store
        .attach_attempt_process(
            &attempt,
            0,
            generation,
            &ProcessIdentity {
                pid: 321,
                pgid: 321,
                started: "identity-two".into(),
            },
        )
        .unwrap();

    store
        .finish_attempt_owned(
            &attempt,
            1,
            generation,
            AttemptOutcome::Succeeded(json!({"ok":true})),
        )
        .unwrap();

    assert_eq!(store.attempt(&attempt).unwrap().state, "succeeded");
}
