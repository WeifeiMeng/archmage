use archmage_core::process::capture;
use std::{ffi::OsString, path::Path, time::Duration};

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
async fn deadline_terminates_direct_process() {
    let args = vec!["10".into()];
    let result = capture(
        Path::new("/bin/sleep"),
        &args,
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
    let args = vec!["-c".into(), "printf 123456789".into()];
    assert!(
        capture(
            Path::new("/bin/sh"),
            &args,
            None,
            b"",
            Duration::from_secs(5),
            4
        )
        .await
        .is_err()
    );
}
