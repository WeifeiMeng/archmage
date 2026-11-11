#![cfg(target_os = "macos")]

use archmage_core::process::identity_is_alive;
use archmage_core::provider::{
    acp::AcpAccess,
    qoder::{QoderSession, QoderSessionConfig, inspect_at},
};
use std::{collections::BTreeMap, os::unix::fs::PermissionsExt, process::Command, time::Duration};

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
        environment: BTreeMap::from([
            ("FAKE_QODER_SCENARIO".into(), scenario.into()),
            ("PATH".into(), "/usr/bin:/bin:/usr/sbin:/sbin".into()),
        ]),
    }
}

fn initialize_repository(path: &std::path::Path) -> String {
    for args in [
        vec!["init"],
        vec!["config", "user.email", "archmage@example.invalid"],
        vec!["config", "user.name", "Archmage Test"],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(path)
                .status()
                .unwrap()
                .success()
        );
    }
    std::fs::write(path.join("tracked.txt"), "baseline\n").unwrap();
    assert!(
        Command::new("git")
            .args(["add", "tracked.txt"])
            .current_dir(path)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .args(["commit", "-m", "baseline"])
            .current_dir(path)
            .status()
            .unwrap()
            .success()
    );
    let repository = archmage_core::workspace::inspect_repository(path).unwrap();
    archmage_core::workspace::snapshot(&repository).unwrap().id
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

    let bad_id = tempfile::tempdir().unwrap();
    assert!(
        QoderSession::start(config(&bad_id, "bad-id"))
            .await
            .is_err()
    );

    let invalid = tempfile::tempdir().unwrap();
    let mut session = QoderSession::start(config(&invalid, "invalid-result"))
        .await
        .unwrap();
    assert!(session.prompt("attempt-1", b"return JSON").await.is_err());
    session.cancel().await.unwrap();

    let wrong_session = tempfile::tempdir().unwrap();
    let mut session = QoderSession::start(config(&wrong_session, "wrong-session"))
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
async fn qoder_session_loads_only_when_capability_and_workspace_snapshot_match() {
    let matching = tempfile::tempdir().unwrap();
    let matching_config = config(&matching, "success");
    let snapshot_id = initialize_repository(&matching_config.repository);
    let session =
        QoderSession::resume_if_eligible(matching_config, "persisted-session", snapshot_id.clone())
            .await
            .unwrap();
    assert!(session.was_resumed());
    assert_eq!(session.session_id(), "persisted-session");
    session.close().await.unwrap();

    let changed = tempfile::tempdir().unwrap();
    let changed_config = config(&changed, "success");
    let old_snapshot = initialize_repository(&changed_config.repository);
    std::fs::write(changed_config.repository.join("tracked.txt"), "changed\n").unwrap();
    let session =
        QoderSession::resume_if_eligible(changed_config, "persisted-session", old_snapshot)
            .await
            .unwrap();
    assert!(!session.was_resumed());
    assert_eq!(session.session_id(), "fake-session");
    session.close().await.unwrap();

    let unsupported = tempfile::tempdir().unwrap();
    let unsupported_config = config(&unsupported, "no-load");
    let snapshot_id = initialize_repository(&unsupported_config.repository);
    let session =
        QoderSession::resume_if_eligible(unsupported_config, "persisted-session", snapshot_id)
            .await
            .unwrap();
    assert!(!session.was_resumed());
    assert_eq!(session.session_id(), "fake-session");
    session.close().await.unwrap();

    let expired = tempfile::tempdir().unwrap();
    let expired_config = config(&expired, "load-expired");
    let snapshot_id = initialize_repository(&expired_config.repository);
    let session = QoderSession::resume_if_eligible(expired_config, "expired-session", snapshot_id)
        .await
        .unwrap();
    assert!(!session.was_resumed());
    assert_eq!(session.session_id(), "fake-session");
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

#[tokio::test]
async fn qoder_close_reaps_unreleased_acp_terminals() {
    let temp = tempfile::tempdir().unwrap();
    let pid_path = temp.path().join("repo/terminal.pid");
    let mut session = QoderSession::start(config(&temp, "terminal-leak"))
        .await
        .unwrap();
    session
        .prompt("attempt-1", b"start terminal")
        .await
        .unwrap();
    let pid: i32 = std::fs::read_to_string(pid_path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(unsafe { libc::kill(pid, 0) }, 0);
    session.close().await.unwrap();
    assert_ne!(unsafe { libc::kill(pid, 0) }, 0);
}

#[tokio::test]
async fn qoder_cancellation_interrupts_terminal_wait_and_reaps_the_terminal() {
    let temp = tempfile::tempdir().unwrap();
    let session = QoderSession::start(config(&temp, "terminal-wait-hang"))
        .await
        .unwrap();
    let cancel = session.cancellation_handle();
    let prompt = tokio::spawn(async move {
        let mut session = session;
        session.prompt("attempt-1", b"wait on terminal").await
    });
    tokio::time::sleep(Duration::from_millis(200)).await;
    cancel.cancel().await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), prompt)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn qoder_readonly_os_sandbox_blocks_direct_and_terminal_writes() {
    let direct = tempfile::tempdir().unwrap();
    let mut direct_config = config(&direct, "readonly-direct-write");
    direct_config.access = AcpAccess::ReadOnly;
    let forbidden = direct_config.repository.join("forbidden.txt");
    let forbidden_child = direct_config.repository.join("forbidden-child.txt");
    let mut session = QoderSession::start(direct_config).await.unwrap();
    session
        .prompt("attempt-1", b"try direct write")
        .await
        .unwrap();
    session.close().await.unwrap();
    assert!(!forbidden.exists());
    assert!(!forbidden_child.exists());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn qoder_terminal_os_sandbox_blocks_writes_outside_the_worktree() {
    let temp = tempfile::tempdir().unwrap();
    let outside = temp.path().join("outside.txt");
    let mut config = config(&temp, "sandbox-escape");
    config
        .environment
        .insert("FAKE_ESCAPE_PATH".into(), outside.clone().into_os_string());
    let mut session = QoderSession::start(config).await.unwrap();
    session.prompt("attempt-1", b"try escape").await.unwrap();
    session.close().await.unwrap();
    assert!(!outside.exists());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn qoder_os_sandbox_blocks_direct_and_terminal_reads_outside_the_worktree() {
    let temp = tempfile::tempdir().unwrap();
    let outside = temp.path().join("outside-secret.txt");
    std::fs::write(&outside, "TOP_SECRET_CONTENT").unwrap();
    let mut config = config(&temp, "sandbox-read-escape");
    config
        .environment
        .insert("FAKE_ESCAPE_PATH".into(), outside.into_os_string());
    let mut session = QoderSession::start(config).await.unwrap();
    let outcome = session
        .prompt("attempt-1", b"try read escape")
        .await
        .unwrap();
    session.close().await.unwrap();
    assert_eq!(
        outcome.result,
        serde_json::json!({"direct": false, "terminal": false})
    );
}
