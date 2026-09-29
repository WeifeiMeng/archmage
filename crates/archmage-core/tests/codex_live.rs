use archmage_core::provider::codex::{inspect, readonly_probe};
use archmage_core::{
    agent::{AgentService, ConnectionState},
    config::Config,
};

#[tokio::test]
#[ignore = "requires real authenticated Codex"]
async fn real_readonly_probe() {
    let tools = inspect().await.unwrap();
    let result = readonly_probe(&tools).await.unwrap();
    assert_eq!(result.result, serde_json::json!({"ok": true}));
    assert!(!result.thread_id.is_empty());
    assert_eq!(result.exit_code, 0);
    println!(
        "version={} thread={} exit={} result={}",
        tools.codex_version, result.thread_id, result.exit_code, result.result
    );
}

#[tokio::test]
#[ignore = "requires real authenticated Codex"]
async fn real_agent_service_verification_persists_ready_state() {
    let temp = tempfile::tempdir().unwrap();
    let config = Config::resolve(Some(&temp.path().join("data")), None).unwrap();
    config.prepare().unwrap();
    let service = AgentService::new(config.data_root.clone());
    let discovered = service.discover("codex-default").await.unwrap();
    let before = &discovered.instances[0];
    assert!(before.discovered_version.is_some());
    assert_eq!(before.connection_state, ConnectionState::Unverified);
    let verified = service.verify("codex-default").await.unwrap();
    assert_eq!(
        verified.instances[0].connection_state,
        ConnectionState::Ready
    );
    assert!(
        verified.instances[0]
            .verification_thread_id
            .as_ref()
            .is_some_and(|id| !id.is_empty())
    );
    assert_eq!(
        service.snapshot().unwrap().instances[0].connection_state,
        ConnectionState::Ready
    );
}
