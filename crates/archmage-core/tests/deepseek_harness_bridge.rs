use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
};

fn bridge_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/deepseek_harness_bridge.py")
}

fn fake_sdk_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/deepseek_harness_sdk")
}

fn start_frame(root: &std::path::Path) -> Value {
    json!({
        "op": "start", "protocol": 1, "cwd": root,
        "provider": "deepseek-official", "model": "deepseek-v4-flash",
        "sessionId": "archmage-test", "cordis": root.join("cordis.yml"),
        "sessionRoot": root.join("sessions"), "maxResultBytes": 32,
        "requestTimeoutSeconds": 10
    })
}

fn run_bridge(
    _root: &std::path::Path,
    log: &std::path::Path,
    frames: &[Value],
    mode: Option<&str>,
) -> std::process::Output {
    let mut command = Command::new("python3");
    command
        .args(["-u", bridge_path().to_str().unwrap()])
        .env("PYTHONPATH", fake_sdk_path())
        .env("FAKE_DSH_LOG", log)
        .env("DSH_HOME", _root.join("dsh-home"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(mode) = mode {
        command.env("FAKE_DSH_MODE", mode);
    }
    let mut child = command.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for frame in frames {
        writeln!(stdin, "{}", frame).unwrap();
    }
    drop(stdin);
    child.wait_with_output().unwrap()
}

#[test]
fn bridge_reuses_one_named_sdk_session_for_two_invocations() {
    let temp = tempfile::tempdir().unwrap();
    let log = temp.path().join("sdk.jsonl");
    let mut child = Command::new("python3")
        .args(["-u", bridge_path().to_str().unwrap()])
        .env("PYTHONPATH", fake_sdk_path())
        .env("FAKE_DSH_LOG", &log)
        .env("DSH_HOME", temp.path().join("dsh-home"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for frame in [
        json!({
            "op": "start", "protocol": 1, "cwd": temp.path(),
            "provider": "deepseek-official", "model": "deepseek-v4-flash",
            "sessionId": "archmage-test", "cordis": temp.path().join("cordis.yml"),
            "sessionRoot": temp.path().join("sessions"), "maxResultBytes": 4096,
            "requestTimeoutSeconds": 10
        }),
        json!({"op":"invoke", "id":"one", "task":"first", "schema":{"type":"object"}}),
        json!({"op":"invoke", "id":"two", "task":"second", "schema":{"type":"object"}}),
        json!({"op":"close"}),
    ] {
        writeln!(stdin, "{}", frame).unwrap();
    }
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let frames = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        frames
            .iter()
            .map(|frame| frame["op"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["ready", "result", "result", "closed"]
    );
    assert_eq!(frames[1]["id"], "one");
    assert_eq!(frames[2]["id"], "two");
    assert_eq!(frames[1]["sessionId"], "archmage-test");
    assert_eq!(frames[2]["sessionId"], "archmage-test");

    let events = std::fs::read_to_string(log)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        events
            .iter()
            .filter(|event| event["event"] == "harness")
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event["event"] == "run")
            .count(),
        2
    );
    let harness = events
        .iter()
        .find(|event| event["event"] == "harness")
        .unwrap();
    assert_eq!(harness["profile"], "sdk");
    assert_eq!(
        harness["dshHome"],
        temp.path().join("dsh-home").to_string_lossy().as_ref()
    );
    assert_eq!(
        harness["patches"],
        json!([temp.path().join("cordis.yml").to_string_lossy()])
    );
    assert_eq!(events.last().unwrap()["event"], "close");
}

#[test]
fn bridge_fails_closed_on_duplicate_invocations_and_sdk_failures() {
    let temp = tempfile::tempdir().unwrap();
    let invoke = json!({"op":"invoke", "id":"same", "task":"task", "schema":{"type":"object"}});
    let duplicate = run_bridge(
        temp.path(),
        &temp.path().join("duplicate.jsonl"),
        &[start_frame(temp.path()), invoke.clone(), invoke],
        None,
    );
    assert!(!duplicate.status.success());
    let duplicate_frames = String::from_utf8(duplicate.stdout).unwrap();
    assert!(
        duplicate_frames
            .lines()
            .any(|line| line.contains("duplicate invocation id"))
    );

    for mode in ["error", "finish-error", "oversize"] {
        let output = run_bridge(
            temp.path(),
            &temp.path().join(format!("{mode}.jsonl")),
            &[
                start_frame(temp.path()),
                json!({"op":"invoke", "id":"one", "task":"task", "schema":{"type":"object"}}),
            ],
            Some(mode),
        );
        assert!(!output.status.success(), "mode {mode}");
        let last = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .last()
            .unwrap()
            .to_owned();
        let error: Value = serde_json::from_str(&last).unwrap();
        assert_eq!(error["op"], "error");
        assert_eq!(error["id"], "one");
    }
}
