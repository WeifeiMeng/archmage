#![cfg(unix)]

use archmage_core::process::identity_is_alive;
use archmage_core::provider::{
    acp::AcpAccess,
    qoder::{QoderSession, QoderSessionConfig, inspect_at},
};
use std::{collections::BTreeMap, os::unix::fs::PermissionsExt, time::Duration};

fn fake_binary(temp: &tempfile::TempDir) -> std::path::PathBuf {
    let path = temp.path().join("qoder");
    std::fs::copy("tests/fixtures/fake_qoder_acp.py", &path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}

fn config(temp: &tempfile::TempDir, scenario: &str) -> QoderSessionConfig {
    let repository = temp.path().join("repo");
    std::fs::create_dir_all(&repository).unwrap();
    QoderSessionConfig {
        binary: fake_binary(temp),
        repository,
        model: Some("performance".into()),
        agent_name: Some("archmage-coder".into()),
        provider_config: serde_json::json!({"mcpServers": []}),
        access: AcpAccess::WorkspaceWrite,
        diagnostics_dir: temp.path().join("diagnostics"),
        deadline: Duration::from_secs(5),
        max_line_bytes: 1024 * 1024,
        max_stream_bytes: 4 * 1024 * 1024,
        max_result_bytes: 1024,
        environment: BTreeMap::from([("FAKE_QODER_SCENARIO".into(), scenario.into())]),
    }
}

#[tokio::test]
async fn qoder_session_initializes_and_strictly_parses_a_turn() {
    let temp = tempfile::tempdir().unwrap();
    let tools = inspect_at(Some(&fake_binary(&temp))).await.unwrap();
    assert_eq!(tools.version, "1.2.3-test");

    let mut session = QoderSession::start(config(&temp, "success")).await.unwrap();
    assert_eq!(session.session_id(), "fake-session");
    let outcome = session.prompt("attempt-1", b"return JSON").await.unwrap();
    assert_eq!(outcome.conversation_id, "fake-session");
    assert_eq!(outcome.result, serde_json::json!({"status":"implemented"}));
    session.close().await.unwrap();
}

#[tokio::test]
async fn qoder_session_rejects_protocol_and_result_violations() {
    let bad_version = tempfile::tempdir().unwrap();
    assert!(
        QoderSession::start(config(&bad_version, "bad-version"))
            .await
            .is_err()
    );

    let invalid = tempfile::tempdir().unwrap();
    let mut session = QoderSession::start(config(&invalid, "invalid-result"))
        .await
        .unwrap();
    assert!(session.prompt("attempt-1", b"return JSON").await.is_err());
    session.cancel().await.unwrap();

    let oversized = tempfile::tempdir().unwrap();
    let mut session = QoderSession::start(config(&oversized, "oversized"))
        .await
        .unwrap();
    assert!(session.prompt("attempt-1", b"return JSON").await.is_err());
    session.cancel().await.unwrap();
}

#[tokio::test]
async fn qoder_session_serves_bounded_client_operations() {
    let temp = tempfile::tempdir().unwrap();
    let repository = temp.path().join("repo");
    let mut session = QoderSession::start(config(&temp, "client-tools"))
        .await
        .unwrap();
    let outcome = session
        .prompt("attempt-1", b"use client operations")
        .await
        .unwrap();
    assert_eq!(outcome.result, serde_json::json!({"status":"implemented"}));
    assert_eq!(
        std::fs::read_to_string(repository.join("acp-written.txt")).unwrap(),
        "written through ACP\n"
    );
    session.close().await.unwrap();
}

#[tokio::test]
async fn qoder_close_reaps_a_cli_that_stays_resident_after_session_close() {
    let temp = tempfile::tempdir().unwrap();
    let session = QoderSession::start(config(&temp, "linger-close"))
        .await
        .unwrap();
    let identity = session.identity().clone();
    session.close().await.unwrap();
    assert!(!identity_is_alive(&identity));
}
