use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_archmage"))
}

#[test]
fn help_lists_six_commands_without_creating_data() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("absent");
    let output = cli()
        .args(["--help"])
        .env("ARCHMAGE_HOME", &home)
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for name in ["doctor", "run", "status", "logs", "resume", "cancel"] {
        assert!(text.contains(name), "missing {name}");
    }
    assert!(!home.exists());
}

#[test]
fn missing_run_arguments_are_rejected_without_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("absent");
    let output = cli()
        .arg("run")
        .env("ARCHMAGE_HOME", &home)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(!home.exists());
}

#[test]
fn unimplemented_workflow_never_claims_success() {
    let output = cli()
        .args(["run", "--repo", ".", "--goal", "health", "--check", "true"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(20));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("not implemented")
    );
}

#[test]
fn blank_goal_is_rejected() {
    let output = cli()
        .args(["run", "--repo", ".", "--goal", "  ", "--check", "true"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn internal_check_requires_only_endpoint_attempt_and_check() {
    let output = cli()
        .args(["internal", "check", "--attempt", "a", "--check", "unit"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let help = cli()
        .args(["internal", "check", "--help"])
        .output()
        .unwrap();
    let text = String::from_utf8(help.stdout).unwrap();
    assert!(text.contains("--endpoint") && text.contains("--attempt") && text.contains("--check"));
    assert!(!text.contains("--command") && !text.contains("--repo"));
}

#[test]
#[ignore = "requires installed local Codex and Git; no model request"]
fn offline_doctor_opens_database_without_auth_probe() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("data");
    let out = cli()
        .args(["doctor", "--offline"])
        .env("ARCHMAGE_HOME", &root)
        .output()
        .unwrap();
    assert!(out.status.success());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["database"]["schema_version"], 5);
    assert_eq!(value["authentication"], "not_verified_offline");
    assert!(root.join("archmage.db").is_file());
}

#[test]
fn doctor_missing_tools_is_explicit_without_creating_data() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("absent");
    let output = cli()
        .args(["doctor", "--offline"])
        .env("PATH", "")
        .env("ARCHMAGE_HOME", &root)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(20));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("executable not found")
    );
    assert!(!root.exists());
}
