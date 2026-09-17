use archmage_core::{
    artifact::ArtifactStore,
    store::{AttemptOutcome, Store},
};
use serde_json::json;

fn setup() -> (tempfile::TempDir, Store, ArtifactStore, String) {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Store::open(&dir.path().join("db")).unwrap();
    let run = db.create_run("goal", &["design"]).unwrap();
    db.start_run(&run, 0).unwrap();
    let node = db.nodes(&run).unwrap()[0].id.clone();
    db.ready_node(&node, 0).unwrap();
    let a = db.begin_attempt(&node, 1).unwrap();
    db.mark_running(&a.id, 0).unwrap();
    let files = ArtifactStore::open(&dir.path().join("artifacts")).unwrap();
    (dir, db, files, a.id)
}

#[test]
fn artifact_is_not_context_eligible_before_success() {
    let (_dir, mut db, files, attempt) = setup();
    let id = files
        .publish(&mut db, &attempt, b"design", "text/markdown")
        .unwrap();
    assert!(files.read_committed(&db, &id).is_err());
    db.finish_attempt(&attempt, 1, AttemptOutcome::Succeeded(json!({"ok":true})))
        .unwrap();
    assert_eq!(files.read_committed(&db, &id).unwrap(), b"design");
    assert!(
        files
            .publish(&mut db, &attempt, b"late", "text/plain")
            .is_err()
    );
}

#[test]
fn missing_and_tampered_content_are_rejected() {
    let (dir, mut db, files, attempt) = setup();
    let id = files
        .publish(&mut db, &attempt, b"original", "text/plain")
        .unwrap();
    db.finish_attempt(&attempt, 1, AttemptOutcome::Succeeded(json!({})))
        .unwrap();
    let path = dir.path().join("artifacts").join(&id);
    std::fs::write(&path, b"tampered").unwrap();
    assert!(files.read_committed(&db, &id).is_err());
    std::fs::remove_file(&path).unwrap();
    assert!(files.read_committed(&db, &id).is_err());
}

#[test]
fn invalid_ids_and_symlink_roots_are_rejected() {
    let (dir, db, files, _) = setup();
    for id in ["../db", "/etc/passwd", ""] {
        assert!(files.read_committed(&db, id).is_err());
    }
    #[cfg(unix)]
    {
        let alias = dir.path().join("alias");
        std::os::unix::fs::symlink(dir.path().join("artifacts"), &alias).unwrap();
        assert!(ArtifactStore::open(&alias).is_err());
    }
}

#[test]
fn metadata_failure_leaves_only_unregistered_orphan() {
    let (dir, mut db, files, attempt) = setup();
    let conn = rusqlite::Connection::open(dir.path().join("db")).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_artifact BEFORE INSERT ON artifacts BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    assert!(
        files
            .publish(&mut db, &attempt, b"orphan", "text/plain")
            .is_err()
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM artifacts", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    let paths: Vec<_> = std::fs::read_dir(dir.path().join("artifacts"))
        .unwrap()
        .collect();
    assert_eq!(paths.len(), 1);
    let id = paths[0]
        .as_ref()
        .unwrap()
        .file_name()
        .to_string_lossy()
        .into_owned();
    assert!(files.read_committed(&db, &id).is_err());
}
