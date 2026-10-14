use archmage_core::{artifact::ArtifactStore, process::ProcessIdentity, store::Store};

#[test]
fn flow_entry_is_durable_and_can_be_claimed_once() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let run = store.create_run("goal", &["coder", "reviewer"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    store.enqueue_flow_entry(&run, generation, "coder").unwrap();
    let claimed = store
        .claim_flow_delivery(&run, generation)
        .unwrap()
        .unwrap();
    assert_eq!(claimed.target, "coder");
    assert_eq!(claimed.sequence, 1);
    assert_eq!(claimed.state, "running");
    assert!(
        store
            .claim_flow_delivery(&run, generation)
            .unwrap()
            .is_none()
    );
    drop(store);
    let reopened = Store::open(&temp.path().join("archmage.db")).unwrap();
    assert_eq!(reopened.flow_deliveries(&run).unwrap()[0].id, claimed.id);
}

#[test]
fn second_claim_waits_for_running_delivery_and_stale_owner_cannot_claim() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let run = store.create_run("goal", &["coder", "reviewer"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    store.enqueue_flow_entry(&run, generation, "coder").unwrap();
    let first = store
        .claim_flow_delivery(&run, generation)
        .unwrap()
        .unwrap();
    assert!(
        store
            .claim_flow_delivery(&run, generation)
            .unwrap()
            .is_none()
    );
    assert!(store.claim_flow_delivery(&run, generation - 1).is_err());
    assert_eq!(store.flow_deliveries(&run).unwrap()[0].id, first.id);
}

#[test]
fn finishing_one_delivery_atomically_queues_each_selected_recipient() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let run = store
        .create_run("goal", &["designer", "coder", "tester"])
        .unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    store
        .enqueue_flow_entry(&run, generation, "designer")
        .unwrap();
    let delivery = store
        .claim_flow_delivery(&run, generation)
        .unwrap()
        .unwrap();
    let node = store.nodes(&run).unwrap()[0].clone();
    store.ready_node(&node.id, node.revision).unwrap();
    let attempt = store.begin_attempt(&node.id, node.revision + 1).unwrap();
    store
        .attach_attempt_process(
            &attempt.id,
            attempt.revision,
            generation,
            &ProcessIdentity {
                pid: 42,
                pgid: 42,
                started: "test".into(),
            },
        )
        .unwrap();
    let artifact = ArtifactStore::open(&temp.path().join("artifacts"))
        .unwrap()
        .publish(
            &mut store,
            &attempt.id,
            br#"{"summary":"design"}"#,
            "application/json",
        )
        .unwrap();
    store
        .finish_flow_delivery(
            &delivery.id,
            generation,
            &attempt.id,
            &artifact,
            None,
            &serde_json::json!({"summary":"design"}),
            "thread-1",
            &["coder".into(), "tester".into()],
            false,
        )
        .unwrap();
    let deliveries = store.flow_deliveries(&run).unwrap();
    assert_eq!(
        deliveries
            .iter()
            .map(|item| item.target.as_str())
            .collect::<Vec<_>>(),
        ["designer", "coder", "tester"]
    );
    assert_eq!(deliveries[0].state, "completed");
    assert_eq!(
        deliveries[1].payload_artifact_id.as_deref(),
        Some(artifact.as_str())
    );
    assert_eq!(
        deliveries[2].source_attempt_id.as_deref(),
        Some(attempt.id.as_str())
    );
    let first_recipient = store
        .claim_flow_delivery(&run, generation)
        .unwrap()
        .unwrap();
    assert_eq!(first_recipient.target, "coder");
    assert!(
        store
            .claim_flow_delivery(&run, generation)
            .unwrap()
            .is_none()
    );
}

#[test]
fn terminal_delivery_skips_pending_siblings() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let run = store
        .create_run("goal", &["designer", "coder", "reviewer"])
        .unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    store
        .enqueue_flow_entry(&run, generation, "designer")
        .unwrap();
    let first = store
        .claim_flow_delivery(&run, generation)
        .unwrap()
        .unwrap();
    finish_test_delivery(
        &mut store,
        &temp,
        &run,
        generation,
        &first,
        &["reviewer".into(), "coder".into()],
        false,
    );
    let reviewer = store
        .claim_flow_delivery(&run, generation)
        .unwrap()
        .unwrap();
    finish_test_delivery(&mut store, &temp, &run, generation, &reviewer, &[], true);
    let deliveries = store.flow_deliveries(&run).unwrap();
    assert_eq!(
        deliveries
            .iter()
            .map(|d| d.state.as_str())
            .collect::<Vec<_>>(),
        ["completed", "completed", "skipped_due_terminal"]
    );
    assert!(store.claim_flow_delivery(&run, generation).is_err());
}

fn finish_test_delivery(
    store: &mut Store,
    temp: &tempfile::TempDir,
    run: &str,
    generation: i64,
    delivery: &archmage_core::store::FlowDelivery,
    next: &[String],
    complete: bool,
) {
    let node = store
        .nodes(run)
        .unwrap()
        .into_iter()
        .find(|node| node.key == delivery.target)
        .unwrap();
    store.ready_node(&node.id, node.revision).unwrap();
    let attempt = store.begin_attempt(&node.id, node.revision + 1).unwrap();
    store
        .attach_attempt_process(
            &attempt.id,
            attempt.revision,
            generation,
            &ProcessIdentity {
                pid: 42,
                pgid: 42,
                started: "test".into(),
            },
        )
        .unwrap();
    let artifact = ArtifactStore::open(&temp.path().join("artifacts"))
        .unwrap()
        .publish(
            store,
            &attempt.id,
            br#"{"summary":"done"}"#,
            "application/json",
        )
        .unwrap();
    store
        .finish_flow_delivery(
            &delivery.id,
            generation,
            &attempt.id,
            &artifact,
            None,
            &serde_json::json!({"summary":"done"}),
            "thread-test",
            next,
            complete,
        )
        .unwrap();
}
