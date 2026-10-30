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
fn cancelled_delivery_keeps_its_attempt_and_terminal_state() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let run = store.create_run("goal", &["coder", "reviewer"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    store.enqueue_flow_entry(&run, generation, "coder").unwrap();
    let delivery = store
        .claim_flow_delivery(&run, generation)
        .unwrap()
        .unwrap();
    let node = store.nodes(&run).unwrap().remove(0);
    store.ready_node(&node.id, node.revision).unwrap();
    let attempt = store.begin_attempt(&node.id, node.revision + 1).unwrap();
    store
        .bind_flow_attempt(&delivery.id, generation, &attempt.id)
        .unwrap();
    store.request_cancel(&run).unwrap();
    store.apply_cancel(&run).unwrap();
    let settled = store.flow_deliveries(&run).unwrap();
    assert_eq!(settled[0].state, "cancelled");
    assert_eq!(settled[0].attempt_id.as_deref(), Some(attempt.id.as_str()));
}

#[test]
fn interrupted_delivery_keeps_its_attempt_and_terminal_state() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let run = store.create_run("goal", &["coder", "reviewer"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    store.enqueue_flow_entry(&run, generation, "coder").unwrap();
    let delivery = store
        .claim_flow_delivery(&run, generation)
        .unwrap()
        .unwrap();
    let node = store.nodes(&run).unwrap().remove(0);
    store.ready_node(&node.id, node.revision).unwrap();
    let attempt = store.begin_attempt(&node.id, node.revision + 1).unwrap();
    store
        .bind_flow_attempt(&delivery.id, generation, &attempt.id)
        .unwrap();
    store.interrupt_orphaned_run(&run).unwrap();
    let settled = store.flow_deliveries(&run).unwrap();
    assert_eq!(settled[0].state, "interrupted");
    assert_eq!(settled[0].attempt_id.as_deref(), Some(attempt.id.as_str()));
}

#[test]
fn failed_run_settles_active_and_queued_deliveries() {
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
        &["coder".into(), "reviewer".into()],
        false,
    );
    let active = store
        .claim_flow_delivery(&run, generation)
        .unwrap()
        .unwrap();
    assert_eq!(active.target, "coder");
    store
        .fail_flow_run(&run, generation, "contract_invalid")
        .unwrap();
    let deliveries = store.flow_deliveries(&run).unwrap();
    assert_eq!(
        deliveries
            .iter()
            .map(|item| item.state.as_str())
            .collect::<Vec<_>>(),
        ["completed", "failed", "skipped_due_failure"]
    );
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
    assert!(
        store
            .finish_flow_delivery(
                &delivery.id,
                generation,
                &attempt.id,
                &artifact,
                None,
                &serde_json::json!({"summary":"design"}),
                "thread-1",
                &["coder".into()],
                false,
            )
            .is_err()
    );
    store
        .bind_flow_attempt(&delivery.id, generation, &attempt.id)
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

#[test]
fn completed_delivery_projects_the_output_of_its_own_attempt() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let run = store.create_run("goal", &["coder", "reviewer"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    store.enqueue_flow_entry(&run, generation, "coder").unwrap();
    let outputs = [
        serde_json::json!({"status":"implemented","changed_paths":["src/a.rs"]}),
        serde_json::json!({"status":"changes_requested","issues":["缺少测试"]}),
        serde_json::json!({"status":"fixed","notes":["补充测试"]}),
        serde_json::json!({"status":"approved","summary_markdown":"实现已验证通过"}),
    ];
    for (index, (target, next, complete)) in [
        ("coder", vec!["reviewer".into()], false),
        ("reviewer", vec!["coder".into()], false),
        ("coder", vec!["reviewer".into()], false),
        ("reviewer", vec![], true),
    ]
    .into_iter()
    .enumerate()
    {
        let delivery = store
            .claim_flow_delivery(&run, generation)
            .unwrap()
            .unwrap();
        assert_eq!(delivery.target, target);
        store.prepare_flow_node(&run, target, generation).unwrap();
        finish_test_delivery_with_output(
            &mut store,
            &temp,
            &run,
            generation,
            &delivery,
            &next,
            complete,
            &outputs[index],
        );
    }
    let deliveries = store.flow_deliveries(&run).unwrap();
    assert_eq!(deliveries.len(), 4);
    for (delivery, output) in deliveries.iter().zip(outputs.iter()) {
        assert_eq!(delivery.output.as_ref(), Some(output));
    }
    // Each round keeps its own output even though the capability ran twice.
    assert_ne!(deliveries[0].output, deliveries[2].output);
    assert_eq!(
        deliveries[0].output.as_ref().unwrap()["changed_paths"][0],
        "src/a.rs"
    );
    assert_eq!(
        deliveries[1].output.as_ref().unwrap()["issues"][0],
        "缺少测试"
    );
}

#[test]
fn pending_and_running_deliveries_project_no_output() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let run = store.create_run("goal", &["coder", "reviewer"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    store.enqueue_flow_entry(&run, generation, "coder").unwrap();
    let queued = store.flow_deliveries(&run).unwrap();
    assert_eq!(queued[0].state, "queued");
    assert_eq!(queued[0].output, None);
    let delivery = store
        .claim_flow_delivery(&run, generation)
        .unwrap()
        .unwrap();
    let node = store.nodes(&run).unwrap().remove(0);
    store.ready_node(&node.id, node.revision).unwrap();
    let attempt = store.begin_attempt(&node.id, node.revision + 1).unwrap();
    store
        .bind_flow_attempt(&delivery.id, generation, &attempt.id)
        .unwrap();
    let running = store.flow_deliveries(&run).unwrap();
    assert_eq!(running[0].state, "running");
    assert_eq!(running[0].attempt_id.as_deref(), Some(attempt.id.as_str()));
    assert_eq!(running[0].output, None);
}

#[test]
fn existing_rows_read_back_historical_output_without_backfill() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let run = store.create_run("goal", &["coder", "reviewer"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    store.enqueue_flow_entry(&run, generation, "coder").unwrap();
    let delivery = store
        .claim_flow_delivery(&run, generation)
        .unwrap()
        .unwrap();
    finish_test_delivery_with_output(
        &mut store,
        &temp,
        &run,
        generation,
        &delivery,
        &["reviewer".into()],
        false,
        &serde_json::json!({"summary_markdown":"第一轮交付","notes":["读取历史"]}),
    );
    drop(store);
    let reopened = Store::open(&temp.path().join("archmage.db")).unwrap();
    let deliveries = reopened.flow_deliveries(&run).unwrap();
    assert_eq!(
        deliveries[0].output.as_ref().unwrap()["summary_markdown"],
        "第一轮交付"
    );
    // The queued follow-up has no attempt yet, so it projects no output.
    assert_eq!(deliveries[1].state, "queued");
    assert_eq!(deliveries[1].output, None);
}

#[test]
fn feedback_cycle_reexecutes_the_same_capability_for_each_delivery() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let run = store.create_run("goal", &["coder", "reviewer"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    store.enqueue_flow_entry(&run, generation, "coder").unwrap();
    for (target, next, complete) in [
        ("coder", vec!["reviewer".into()], false),
        ("reviewer", vec!["coder".into()], false),
        ("coder", vec!["reviewer".into()], false),
        ("reviewer", vec![], true),
    ] {
        let delivery = store
            .claim_flow_delivery(&run, generation)
            .unwrap()
            .unwrap();
        assert_eq!(delivery.target, target);
        store.prepare_flow_node(&run, target, generation).unwrap();
        finish_test_delivery(
            &mut store, &temp, &run, generation, &delivery, &next, complete,
        );
    }
    let deliveries = store.flow_deliveries(&run).unwrap();
    assert_eq!(deliveries.len(), 4);
    assert_eq!(
        deliveries
            .iter()
            .filter(|item| item.state == "completed")
            .count(),
        4
    );
    assert_ne!(deliveries[0].attempt_id, deliveries[2].attempt_id);
    assert_eq!(
        deliveries[3].routing.as_ref().unwrap()["decision"],
        "complete"
    );
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
    finish_test_delivery_with_output(
        store,
        temp,
        run,
        generation,
        delivery,
        next,
        complete,
        &serde_json::json!({"summary":"done"}),
    );
}

#[allow(clippy::too_many_arguments)]
fn finish_test_delivery_with_output(
    store: &mut Store,
    temp: &tempfile::TempDir,
    run: &str,
    generation: i64,
    delivery: &archmage_core::store::FlowDelivery,
    next: &[String],
    complete: bool,
    output: &serde_json::Value,
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
        .bind_flow_attempt(&delivery.id, generation, &attempt.id)
        .unwrap();
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
            output.to_string().as_bytes(),
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
            output,
            "thread-test",
            next,
            complete,
        )
        .unwrap();
}
