use std::process::Command;

#[test]
#[ignore = "requires real authenticated Codex"]
fn doctor_online_reports_probe_and_retains_private_logs() {
    let temp = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_archmage"))
        .arg("doctor")
        .env("ARCHMAGE_HOME", temp.path().join("data"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["authentication"], "verified");
    assert_eq!(report["probe"]["result"]["ok"], true);
    let dir = std::path::Path::new(report["probe"]["diagnostics_dir"].as_str().unwrap());
    for file in ["events.jsonl", "stderr.log", "result.json", "metadata.json"] {
        assert!(dir.join(file).is_file(), "missing {file}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(dir.join(file))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }
}
