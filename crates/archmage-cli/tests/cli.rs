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
