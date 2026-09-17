use archmage_core::{
    artifact::ArtifactStore,
    store::AttemptOutcome,
    store::Store,
    workspace::{
        ProvisionState, inspect_repository, provision, publish_changeset, reconcile, snapshot,
    },
};
use std::{fs, path::Path, process::Command};

fn git(cwd: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.name", "Archmage Test"]);
    git(
        dir.path(),
        &["config", "user.email", "archmage@example.invalid"],
    );
    fs::write(dir.path().join("tracked.txt"), "base\n").unwrap();
    git(dir.path(), &["add", "tracked.txt"]);
    git(dir.path(), &["commit", "-qm", "base"]);
    dir
}

#[test]
fn dirty_source_is_not_copied_into_run_worktree() {
    let source = repo();
    fs::write(source.path().join("tracked.txt"), "dirty\n").unwrap();
    fs::write(source.path().join("local only.txt"), "private\n").unwrap();
    let data = tempfile::tempdir().unwrap();
    let mut store = Store::open(&data.path().join("state.db")).unwrap();
    let run = store.create_run("isolate", &["design"]).unwrap();

    let created = provision(&mut store, &run, source.path(), &data.path().join("runs")).unwrap();

    assert_eq!(created.state, ProvisionState::Ready);
    assert_eq!(
        fs::read_to_string(created.path.join("tracked.txt")).unwrap(),
        "base\n"
    );
    assert!(!created.path.join("local only.txt").exists());
    assert_eq!(store.run(&run).unwrap().state, "running");
    assert_eq!(store.workspace_for_run(&run).unwrap().state, "ready");
    let events = store.events(&run, 0).unwrap();
    assert_eq!(events[events.len() - 2].event_type, "workspace.created");
    assert_eq!(events.last().unwrap().event_type, "workflow.started");
}

#[test]
fn provisions_two_runs_with_paths_containing_spaces() {
    let parent = tempfile::tempdir().unwrap();
    let source = parent.path().join("source repo");
    fs::create_dir(&source).unwrap();
    git(&source, &["init", "-q"]);
    git(&source, &["config", "user.name", "Archmage Test"]);
    git(
        &source,
        &["config", "user.email", "archmage@example.invalid"],
    );
    fs::write(source.join("a.txt"), "a").unwrap();
    git(&source, &["add", "a.txt"]);
    git(&source, &["commit", "-qm", "base"]);
    let data = parent.path().join("data with spaces");
    fs::create_dir(&data).unwrap();
    let mut store = Store::open(&data.join("state.db")).unwrap();
    let first = store.create_run("one", &["design"]).unwrap();
    let second = store.create_run("two", &["design"]).unwrap();

    let a = provision(&mut store, &first, &source, &data.join("run worktrees")).unwrap();
    let b = provision(&mut store, &second, &source, &data.join("run worktrees")).unwrap();

    assert_ne!(a.path, b.path);
    assert_ne!(a.branch, b.branch);
    assert_eq!(a.baseline, b.baseline);
}

#[test]
fn unborn_repository_is_rejected_before_intent() {
    let source = tempfile::tempdir().unwrap();
    git(source.path(), &["init", "-q"]);
    let data = tempfile::tempdir().unwrap();
    let mut store = Store::open(&data.path().join("state.db")).unwrap();
    let run = store.create_run("unborn", &["design"]).unwrap();

    assert!(provision(&mut store, &run, source.path(), &data.path().join("runs")).is_err());
    assert!(store.workspace_for_run(&run).is_err());
    assert_eq!(store.run(&run).unwrap().state, "created");
}

#[test]
fn failed_git_add_keeps_provisioning_intent() {
    let source = repo();
    let data = tempfile::tempdir().unwrap();
    let roots = data.path().join("runs");
    fs::create_dir(&roots).unwrap();
    let mut store = Store::open(&data.path().join("state.db")).unwrap();
    let run = store.create_run("collision", &["design"]).unwrap();
    let expected = roots.join(&run);
    fs::create_dir(&expected).unwrap();
    fs::write(expected.join("do-not-delete"), "owned elsewhere").unwrap();

    assert!(provision(&mut store, &run, source.path(), &roots).is_err());
    let intent = store.workspace_for_run(&run).unwrap();
    assert_eq!(intent.state, "provisioning");
    let canonical_parent = fs::canonicalize(&roots).unwrap();
    assert_eq!(intent.path, canonical_parent.join(&run).to_string_lossy());
    assert_eq!(
        fs::read_to_string(expected.join("do-not-delete")).unwrap(),
        "owned elsewhere"
    );
}

#[test]
fn snapshot_changes_for_staged_unstaged_untracked_binary_and_symlink_content() {
    let source = repo();
    let inspected = inspect_repository(source.path()).unwrap();
    let initial = snapshot(&inspected).unwrap();

    fs::write(source.path().join("tracked.txt"), "staged\n").unwrap();
    git(source.path(), &["add", "tracked.txt"]);
    let staged = snapshot(&inspected).unwrap();
    assert_ne!(initial.id, staged.id);

    fs::write(source.path().join("tracked.txt"), "unstaged\n").unwrap();
    let unstaged = snapshot(&inspected).unwrap();
    assert_ne!(staged.id, unstaged.id);

    fs::write(source.path().join("new.bin"), [0_u8, 255, 1, 2]).unwrap();
    let untracked = snapshot(&inspected).unwrap();
    assert_ne!(unstaged.id, untracked.id);

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("tracked.txt", source.path().join("link")).unwrap();
        let linked = snapshot(&inspected).unwrap();
        assert_ne!(untracked.id, linked.id);
        assert!(
            linked
                .entries
                .iter()
                .any(|e| e.path == "link" && e.kind == "symlink")
        );
    }
}

#[test]
fn reconcile_completes_an_exact_partial_provision() {
    let source = repo();
    let data = tempfile::tempdir().unwrap();
    let roots = data.path().join("runs");
    fs::create_dir(&roots).unwrap();
    let roots = fs::canonicalize(roots).unwrap();
    let inspected = inspect_repository(source.path()).unwrap();
    let mut store = Store::open(&data.path().join("state.db")).unwrap();
    let run = store.create_run("recover", &["design"]).unwrap();
    let path = roots.join(&run);
    let branch = format!("archmage/run-{run}");
    store
        .record_workspace_intent(
            &run,
            inspected.root.to_str().unwrap(),
            path.to_str().unwrap(),
            &branch,
            &inspected.baseline,
            inspected.common_dir.to_str().unwrap(),
        )
        .unwrap();
    git(
        source.path(),
        &[
            "worktree",
            "add",
            "-b",
            &branch,
            path.to_str().unwrap(),
            &inspected.baseline,
        ],
    );

    let recovered = reconcile(&mut store, &run).unwrap();

    assert_eq!(recovered.state, ProvisionState::Ready);
    assert_eq!(store.run(&run).unwrap().state, "running");
}

#[test]
fn changeset_publishes_tracked_patch_and_untracked_content() {
    let source = repo();
    fs::write(source.path().join("tracked.txt"), "changed\n").unwrap();
    fs::write(source.path().join("new.bin"), [0_u8, 255, 7]).unwrap();
    let inspected = inspect_repository(source.path()).unwrap();
    let data = tempfile::tempdir().unwrap();
    let mut store = Store::open(&data.path().join("state.db")).unwrap();
    let run = store.create_run("changes", &["code"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let node = store.nodes(&run).unwrap()[0].id.clone();
    store.ready_node(&node, 0).unwrap();
    let attempt = store.begin_attempt(&node, 1).unwrap();
    store.mark_running(&attempt.id, 0).unwrap();
    let artifacts = ArtifactStore::open(&data.path().join("artifacts")).unwrap();

    let changes = publish_changeset(&artifacts, &mut store, &attempt.id, &inspected).unwrap();
    store
        .finish_attempt(
            &attempt.id,
            1,
            AttemptOutcome::Succeeded(serde_json::json!({})),
        )
        .unwrap();

    let patch = artifacts
        .read_committed(&store, &changes.tracked_patch_artifact)
        .unwrap();
    assert!(String::from_utf8_lossy(&patch).contains("+changed"));
    assert_eq!(changes.untracked.len(), 1);
    assert_eq!(changes.untracked[0].path, "new.bin");
    assert_eq!(
        artifacts
            .read_committed(&store, &changes.untracked[0].artifact_id)
            .unwrap(),
        [0_u8, 255, 7]
    );
    assert!(
        artifacts
            .read_committed(&store, &changes.manifest_artifact)
            .is_ok()
    );
}
