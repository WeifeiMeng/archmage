use archmage_core::{
    agent::ProviderKind,
    provider::{
        codex::Sandbox,
        executor::{ProviderExecution, ProviderLifecycle, ProviderSession, invoke},
    },
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
#[tokio::test]
async fn deepseek_execution_reports_persistence_and_reuses_the_requested_session() {
    let temp = tempfile::tempdir().unwrap();
    let binary = temp.path().join("dsh-test");
    std::fs::write(
        &binary,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo 'dsh 0.1.0'; exit 0; fi
if [ "$1" = "--profile" ] && [ "$3" = "--help" ]; then echo '--session-id --resume-session-id'; exit 0; fi
printf '%s\n' "$*" >> invocations.log
printf '{"status":"completed"}\n'
"#,
    )
    .unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    let schema = temp.path().join("schema.json");
    std::fs::write(&schema, r#"{"type":"object"}"#).unwrap();

    let make = |session| ProviderExecution {
        provider_kind: ProviderKind::DeepseekHarness,
        executable_path: Some(binary.clone()),
        model: None,
        repository: temp.path().to_path_buf(),
        schema_path: schema.clone(),
        result_path: temp
            .path()
            .join(format!("result-{}.json", uuid::Uuid::new_v4())),
        diagnostics_dir: temp.path().join(format!("diag-{}", uuid::Uuid::new_v4())),
        sandbox: Sandbox::WorkspaceWrite,
        session,
        deadline: std::time::Duration::from_secs(10),
        max_line_bytes: 1024,
        max_stream_bytes: 1024 * 1024,
        max_result_bytes: 1024,
    };
    let mut persisted = Vec::new();
    let first = invoke(
        &make(ProviderSession::Create("session-a".into())),
        b"first",
        |event| {
            if let ProviderLifecycle::ConversationPersisted { session_id } = event {
                persisted.push(session_id);
            }
            Ok(())
        },
    )
    .await
    .unwrap();
    let second = invoke(
        &make(ProviderSession::Resume("session-a".into())),
        b"second",
        |_| Ok(()),
    )
    .await
    .unwrap();

    assert_eq!(first.conversation_id, "session-a");
    assert_eq!(second.conversation_id, "session-a");
    assert_eq!(persisted, ["session-a"]);
    let invocations = std::fs::read_to_string(temp.path().join("invocations.log")).unwrap();
    assert!(invocations.contains("--session-id session-a"));
    assert!(invocations.contains("--resume-session-id session-a"));
}
