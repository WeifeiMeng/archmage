use archmage_core::store::{AttemptOutcome, Store};

#[test]
fn consultation_is_bound_to_the_source_attempt_and_reopen_preserves_history() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let run = store.create_run("goal", &["designer", "coder"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    let designer = store.nodes(&run).unwrap()[0].clone();
    store.ready_node(&designer.id, designer.revision).unwrap();
    let attempt = store.begin_attempt(&designer.id, designer.revision + 1).unwrap();
    store.mark_running(&attempt.id, attempt.revision).unwrap();
    store.finish_attempt(&attempt.id, attempt.revision + 1, AttemptOutcome::Succeeded(serde_json::json!({"design":"v1"}))).unwrap();

    store.record_consultation(&run, "designer", "coder", &attempt.id, 0, generation,
        "changes_requested", "Needs more detail", &["Specify error cases".into()], "thread-1").unwrap();
    let reviews = store.consultations_for_attempt(&attempt.id).unwrap();
    assert_eq!(reviews.len(), 1);
    assert_eq!(reviews[0].suggestions, ["Specify error cases"]);
    store.reopen_node_for_feedback(&designer.id, generation, &["Specify error cases".into()]).unwrap();
    assert_eq!(store.node(&designer.id).unwrap().state, "pending");
    assert_eq!(store.consultations_for_attempt(&attempt.id).unwrap().len(), 1);
    assert!(store.record_consultation(&run, "designer", "tester", &attempt.id, 0, generation,
        "approved", "Looks fine", &[], "thread-2").is_err());
    let reopened = store.node(&designer.id).unwrap();
    store.ready_node(&reopened.id, reopened.revision).unwrap();
    let revised = store.begin_attempt(&reopened.id, reopened.revision + 1).unwrap();
    store.mark_running(&revised.id, revised.revision).unwrap();
    store.finish_attempt(&revised.id, revised.revision + 1, AttemptOutcome::Succeeded(serde_json::json!({"design":"v2"}))).unwrap();
    assert!(store.record_consultation(&run, "designer", "tester", &attempt.id, 0, generation,
        "approved", "Stale", &[], "thread-stale").is_err());
}

#[test]
fn rejected_consultation_requires_actionable_suggestions() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let run = store.create_run("goal", &["designer"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    let designer = store.nodes(&run).unwrap()[0].clone();
    store.ready_node(&designer.id, designer.revision).unwrap();
    let attempt = store.begin_attempt(&designer.id, designer.revision + 1).unwrap();
    store.mark_running(&attempt.id, attempt.revision).unwrap();
    store.finish_attempt(&attempt.id, attempt.revision + 1, AttemptOutcome::Succeeded(serde_json::json!({"design":"v1"}))).unwrap();
    assert!(store.record_consultation(&run, "designer", "coder", &attempt.id, 0, generation,
        "changes_requested", "No", &[], "thread-1").is_err());
}
