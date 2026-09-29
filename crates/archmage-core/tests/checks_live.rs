#![cfg(unix)]

use archmage_core::{
    artifact::ArtifactStore,
    capability::CapabilityKind,
    check_runner::{CheckRunner, FrozenCheck, load_evidence, serve_check_once},
    contract::{ContractContext, validate_output},
    process::ProcessIdentity,
    store::Store,
    workspace::inspect_repository,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncWriteExt, process::Command};

fn git(cwd: &Path, args: &[&str]) {
    assert!(
        std::process::Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .unwrap()
            .success()
    );
}

#[tokio::test]
#[ignore = "requires real authenticated Codex Tester"]
async fn real_tester_requests_success_and_failure_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let repo_path = dir.path().join("run");
    fs::create_dir(&repo_path).unwrap();
    git(&repo_path, &["init", "-q"]);
    git(&repo_path, &["config", "user.name", "Archmage Test"]);
    git(
        &repo_path,
        &["config", "user.email", "archmage@example.invalid"],
    );
    fs::write(repo_path.join("base.txt"), "base").unwrap();
    git(&repo_path, &["add", "base.txt"]);
    git(&repo_path, &["commit", "-qm", "base"]);
    let mut store = Store::open(&dir.path().join("state.db")).unwrap();
    let run = store.create_run("live tester", &["test"]).unwrap();
    store.start_run(&run, 0).unwrap();
    let generation = store.claim_run_owner(&run).unwrap();
    let node = store.nodes(&run).unwrap()[0].id.clone();
    store.ready_node(&node, 0).unwrap();
    let attempt = store.begin_attempt(&node, 1).unwrap();
    store
        .attach_attempt_process(
            &attempt.id,
            0,
            generation,
            &ProcessIdentity {
                pid: 1,
                pgid: 1,
                started: "live-test".into(),
            },
        )
        .unwrap();
    let artifacts = ArtifactStore::open(&dir.path().join("artifacts")).unwrap();
    let repo = inspect_repository(&repo_path).unwrap();
    let mut runner = CheckRunner::new(
        &mut store,
        &artifacts,
        &attempt.id,
        generation,
        &repo,
        vec![
            FrozenCheck::new("pass", "printf pass", Duration::from_secs(10)),
            FrozenCheck::new("fail", "printf fail >&2; exit 7", Duration::from_secs(10)),
        ],
    )
    .unwrap();
    let socket = repo_path.join("check.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let helper = workspace.join("target/debug/archmage");
    assert!(helper.is_file(), "build archmage-cli before live test");
    let schema = workspace.join("crates/archmage-core/capabilities/v1/tester.schema.json");
    let result = dir.path().join("tester-result.json");
    let prompt = format!(
        "Act as Tester. Invoke exactly these two legacy check helpers and read their JSON evidence: {} internal check --endpoint {} --attempt {} --check pass ; {} internal check --endpoint {} --attempt {} --check fail. Return schema-valid Tester JSON with status failed. In commands report the underlying commands `printf pass` with exit_code 0/status passed and `printf fail >&2; exit 7` with exit_code 7/status failed. Include concise summaries and an empty limitations array. Do not edit files.",
        helper.display(),
        socket.display(),
        attempt.id,
        helper.display(),
        socket.display(),
        attempt.id
    );
    let socket_policy = format!(
        "permissions.archmage_tester.network.unix_sockets={{{}=\"allow\"}}",
        serde_json::to_string(&socket.to_string_lossy()).unwrap()
    );
    let mut child = Command::new("codex")
        .args([
            "exec",
            "-C",
            repo_path.to_str().unwrap(),
            "--strict-config",
            "-c",
            "default_permissions=\"archmage_tester\"",
            "-c",
            "permissions.archmage_tester.extends=\":workspace\"",
            "-c",
            "permissions.archmage_tester.network.enabled=true",
            "-c",
            "features.network_proxy.enabled=true",
            "-c",
            &socket_policy,
            "--json",
            "--output-schema",
            schema.to_str().unwrap(),
            "-o",
            result.to_str().unwrap(),
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(prompt.as_bytes())
        .await
        .unwrap();
    let server = async {
        serve_check_once(&listener, &mut runner).await.unwrap();
        serve_check_once(&listener, &mut runner).await.unwrap();
    };
    let output = tokio::time::timeout(Duration::from_secs(180), async {
        tokio::pin!(server);
        let output = child.wait_with_output();
        tokio::pin!(output);
        tokio::select! {
            biased;
            _ = &mut server => output.await.unwrap(),
            result = &mut output => {
                let result = result.unwrap();
                panic!("Codex exited before both check requests: status={:?} stdout={} stderr={}", result.status.code(), String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
            }
        }
    }).await.unwrap();
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    drop(runner);
    let pass = load_evidence(&store, &attempt.id, "pass").unwrap();
    let fail = load_evidence(&store, &attempt.id, "fail").unwrap();
    assert_eq!(pass.exit_code, Some(0));
    assert_eq!(fail.exit_code, Some(7));
    let output = fs::read(result).unwrap();
    let validated =
        validate_output(CapabilityKind::Tester, &output, &ContractContext::default()).unwrap();
    assert_eq!(validated["status"], "failed");
}
