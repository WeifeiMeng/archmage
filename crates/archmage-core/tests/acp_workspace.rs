use archmage_core::provider::acp::{AcpAccess, AcpWorkspacePolicy};

#[tokio::test]
async fn readonly_policy_reads_but_rejects_writes() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("notes.txt");
    std::fs::write(&file, "one\ntwo\nthree\n").unwrap();
    let policy = AcpWorkspacePolicy::new(root.path(), AcpAccess::ReadOnly).unwrap();

    assert_eq!(
        policy.read_text(&file, Some(2), Some(1)).await.unwrap(),
        "two\n"
    );
    assert!(policy.write_text(&file, "changed").await.is_err());
    assert_eq!(std::fs::read_to_string(file).unwrap(), "one\ntwo\nthree\n");
}

#[tokio::test]
async fn workspace_write_policy_writes_inside_the_root() {
    let root = tempfile::tempdir().unwrap();
    let policy = AcpWorkspacePolicy::new(root.path(), AcpAccess::WorkspaceWrite).unwrap();
    let file = root.path().join("src/new.txt");

    policy.write_text(&file, "created").await.unwrap();
    assert_eq!(std::fs::read_to_string(file).unwrap(), "created");
}

#[cfg(unix)]
#[tokio::test]
async fn policy_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let secret = outside.path().join("secret.txt");
    std::fs::write(&secret, "secret").unwrap();
    symlink(outside.path(), root.path().join("escape")).unwrap();
    let policy = AcpWorkspacePolicy::new(root.path(), AcpAccess::WorkspaceWrite).unwrap();

    assert!(
        policy
            .read_text(&root.path().join("escape/secret.txt"), None, None)
            .await
            .is_err()
    );
    assert!(
        policy
            .write_text(&root.path().join("escape/new.txt"), "bad")
            .await
            .is_err()
    );
}

#[test]
fn terminal_policy_constrains_cwd_and_dangerous_commands() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let policy = AcpWorkspacePolicy::new(root.path(), AcpAccess::WorkspaceWrite).unwrap();

    assert!(
        policy
            .validate_terminal(root.path(), "cargo", &["test".into()])
            .is_ok()
    );
    assert!(
        policy
            .validate_terminal(outside.path(), "cargo", &["test".into()])
            .is_err()
    );
    assert!(
        policy
            .validate_terminal(
                root.path(),
                "sudo",
                &["rm".into(), "-rf".into(), "/".into()]
            )
            .is_err()
    );
    assert!(
        policy
            .validate_terminal(
                root.path(),
                "rm",
                &["-rf".into(), root.path().display().to_string()]
            )
            .is_err()
    );
}
