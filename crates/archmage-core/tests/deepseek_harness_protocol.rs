use archmage_core::{
    process::identity_is_alive,
    provider::deepseek_harness::{DeepseekHarnessSidecar, PermissionMode, SidecarConfig},
};
use serde_json::json;
use std::{collections::BTreeMap, path::Path, time::Duration};

fn config(root: &Path) -> SidecarConfig {
    let mut environment = BTreeMap::new();
    environment.insert(
        "PYTHONPATH".into(),
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/deepseek_harness_sdk")
            .into_os_string(),
    );
    environment.insert(
        "FAKE_DSH_LOG".into(),
        root.join("sdk.jsonl").into_os_string(),
    );
    environment.insert("PYTHONDONTWRITEBYTECODE".into(), "1".into());
    SidecarConfig {
        python_binary: "python3".into(),
        data_root: root.join("data"),
        repository: root.to_path_buf(),
        provider_route: "deepseek-official".into(),
        model: "deepseek-v4-flash".into(),
        session_id: "archmage-sidecar-test".into(),
        permission_mode: PermissionMode::WorkspaceWrite,
        session_root: root.join("sessions"),
        diagnostics_dir: root.join("diagnostics"),
        deadline: Duration::from_secs(10),
        max_line_bytes: 4096,
        max_stream_bytes: 64 * 1024,
        max_result_bytes: 4096,
        environment,
    }
}

#[tokio::test]
async fn sidecar_reuses_one_process_and_strictly_parses_results() {
    let temp = tempfile::tempdir().unwrap();
    let mut sidecar = DeepseekHarnessSidecar::start(config(temp.path()))
        .await
        .unwrap();
    let identity = sidecar.identity().clone();
    let first = sidecar
        .invoke("first", b"first task", &json!({"type":"object"}))
        .await
        .unwrap();
    let second = sidecar
        .invoke("second", b"second task", &json!({"type":"object"}))
        .await
        .unwrap();
    assert_eq!(first.conversation_id, "archmage-sidecar-test");
    assert_eq!(second.conversation_id, "archmage-sidecar-test");
    assert_eq!(first.result, json!({"ok": true}));
    assert!(identity_is_alive(&identity));
    sidecar.close().await.unwrap();
    assert!(!identity_is_alive(&identity));
}

#[tokio::test]
async fn sidecar_poisoning_is_sticky_after_bridge_failure() {
    let temp = tempfile::tempdir().unwrap();
    let mut config = config(temp.path());
    config
        .environment
        .insert("FAKE_DSH_MODE".into(), "error".into());
    let mut sidecar = DeepseekHarnessSidecar::start(config).await.unwrap();
    let first = sidecar
        .invoke("first", b"fail", &json!({"type":"object"}))
        .await
        .unwrap_err()
        .to_string();
    let second = sidecar
        .invoke("second", b"never sent", &json!({"type":"object"}))
        .await
        .unwrap_err()
        .to_string();
    assert_eq!(second, first);
    sidecar.cancel().await.unwrap();
}
