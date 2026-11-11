use archmage_core::{
    process::{ProcessIdentity, identity_is_alive},
    provider::{
        acp::AcpAccess,
        deepseek_harness::{PermissionMode, SidecarConfig},
        pool::{ProviderPool, SidecarKey},
        qoder::QoderSessionConfig,
    },
};
use serde_json::json;
use std::{collections::BTreeMap, path::Path, sync::Mutex, time::Duration};

#[cfg(target_os = "macos")]
fn qoder_config(root: &Path, name: &str) -> QoderSessionConfig {
    use std::os::unix::fs::PermissionsExt;
    let binary = root.join(format!("qoder-{name}"));
    std::fs::copy("tests/fixtures/fake_qoder_acp.py", &binary).unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    QoderSessionConfig {
        binary,
        repository: root.to_path_buf(),
        model: None,
        agent_name: None,
        provider_config: json!({"mcpServers": []}),
        access: AcpAccess::WorkspaceWrite,
        diagnostics_dir: root.join("qoder-diagnostics").join(name),
        deadline: Duration::from_secs(5),
        max_line_bytes: 4096,
        max_stream_bytes: 64 * 1024,
        max_result_bytes: 4096,
        environment: BTreeMap::from([
            ("FAKE_QODER_SCENARIO".into(), "success".into()),
            ("PATH".into(), "/usr/bin:/bin:/usr/sbin:/sbin".into()),
        ]),
    }
}

fn config(root: &Path, name: &str) -> SidecarConfig {
    let mut environment = BTreeMap::new();
    environment.insert(
        "PYTHONPATH".into(),
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/deepseek_harness_sdk")
            .into_os_string(),
    );
    environment.insert(
        "FAKE_DSH_LOG".into(),
        root.join(format!("{name}.jsonl")).into_os_string(),
    );
    environment.insert("PYTHONDONTWRITEBYTECODE".into(), "1".into());
    SidecarConfig {
        python_binary: "python3".into(),
        data_root: root.join("data"),
        repository: root.to_path_buf(),
        provider_route: "deepseek-official".into(),
        model: "deepseek-v4-flash".into(),
        session_id: String::new(),
        permission_mode: PermissionMode::WorkspaceWrite,
        session_root: root.join("sessions").join(name),
        diagnostics_dir: root.join("diagnostics").join(name),
        deadline: Duration::from_secs(10),
        max_line_bytes: 4096,
        max_stream_bytes: 64 * 1024,
        max_result_bytes: 4096,
        environment,
    }
}

#[tokio::test]
async fn pool_reuses_by_run_capability_and_isolates_other_keys() {
    let temp = tempfile::tempdir().unwrap();
    let pool = ProviderPool::new();
    let coder = SidecarKey::new("run-a", "coder");
    let reviewer = SidecarKey::new("run-a", "reviewer");
    let identities = Mutex::new(Vec::<ProcessIdentity>::new());
    for invocation in ["one", "two"] {
        pool.invoke_main(
            coder.clone(),
            config(temp.path(), "coder"),
            invocation,
            invocation.as_bytes(),
            &json!({"type":"object"}),
            |identity| {
                identities.lock().unwrap().push(identity.clone());
                Ok(())
            },
        )
        .await
        .unwrap();
    }
    pool.invoke_main(
        reviewer,
        config(temp.path(), "reviewer"),
        "review",
        b"review",
        &json!({"type":"object"}),
        |identity| {
            identities.lock().unwrap().push(identity.clone());
            Ok(())
        },
    )
    .await
    .unwrap();
    let identities = identities.into_inner().unwrap();
    assert_eq!(identities[0].pid, identities[1].pid);
    assert_ne!(identities[0].pid, identities[2].pid);
    assert!(identities.iter().all(identity_is_alive));
    pool.close_run("run-a").await.unwrap();
    assert!(
        identities
            .iter()
            .all(|identity| !identity_is_alive(identity))
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn pool_reuses_qoder_by_run_capability_and_closes_temporary_sessions() {
    let temp = tempfile::tempdir().unwrap();
    let pool = ProviderPool::new();
    let identities = Mutex::new(Vec::<ProcessIdentity>::new());
    for invocation in ["one", "two"] {
        pool.invoke_main(
            SidecarKey::new("run-q", "coder"),
            qoder_config(temp.path(), "coder"),
            invocation,
            b"return JSON",
            &json!({"type":"object"}),
            |identity| {
                identities.lock().unwrap().push(identity.clone());
                Ok(())
            },
        )
        .await
        .unwrap();
    }
    pool.invoke_main(
        SidecarKey::new("run-q", "reviewer"),
        qoder_config(temp.path(), "reviewer"),
        "review",
        b"return JSON",
        &json!({"type":"object"}),
        |identity| {
            identities.lock().unwrap().push(identity.clone());
            Ok(())
        },
    )
    .await
    .unwrap();
    let temporary = Mutex::new(None);
    pool.invoke_temporary(
        qoder_config(temp.path(), "consult"),
        "consult",
        b"return JSON",
        &json!({"type":"object"}),
        |identity| {
            *temporary.lock().unwrap() = Some(identity.clone());
            Ok(())
        },
    )
    .await
    .unwrap();
    let identities = identities.into_inner().unwrap();
    assert_eq!(identities[0].pid, identities[1].pid);
    assert_ne!(identities[0].pid, identities[2].pid);
    assert!(!identity_is_alive(
        &temporary.into_inner().unwrap().unwrap()
    ));
    pool.close_run("run-q").await.unwrap();
    assert!(
        identities
            .iter()
            .all(|identity| !identity_is_alive(identity))
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn qoder_pool_cancellation_interrupts_a_running_prompt_without_waiting_for_the_mutex() {
    let temp = tempfile::tempdir().unwrap();
    let pool = std::sync::Arc::new(ProviderPool::new());
    let mut config = qoder_config(temp.path(), "cancel");
    config
        .environment
        .insert("FAKE_QODER_SCENARIO".into(), "prompt-hang".into());
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let started_tx = Mutex::new(Some(started_tx));
    let invoke = tokio::spawn({
        let pool = pool.clone();
        async move {
            pool.invoke_main(
                SidecarKey::new("run-cancel", "coder"),
                config,
                "attempt-1",
                b"hang",
                &json!({"type":"object"}),
                |_| {
                    started_tx.lock().unwrap().take().unwrap().send(()).unwrap();
                    Ok(())
                },
            )
            .await
        }
    });
    tokio::time::timeout(Duration::from_secs(5), started_rx)
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(2),
        pool.cancel_capability("run-cancel", "coder"),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), invoke)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    let retry = pool
        .invoke_main(
            SidecarKey::new("run-cancel", "coder"),
            qoder_config(temp.path(), "unexpected-retry"),
            "attempt-2",
            b"must not reuse the killed session",
            &json!({"type":"object"}),
            |_| panic!("a cancelled pooled session must stay poisoned"),
        )
        .await;
    assert!(retry.is_err());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn qoder_pool_cancellation_interrupts_session_initialization() {
    let temp = tempfile::tempdir().unwrap();
    let child_pid_path = temp.path().join("initialize-child.pid");
    let pool = std::sync::Arc::new(ProviderPool::new());
    let mut config = qoder_config(temp.path(), "initializing");
    config
        .environment
        .insert("FAKE_QODER_SCENARIO".into(), "initialize-hang".into());
    config.environment.insert(
        "FAKE_CHILD_PID_PATH".into(),
        child_pid_path.clone().into_os_string(),
    );
    let invoke = tokio::spawn({
        let pool = pool.clone();
        async move {
            pool.invoke_main(
                SidecarKey::new("run-initializing", "coder"),
                config,
                "attempt-1",
                b"must not start",
                &json!({"type":"object"}),
                |_| panic!("initialization never reached a live session"),
            )
            .await
        }
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        while !child_pid_path.is_file() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("fake Qoder did not create its child PID file");
    let child_pid: i32 = std::fs::read_to_string(&child_pid_path)
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(unsafe { libc::kill(child_pid, 0) }, 0);
    tokio::time::timeout(
        Duration::from_secs(2),
        pool.cancel_capability("run-initializing", "coder"),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), invoke)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    tokio::time::timeout(Duration::from_secs(2), async {
        while unsafe { libc::kill(child_pid, 0) } == 0 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("Qoder initialization child survived cancellation");
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn qoder_pool_cancellation_terminates_and_poisons_an_idle_session() {
    let temp = tempfile::tempdir().unwrap();
    let pool = ProviderPool::new();
    let identity = Mutex::new(None);
    pool.invoke_main(
        SidecarKey::new("run-idle", "coder"),
        qoder_config(temp.path(), "idle"),
        "attempt-1",
        b"return JSON",
        &json!({"type":"object"}),
        |spawned| {
            *identity.lock().unwrap() = Some(spawned.clone());
            Ok(())
        },
    )
    .await
    .unwrap();
    let identity = identity.into_inner().unwrap().unwrap();
    assert!(identity_is_alive(&identity));
    pool.cancel_capability("run-idle", "coder").await.unwrap();
    assert!(!identity_is_alive(&identity));
    assert!(
        pool.invoke_main(
            SidecarKey::new("run-idle", "coder"),
            qoder_config(temp.path(), "must-not-restart"),
            "attempt-2",
            b"return JSON",
            &json!({"type":"object"}),
            |_| panic!("idle cancellation must poison the slot"),
        )
        .await
        .is_err()
    );
}
