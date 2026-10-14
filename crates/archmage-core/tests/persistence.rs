use archmage_core::store::{AttemptOutcome, Store};
use serde_json::json;

fn setup() -> (tempfile::TempDir, Store, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("state.db")).unwrap();
    let run = store.create_run("health", &["design", "code"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let node = store.nodes(&run).unwrap()[0].id.clone();
    store.ready_node(&node, 0).unwrap();
    (dir, store, run, node)
}

#[test]
fn migration_and_pragmas_survive_reopen() {
    let (dir, store, run, _) = setup();
    assert_eq!(store.schema_version().unwrap(), 14);
    drop(store);
    let store = Store::open(&dir.path().join("state.db")).unwrap();
    assert_eq!(store.schema_version().unwrap(), 14);
    assert_eq!(store.run(&run).unwrap().state, "running");
    let db = rusqlite::Connection::open(dir.path().join("state.db")).unwrap();
    assert_eq!(
        db.query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
}

#[test]
fn schema_thirteen_upgrades_without_losing_runs() {
    let (dir, store, run, _) = setup();
    drop(store);
    let path = dir.path().join("state.db");
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch(
            "DROP TABLE flow_deliveries; DELETE FROM schema_migrations WHERE version=14;",
        )
        .unwrap();
    let reopened = Store::open(&path).unwrap();
    assert_eq!(reopened.schema_version().unwrap(), 14);
    assert_eq!(reopened.run(&run).unwrap().state, "running");
    assert!(reopened.flow_deliveries(&run).unwrap().is_empty());
}

#[test]
fn simultaneous_claim_has_one_winner() {
    let (dir, _store, _, node) = setup();
    let gate = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let path = dir.path().join("state.db");
            let node = node.clone();
            let gate = gate.clone();
            std::thread::spawn(move || {
                let mut store = Store::open(&path).unwrap();
                gate.wait();
                store.begin_attempt(&node, 1).is_ok()
            })
        })
        .collect();
    assert_eq!(
        handles
            .into_iter()
            .filter(|h| h.thread().id() != std::thread::current().id())
            .map(|h| usize::from(h.join().unwrap()))
            .sum::<usize>(),
        1
    );
}

#[test]
fn store_enforces_foreign_keys_and_full_durability() {
    let (_dir, store, _, _) = setup();
    let health = store.health().unwrap();
    assert!(health.foreign_keys);
    assert_eq!(health.synchronous, 2);
    assert_eq!(health.journal_mode, "wal");
    assert_eq!(health.busy_timeout_ms, 5000);
}

#[test]
fn schema_eleven_upgrades_to_consultation_provenance() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("schema-11.db");
    let conn = rusqlite::Connection::open(&path).unwrap();
    for migration in [
        include_str!("../migrations/0001_initial.sql"),
        include_str!("../migrations/0002_workspaces.sql"),
        include_str!("../migrations/0003_ownership.sql"),
        include_str!("../migrations/0004_contexts.sql"),
        include_str!("../migrations/0005_checks.sql"),
        include_str!("../migrations/0006_runtime.sql"),
        include_str!("../migrations/0007_control.sql"),
        include_str!("../migrations/0008_agents.sql"),
        include_str!("../migrations/0009_capabilities.sql"),
        include_str!("../migrations/0010_teams.sql"),
        include_str!("../migrations/0011_consultations.sql"),
    ] {
        conn.execute_batch(migration).unwrap();
    }
    drop(conn);
    let store = Store::open(&path).unwrap();
    assert_eq!(store.schema_version().unwrap(), 14);
    assert!(
        store
            .active_consultation_processes("missing")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn foreign_key_failure_inside_transition_rolls_back() {
    let (dir, mut store, run, node) = setup();
    let conn = rusqlite::Connection::open(dir.path().join("state.db")).unwrap();
    conn.execute_batch("CREATE TRIGGER invalid_reference AFTER INSERT ON attempts BEGIN
        INSERT INTO node_runs(id,run_id,node_key,ordinal,state) VALUES('bad','missing','bad',99,'pending');
        END;").unwrap();
    let before = store.events(&run, 0).unwrap();
    assert!(store.begin_attempt(&node, 1).is_err());
    assert!(store.active_attempts().unwrap().is_empty());
    assert_eq!(store.events(&run, 0).unwrap(), before);
    assert_eq!(store.node(&node).unwrap().state, "ready");
}

#[test]
fn future_schema_is_rejected_without_rewriting_version() {
    let (dir, store, _, _) = setup();
    drop(store);
    let conn = rusqlite::Connection::open(dir.path().join("state.db")).unwrap();
    conn.execute("INSERT INTO schema_migrations(version) VALUES(999)", [])
        .unwrap();
    assert!(Store::open(&dir.path().join("state.db")).is_err());
    assert_eq!(
        conn.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        999
    );
}

#[test]
fn attempt_completion_and_event_are_exactly_once() {
    let (dir, mut store, run, node) = setup();
    let attempt = store.begin_attempt(&node, 1).unwrap();
    store.mark_running(&attempt.id, 0).unwrap();
    store
        .finish_attempt(&attempt.id, 1, AttemptOutcome::Failed("cli_failed".into()))
        .unwrap();
    let events = store.events(&run, 0).unwrap();
    assert!(
        store
            .finish_attempt(&attempt.id, 1, AttemptOutcome::Failed("cli_failed".into()))
            .is_err()
    );
    assert_eq!(store.events(&run, 0).unwrap(), events);
    assert_eq!(store.node(&node).unwrap().state, "failed");
    drop(store);
    let store = Store::open(&dir.path().join("state.db")).unwrap();
    assert_eq!(store.events(&run, 0).unwrap(), events);
    for (i, event) in events.iter().enumerate() {
        assert_eq!(event.sequence, i as i64 + 1);
    }
}

#[test]
fn failed_event_insert_rolls_back_state_and_revision() {
    let (dir, mut store, run, node) = setup();
    let db = rusqlite::Connection::open(dir.path().join("state.db")).unwrap();
    db.execute_batch("CREATE TRIGGER fail_event BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    let before = store.node(&node).unwrap();
    let count = store.events(&run, 0).unwrap().len();
    assert!(store.begin_attempt(&node, 1).is_err());
    assert_eq!(store.node(&node).unwrap(), before);
    assert_eq!(store.events(&run, 0).unwrap().len(), count);
    assert!(store.active_attempts().unwrap().is_empty());
}

#[test]
fn two_connections_only_one_can_claim_node() {
    let (dir, mut first, _, node) = setup();
    let mut second = Store::open(&dir.path().join("state.db")).unwrap();
    first.begin_attempt(&node, 1).unwrap();
    assert!(second.begin_attempt(&node, 1).is_err());
    assert_eq!(second.active_attempts().unwrap().len(), 1);
}

#[test]
fn invalid_create_is_atomic_and_active_query_is_read_only() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("state.db")).unwrap();
    assert!(store.create_run("goal", &["same", "same"]).is_err());
    let db = rusqlite::Connection::open(dir.path().join("state.db")).unwrap();
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM runs", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    let run = store.create_run("goal", &["one"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let node = store.nodes(&run).unwrap()[0].id.clone();
    store.ready_node(&node, 0).unwrap();
    let a = store.begin_attempt(&node, 1).unwrap();
    let events = store.events(&run, 0).unwrap();
    assert_eq!(store.active_attempts().unwrap()[0].id, a.id);
    assert_eq!(store.events(&run, 0).unwrap(), events);
}

#[test]
fn cancellation_is_idempotent_and_emits_one_request_and_terminal_event() {
    let (_dir, mut store, run, _) = setup();
    store.request_cancel(&run).unwrap();
    store.request_cancel(&run).unwrap();
    store.apply_cancel(&run).unwrap();
    store.apply_cancel(&run).unwrap();

    assert_eq!(store.run(&run).unwrap().state, "cancelled");
    let kinds = store
        .events(&run, 0)
        .unwrap()
        .into_iter()
        .map(|event| event.event_type)
        .collect::<Vec<_>>();
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| *kind == "cancel.requested")
            .count(),
        1
    );
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| *kind == "workflow.cancelled")
            .count(),
        1
    );
}

#[test]
fn terminal_attempt_is_immutable_and_result_is_persisted() {
    let (_dir, mut store, run, node) = setup();
    let a = store.begin_attempt(&node, 1).unwrap();
    // A process which never started cannot complete successfully.
    assert!(
        store
            .finish_attempt(&a.id, 0, AttemptOutcome::Succeeded(json!({"ok":true})))
            .is_err()
    );
    store.mark_running(&a.id, 0).unwrap();
    store
        .finish_attempt(&a.id, 1, AttemptOutcome::Succeeded(json!({"ok":true})))
        .unwrap();
    assert_eq!(
        store.attempt(&a.id).unwrap().output,
        Some(json!({"ok":true}))
    );
    assert!(store.mark_running(&a.id, 2).is_err());
    assert_eq!(store.events(&run, 3).unwrap()[0].sequence, 4);
}
