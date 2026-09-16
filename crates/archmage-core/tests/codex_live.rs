use archmage_core::provider::codex::{inspect, readonly_probe};

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
