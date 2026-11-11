use archmage_core::provider::{
    acp::AcpAccess,
    qoder::{QoderSession, QoderSessionConfig, inspect_at},
};
use std::{collections::BTreeMap, time::Duration};

#[tokio::test]
#[ignore = "requires installed and authenticated Qoder CLI"]
async fn real_qoder_acp_completes_a_strict_json_turn() {
    let tools = inspect_at(None).await.unwrap();
    let repository = tempfile::tempdir().unwrap();
    let diagnostics = tempfile::tempdir().unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(repository.path())
        .status()
        .unwrap();
    assert!(status.success());
    let mut session = QoderSession::start(QoderSessionConfig {
        binary: tools.binary,
        repository: repository.path().to_path_buf(),
        model: None,
        agent_name: None,
        provider_config: serde_json::json!({"mcpServers": []}),
        access: AcpAccess::ReadOnly,
        diagnostics_dir: diagnostics.path().to_path_buf(),
        deadline: Duration::from_secs(120),
        max_line_bytes: 1024 * 1024,
        max_stream_bytes: 4 * 1024 * 1024,
        max_result_bytes: 1024 * 1024,
        environment: BTreeMap::new(),
    })
    .await
    .unwrap();
    let outcome = session
        .prompt(
            "live-smoke",
            b"Return exactly the JSON object {\"ok\":true}. Do not edit files or run commands.",
        )
        .await
        .unwrap();
    assert_eq!(outcome.result, serde_json::json!({"ok": true}));
    session.close().await.unwrap();
}
