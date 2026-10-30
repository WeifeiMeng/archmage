use archmage_core::provider::{
    codex::Sandbox,
    deepseek_harness::{
        self, DeepseekHarnessInvocation, HarnessSession, parse_result, render_task, validate_help,
    },
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn headless_help_requires_explicit_create_and_resume_options() {
    assert!(validate_help("--session-id --resume-session-id").is_ok());
    assert!(validate_help("--session-id").is_err());
}

#[test]
fn invocation_builds_explicit_create_and_resume_argv() {
    let temp = tempfile::tempdir().unwrap();
    let create = DeepseekHarnessInvocation::new(
        temp.path().to_path_buf(),
        temp.path().join("create-diagnostics"),
        Sandbox::WorkspaceWrite,
        HarnessSession::Create("session-a".into()),
    );
    let resume = DeepseekHarnessInvocation::new(
        temp.path().to_path_buf(),
        temp.path().join("resume-diagnostics"),
        Sandbox::ReadOnly,
        HarnessSession::Resume("session-a".into()),
    );
    let strings = |invocation: &DeepseekHarnessInvocation| {
        invocation
            .argv("task")
            .into_iter()
            .map(|part| part.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        strings(&create),
        ["--profile", "headless", "--session-id", "session-a", "task"]
    );
    assert_eq!(
        strings(&resume),
        [
            "--profile",
            "headless",
            "--resume-session-id",
            "session-a",
            "task"
        ]
    );
}

#[test]
fn task_contains_context_schema_and_strict_output_instruction() {
    let task = render_task(
        br#"{"goal":"change it"}"#,
        r#"{"type":"object","required":["status"]}"#,
    )
    .unwrap();
    assert!(task.contains(r#"{"goal":"change it"}"#));
    assert!(task.contains(r#""required":["status"]"#));
    assert!(task.contains("single JSON object"));
    assert!(task.contains("Do not use Markdown"));
}

#[test]
fn result_parser_rejects_prose_wrapped_json() {
    assert_eq!(parse_result(b"{\"ok\":true}\n", 1024).unwrap()["ok"], true);
    assert!(parse_result(br#"result: {"ok":true}"#, 1024).is_err());
    assert!(parse_result(b"{}", 1).is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn discovery_and_invocation_use_the_explicit_dsh_binary() {
    let temp = tempfile::tempdir().unwrap();
    let binary = temp.path().join("custom-dsh");
    std::fs::write(
        &binary,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo 'dsh 0.1.0'; exit 0; fi
if [ "$1" = "--profile" ] && [ "$3" = "--help" ]; then echo '--session-id --resume-session-id'; exit 0; fi
printf '%s' "$DSH_PERMISSION_MODE" > permission.txt
printf '{"ok":true}\n'
"#,
    )
    .unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    let tools = deepseek_harness::inspect_at(Some(&binary)).await.unwrap();
    assert_eq!(tools.dsh_version, "dsh 0.1.0");

    let invocation = DeepseekHarnessInvocation::new(
        temp.path().to_path_buf(),
        temp.path().join("diagnostics"),
        Sandbox::ReadOnly,
        HarnessSession::Create("session-a".into()),
    );
    let mut persisted = false;
    let outcome = deepseek_harness::invoke_with_identity(
        &tools,
        &invocation,
        b"context",
        r#"{"type":"object"}"#,
        |_| Ok(()),
        || {
            persisted = true;
            Ok(())
        },
    )
    .await
    .unwrap();
    assert!(persisted);
    assert_eq!(outcome.session_id.as_deref(), Some("session-a"));
    assert_eq!(outcome.result["ok"], true);
    assert_eq!(
        std::fs::read_to_string(temp.path().join("permission.txt")).unwrap(),
        "read-only"
    );
}
