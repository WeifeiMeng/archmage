use archmage_core::provider::codex::{LocalTools, find_executable, readonly_probe_logged};

#[tokio::test]
async fn spawn_failure_retains_private_metadata_without_exposing_raw_output() {
    let temp = tempfile::tempdir().unwrap();
    let tools = LocalTools {
        codex_binary: temp.path().join("missing-codex"),
        codex_version: "unavailable".into(),
        git_binary: find_executable("git").unwrap(),
        git_version: "test".into(),
    };
    let error = readonly_probe_logged(&tools, temp.path())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("Diagnostics:"));
    let entries: Vec<_> = std::fs::read_dir(temp.path()).unwrap().collect();
    assert_eq!(entries.len(), 1);
    let dir = entries[0].as_ref().unwrap().path();
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("metadata.json")).unwrap()).unwrap();
    assert!(
        record["capture_error"]
            .as_str()
            .unwrap()
            .contains("spawn_failed")
    );
    assert!(record["exit_code"].is_null());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
}
