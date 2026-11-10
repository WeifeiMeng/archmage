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

#[cfg(unix)]
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
        environment: BTreeMap::from([("FAKE_QODER_SCENARIO".into(), "success".into())]),
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

#[cfg(unix)]
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
