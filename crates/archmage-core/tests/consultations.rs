use archmage_core::process::ProcessIdentity;
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
    let attempt = store
        .begin_attempt(&designer.id, designer.revision + 1)
        .unwrap();
    store.mark_running(&attempt.id, attempt.revision).unwrap();
    store
        .finish_attempt(
            &attempt.id,
            attempt.revision + 1,
            AttemptOutcome::Succeeded(serde_json::json!({"design":"v1"})),
        )
        .unwrap();

    store.record_consultation(&run, "designer", "coder", &attempt.id, 0, generation,
        "changes_requested", "Needs more detail", &["Specify error cases".into()], "thread-1",
        &serde_json::json!({"decision":"changes_requested","summary":"Needs more detail","suggestions":["Specify error cases"]})).unwrap();
    let reviews = store.consultations_for_attempt(&attempt.id).unwrap();
    assert_eq!(reviews.len(), 1);
    assert_eq!(reviews[0].suggestions, ["Specify error cases"]);
    assert_eq!(reviews[0].result["decision"], "changes_requested");
    assert_eq!(reviews[0].result_sha256.len(), 64);
    store
        .reopen_node_for_feedback(&designer.id, generation, &["Specify error cases".into()])
        .unwrap();
    assert_eq!(store.node(&designer.id).unwrap().state, "pending");
    assert_eq!(
        store
            .previous_output_for_revision(&designer.id)
            .unwrap()
            .unwrap()["design"],
        "v1"
    );
    assert_eq!(
        store.consultations_for_attempt(&attempt.id).unwrap().len(),
        1
    );
    assert!(
        store
            .record_consultation(
                &run,
                "designer",
                "tester",
                &attempt.id,
                0,
                generation,
                "approved",
                "Looks fine",
                &[],
                "thread-2",
                &serde_json::json!({"decision":"approved","summary":"Looks fine","suggestions":[]})
            )
            .is_err()
    );
    let reopened = store.node(&designer.id).unwrap();
    store.ready_node(&reopened.id, reopened.revision).unwrap();
    let revised = store
        .begin_attempt(&reopened.id, reopened.revision + 1)
        .unwrap();
    store.mark_running(&revised.id, revised.revision).unwrap();
    store
        .finish_attempt(
            &revised.id,
            revised.revision + 1,
            AttemptOutcome::Succeeded(serde_json::json!({"design":"v2"})),
        )
        .unwrap();
    assert!(
        store
            .record_consultation(
                &run,
                "designer",
                "tester",
                &attempt.id,
                0,
                generation,
                "approved",
                "Stale",
                &[],
                "thread-stale",
                &serde_json::json!({"decision":"approved","summary":"Stale","suggestions":[]})
            )
            .is_err()
    );
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
    let attempt = store
        .begin_attempt(&designer.id, designer.revision + 1)
        .unwrap();
    store.mark_running(&attempt.id, attempt.revision).unwrap();
    store
        .finish_attempt(
            &attempt.id,
            attempt.revision + 1,
            AttemptOutcome::Succeeded(serde_json::json!({"design":"v1"})),
        )
        .unwrap();
    assert!(
        store
            .record_consultation(
                &run,
                "designer",
                "coder",
                &attempt.id,
                0,
                generation,
                "changes_requested",
                "No",
                &[],
                "thread-1",
                &serde_json::json!({"decision":"changes_requested","summary":"No","suggestions":[]})
            )
            .is_err()
    );
}

#[test]
fn final_reviewer_feedback_reopens_coder_tester_and_reviewer() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let run = store
        .create_run("goal", &["designer", "coder", "tester", "reviewer"])
        .unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    let mut reviewer_attempt = String::new();
    for node in store.nodes(&run).unwrap() {
        store.ready_node(&node.id, node.revision).unwrap();
        let attempt = store.begin_attempt(&node.id, node.revision + 1).unwrap();
        store.mark_running(&attempt.id, attempt.revision).unwrap();
        store
            .finish_attempt(
                &attempt.id,
                attempt.revision + 1,
                AttemptOutcome::Succeeded(serde_json::json!({"producer":node.key})),
            )
            .unwrap();
        if node.key == "reviewer" {
            reviewer_attempt = attempt.id;
        }
    }
    store
        .reopen_after_reviewer(
            &run,
            generation,
            &reviewer_attempt,
            &["Fix broken test".into()],
        )
        .unwrap();
    let states: Vec<_> = store
        .nodes(&run)
        .unwrap()
        .into_iter()
        .map(|node| node.state)
        .collect();
    assert_eq!(states, ["succeeded", "pending", "pending", "pending"]);
    let coder = store.nodes(&run).unwrap()[1].clone();
    assert_eq!(
        store.feedback_for_node(&coder.id).unwrap(),
        ["Fix broken test"]
    );
    assert_eq!(
        store
            .previous_output_for_revision(&coder.id)
            .unwrap()
            .unwrap()["producer"],
        "coder"
    );
}

#[test]
fn consultation_process_is_registered_and_cancelled_with_run() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let run = store.create_run("goal", &["designer", "coder"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    let node = store.nodes(&run).unwrap()[0].clone();
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
    let identity = ProcessIdentity {
        pid: 1234,
        pgid: 1234,
        started: "test".into(),
    };
    store
        .register_consultation_process(
            "process-1",
            &run,
            &attempt.id,
            "coder",
            generation,
            &identity,
        )
        .unwrap();
    assert_eq!(
        store.active_consultation_processes(&run).unwrap(),
        [identity]
    );
    store.request_cancel(&run).unwrap();
    assert!(
        store
            .register_consultation_process(
                "process-2",
                &run,
                &attempt.id,
                "tester",
                generation,
                &ProcessIdentity {
                    pid: 1235,
                    pgid: 1235,
                    started: "test".into()
                }
            )
            .is_err()
    );
    store.apply_cancel(&run).unwrap();
    assert!(
        store
            .active_consultation_processes(&run)
            .unwrap()
            .is_empty()
    );
}
