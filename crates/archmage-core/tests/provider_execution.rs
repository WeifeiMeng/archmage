use archmage_core::{
    agent::ProviderKind,
    provider::{
        codex::Sandbox,
        executor::{ProviderExecution, invoke},
    },
};

#[tokio::test]
async fn direct_deepseek_execution_is_reserved_for_the_runtime_pool() {
    let temp = tempfile::tempdir().unwrap();
    let execution = ProviderExecution {
        provider_kind: ProviderKind::DeepseekHarness,
        executable_path: None,
        model: Some("deepseek-v4-flash".into()),
        repository: temp.path().to_path_buf(),
        schema_path: temp.path().join("schema.json"),
        result_path: temp.path().join("result.json"),
        diagnostics_dir: temp.path().join("diagnostics"),
        sandbox: Sandbox::WorkspaceWrite,
        deadline: std::time::Duration::from_secs(1),
        max_line_bytes: 1024,
        max_stream_bytes: 1024,
        max_result_bytes: 1024,
    };
    let error = invoke(&execution, b"task", |_| Ok(())).await.unwrap_err();
    assert!(error.to_string().contains("Runtime provider session pool"));
}

#[tokio::test]
async fn direct_qoder_execution_is_reserved_for_the_runtime_pool() {
    let temp = tempfile::tempdir().unwrap();
    let execution = ProviderExecution {
        provider_kind: ProviderKind::Qoder,
        executable_path: None,
        model: None,
        repository: temp.path().to_path_buf(),
        schema_path: temp.path().join("schema.json"),
        result_path: temp.path().join("result.json"),
        diagnostics_dir: temp.path().join("diagnostics"),
        sandbox: Sandbox::WorkspaceWrite,
        deadline: std::time::Duration::from_secs(1),
        max_line_bytes: 1024,
        max_stream_bytes: 1024,
        max_result_bytes: 1024,
    };
    let error = invoke(&execution, b"task", |_| Ok(())).await.unwrap_err();
    assert!(error.to_string().contains("Runtime provider session pool"));
}
