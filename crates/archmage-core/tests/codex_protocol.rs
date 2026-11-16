use agent_client_protocol::v1::schema::SessionUpdate;
use archmage_core::provider::codex::{self, CodexInvocation, Sandbox, validate_help};
use archmage_core::provider::codex_event::Decoder;
use archmage_core::provider::progress::{
    MAX_PROGRESS_TEXT_BYTES, ProviderProgress, codex_progress, qoder_progress,
};
use serde_json::json;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn acp_session_frame(update: serde_json::Value) -> serde_json::Value {
    let update: SessionUpdate = serde_json::from_value(update).unwrap();
    json!({"params": {"update": serde_json::to_value(update).unwrap()}})
}

#[test]
fn sanitized_real_cli_fixture_decodes() {
    let bytes = include_bytes!("fixtures/codex-events.jsonl");
    let events = Decoder::new(1024).push(bytes).unwrap();
    assert_eq!(events.len(), 4);
    assert_eq!(events[0]["type"], "thread.started");
    assert_eq!(events[3]["type"], "turn.completed");
}

#[test]
fn fragmented_utf8_unknown_and_unterminated_events_are_preserved() {
    let input = "{\"type\":\"thread.started\",\"thread_id\":\"abc\"}\n{\"type\":\"future.event\",\"text\":\"中文\"}\n{\"type\":\"turn.completed\"}";
    let mut d = Decoder::new(1024);
    let mut events = Vec::new();
    for b in input.as_bytes() {
        events.extend(d.push(&[*b]).unwrap());
    }
    events.extend(d.finish().unwrap());
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["thread_id"], "abc");
    assert_eq!(events[1]["text"], "中文");
    assert!(events[2].get("thread_id").is_none());
}

#[test]
fn invalid_and_oversized_lines_are_rejected() {
    assert!(Decoder::new(4).push(b"12345").is_err());
    assert!(Decoder::new(1024).push(b"not json\n").is_err());
    assert!(Decoder::new(1024).push(b"[]\n").is_err());
}

#[test]
fn failure_events_are_preserved() {
    let events = Decoder::new(1024)
        .push(b"{\"type\":\"turn.failed\"}\n{\"type\":\"error\"}\n")
        .unwrap();
    assert_eq!(events[0]["type"], "turn.failed");
    assert_eq!(events[1]["type"], "error");
}

#[test]
fn codex_progress_allows_public_items_and_drops_reasoning() {
    assert_eq!(
        codex_progress(&json!({
            "type": "item.completed",
            "item": {"type": "agent_message", "text": "Working on tests"}
        })),
        Some(ProviderProgress::message(
            "Agent message",
            "Working on tests"
        ))
    );
    assert!(
        codex_progress(&json!({
            "type": "item.completed",
            "item": {"type": "reasoning", "text": "private"}
        }))
        .is_none()
    );
}

#[test]
fn codex_progress_never_exposes_raw_commands() {
    let mapped = codex_progress(&json!({
        "type": "item.completed",
        "item": {
            "type": "command_execution",
            "command": "API_TOKEN=secret curl -H 'Authorization: Bearer hidden' https://example.test",
            "status": "completed",
            "exit_code": 0
        }
    }))
    .unwrap();
    assert_eq!(mapped.label, "Run command");
    assert_eq!(mapped.text, None);
    assert!(!serde_json::to_string(&mapped).unwrap().contains("secret"));
    assert!(!serde_json::to_string(&mapped).unwrap().contains("hidden"));
}

#[test]
fn progress_text_is_utf8_bounded() {
    let mapped = codex_progress(&json!({
        "type": "item.completed",
        "item": {"type": "agent_message", "text": "你".repeat(MAX_PROGRESS_TEXT_BYTES)}
    }))
    .unwrap();
    let text = mapped.text.unwrap();
    assert!(text.len() <= MAX_PROGRESS_TEXT_BYTES);
    assert!(text.is_char_boundary(text.len()));
}

#[test]
fn qoder_progress_drops_agent_thought_and_maps_tool_status() {
    assert!(
        qoder_progress(&json!({
            "params": {"update": {
                "sessionUpdate": "agent_thought_chunk",
                "content": {"type": "text", "text": "private"}
            }}
        }))
        .is_none()
    );
    let mapped = qoder_progress(&json!({
        "params": {"update": {
            "sessionUpdate": "tool_call",
            "toolCallId": "t1",
            "title": "Run tests",
            "status": "pending"
        }}
    }))
    .unwrap();
    assert_eq!(mapped.kind, "tool");
    assert_eq!(mapped.status.as_deref(), Some("pending"));
}

#[test]
fn qoder_progress_maps_real_acp_v1_updates() {
    let plan = qoder_progress(&acp_session_frame(json!({
            "sessionUpdate": "plan",
            "entries": [
                {"content": "Inspect code", "priority": "high", "status": "completed"},
                {"content": "Run tests", "priority": "medium", "status": "in_progress"}
            ]
    })))
    .unwrap();
    assert_eq!(plan.status.as_deref(), Some("in_progress"));
    assert!(plan.text.as_deref().unwrap().contains("Run tests"));

    let usage = qoder_progress(&acp_session_frame(json!({
            "sessionUpdate": "usage_update", "used": 1200, "size": 8000,
            "cost": {"amount": 0.42, "currency": "USD"}
    })))
    .unwrap();
    assert_eq!(usage.metadata["used"], 1200);
    assert_eq!(usage.metadata["size"], 8000);
    assert_eq!(usage.metadata["costAmount"], 0.42);
    assert_eq!(usage.metadata["costCurrency"], "USD");

    let mode = qoder_progress(&acp_session_frame(json!({
        "sessionUpdate": "current_mode_update", "currentModeId": "code"
    })))
    .unwrap();
    assert_eq!(mode.metadata["currentModeId"], "code");

    let config = qoder_progress(&acp_session_frame(json!({
        "sessionUpdate": "config_option_update", "configOptions": [
            {"id": "model", "name": "Model", "type": "select", "currentValue": "private-value", "options": []}
        ]
    })))
    .unwrap();
    assert_eq!(config.metadata["optionCount"], 1);
    assert!(
        !serde_json::to_string(&config)
            .unwrap()
            .contains("private-value")
    );

    let notice = qoder_progress(&acp_session_frame(json!({
            "sessionUpdate": "notice", "title": "Connection degraded",
            "description": "Retrying safely", "severity": "warning"
    })))
    .unwrap();
    assert_eq!(notice.label, "Connection degraded");
    assert_eq!(notice.status.as_deref(), Some("warning"));
    assert_eq!(notice.text.as_deref(), Some("Retrying safely"));
}

#[test]
fn all_required_flags_must_be_present() {
    let flags = "--cd --sandbox --json --output-schema --output-last-message";
    assert!(validate_help(flags).is_ok());
    let error = validate_help("--json").unwrap_err().to_string();
    assert!(error.contains("--output-schema"));
}

#[test]
fn invocation_builds_schema_bound_argv_without_prompt_text() {
    let temp = tempfile::tempdir().unwrap();
    let invocation = CodexInvocation::new(
        temp.path().join("repo"),
        temp.path().join("schema.json"),
        temp.path().join("result.json"),
        temp.path().join("diagnostics"),
        Sandbox::WorkspaceWrite,
    );
    let argv = invocation
        .argv()
        .iter()
        .map(|part| part.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(
        argv.windows(2)
            .any(|pair| pair == ["--sandbox", "workspace-write"])
    );
    assert!(argv.windows(2).any(|pair| pair[0] == "--output-schema"));
    assert!(
        argv.windows(2)
            .any(|pair| pair[0] == "--output-last-message")
    );
    assert_eq!(argv.last().map(String::as_str), Some("-"));
    assert!(!argv.iter().any(|part| part.contains("secret goal")));
}

#[test]
fn custom_permission_profile_omits_builtin_sandbox_override() {
    let temp = tempfile::tempdir().unwrap();
    let mut invocation = CodexInvocation::new(
        temp.path().join("repo"),
        temp.path().join("schema.json"),
        temp.path().join("result.json"),
        temp.path().join("diagnostics"),
        Sandbox::WorkspaceWrite,
    );
    invocation.use_custom_permissions();
    let argv = invocation
        .argv()
        .iter()
        .map(|part| part.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(!argv.iter().any(|part| part == "--sandbox"));
}

#[test]
fn configured_model_is_passed_as_an_explicit_argument() {
    let temp = tempfile::tempdir().unwrap();
    let mut invocation = CodexInvocation::new(
        temp.path().join("repo"),
        temp.path().join("schema.json"),
        temp.path().join("result.json"),
        temp.path().join("diagnostics"),
        Sandbox::ReadOnly,
    );
    invocation.model = Some("gpt-5.4".into());
    let argv = invocation
        .argv()
        .iter()
        .map(|part| part.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(argv.windows(2).any(|pair| pair == ["--model", "gpt-5.4"]));
}

#[cfg(unix)]
#[tokio::test]
async fn discovery_uses_the_instances_explicit_executable() {
    let temp = tempfile::tempdir().unwrap();
    let binary = temp.path().join("custom-codex");
    std::fs::write(
        &binary,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'codex 1.2.3'; exit 0; fi\nprintf '%s\\n' '--cd --sandbox --json --output-schema --output-last-message'\n",
    )
    .unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();

    let tools = codex::inspect_at(Some(&binary)).await.unwrap();

    assert_eq!(tools.codex_binary, binary);
    assert_eq!(tools.codex_version, "codex 1.2.3");
}
