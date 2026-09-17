#![cfg(unix)]

use archmage_core::process::{ProcessIdentity, Stream, capture, capture_observed, spawn_owned};
use std::{
    ffi::OsString,
    fs,
    path::Path,
    time::{Duration, Instant},
};

fn alive(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

#[tokio::test]
async fn observer_sees_bytes_before_process_exits() {
    let temp = tempfile::tempdir().unwrap();
    let gate = temp.path().join("gate");
    let args = vec![
        "-c".into(),
        "printf ready; while [ ! -f gate ]; do sleep 0.01; done; printf done >&2".into(),
    ];
    let out = capture_observed(
        Path::new("/bin/sh"),
        &args,
        Some(temp.path()),
        b"",
        Duration::from_secs(5),
        1024,
        |stream, bytes| {
            if stream == Stream::Stdout && bytes == b"ready" {
                std::fs::write(&gate, b"go")?;
            }
            Ok(())
        },
    )
    .await
    .unwrap();
    assert_eq!(out.code, Some(0));
    assert_eq!(out.stderr, b"done");
}

#[tokio::test]
async fn captures_real_stdout_stderr_and_exit() {
    let args: Vec<OsString> = ["-c", "printf out; printf err >&2; exit 7"]
        .into_iter()
        .map(Into::into)
        .collect();
    let result = capture(
        Path::new("/bin/sh"),
        &args,
        None,
        b"",
        Duration::from_secs(5),
        1024,
    )
    .await
    .unwrap();
    assert_eq!(result.code, Some(7));
    assert_eq!(result.stdout, b"out");
    assert_eq!(result.stderr, b"err");
}

#[tokio::test]
async fn deadline_terminates_process_group() {
    let result = capture(
        Path::new("/bin/sleep"),
        &["10".into()],
        None,
        b"",
        Duration::from_millis(50),
        1024,
    )
    .await;
    assert!(result.unwrap_err().to_string().contains("timed_out"));
}

#[tokio::test]
async fn output_limit_is_an_error_not_silent_truncation() {
    assert!(
        capture(
            Path::new("/bin/sh"),
            &["-c".into(), "printf 123456789".into()],
            None,
            b"",
            Duration::from_secs(5),
            4,
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn cancellation_escalates_and_kills_the_process_group() {
    let mut child = spawn_owned(
        Path::new("/bin/sh"),
        &[
            OsString::from("-c"),
            OsString::from("trap '' TERM; while :; do sleep 1; done"),
        ],
        None,
    )
    .await
    .unwrap();
    let identity = child.identity().clone();
    let started = Instant::now();

    child
        .cancel(&identity, Duration::from_millis(100))
        .await
        .unwrap();

    assert!(started.elapsed() >= Duration::from_millis(90));
    assert!(!alive(identity.pid as i32));
}

#[tokio::test]
async fn stale_identity_is_rejected_without_signalling() {
    let mut child = spawn_owned(
        Path::new("/bin/sh"),
        &[
            OsString::from("-c"),
            OsString::from("while :; do sleep 1; done"),
        ],
        None,
    )
    .await
    .unwrap();
    let correct = child.identity().clone();
    let stale = ProcessIdentity {
        started: "wrong".into(),
        ..correct.clone()
    };

    assert!(
        child
            .cancel(&stale, Duration::from_millis(50))
            .await
            .is_err()
    );
    assert!(alive(correct.pid as i32));
    child
        .cancel(&correct, Duration::from_millis(100))
        .await
        .unwrap();
}

#[tokio::test]
async fn capture_timeout_kills_background_children() {
    let dir = tempfile::tempdir().unwrap();
    let pid_file = dir.path().join("child.pid");
    let script = format!(
        "trap '' TERM; (trap '' TERM; while :; do sleep 1; done) & echo $! > '{}'; while :; do sleep 1; done",
        pid_file.display()
    );
    let result = capture(
        Path::new("/bin/sh"),
        &[OsString::from("-c"), OsString::from(script)],
        None,
        b"",
        Duration::from_millis(150),
        1024,
    )
    .await;
    assert!(result.is_err());
    let pid: i32 = fs::read_to_string(pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    for _ in 0..20 {
        if !alive(pid) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("background child {pid} survived timeout");
}
