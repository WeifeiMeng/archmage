//! Bounded direct-child process capture. Durable process-group ownership is a batch B concern.
use crate::error::{Error, Result};
use std::{ffi::OsString, path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
};

#[derive(Debug)]
pub struct Captured {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

async fn bounded(
    mut stream: impl AsyncRead + Unpin,
    max: usize,
    kind: Stream,
    observer: &std::cell::RefCell<impl FnMut(Stream, &[u8]) -> Result<()>>,
) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let count = stream.read(&mut chunk).await?;
        if count == 0 {
            break;
        }
        if result.len().saturating_add(count) > max {
            return Err(Error::Limit("process output limit exceeded".into()));
        }
        result.extend_from_slice(&chunk[..count]);
        (observer.borrow_mut())(kind, &chunk[..count])?;
    }
    Ok(result)
}

pub async fn capture(
    binary: &Path,
    args: &[OsString],
    cwd: Option<&Path>,
    input: &[u8],
    deadline: Duration,
    max: usize,
) -> Result<Captured> {
    capture_observed(binary, args, cwd, input, deadline, max, |_, _| Ok(())).await
}

/// Observer runs while pipes are drained, before child exit. Errors stop capture.
pub async fn capture_observed(
    binary: &Path,
    args: &[OsString],
    cwd: Option<&Path>,
    input: &[u8],
    deadline: Duration,
    max: usize,
    observer: impl FnMut(Stream, &[u8]) -> Result<()>,
) -> Result<Captured> {
    let observer = std::cell::RefCell::new(observer);
    let mut command = Command::new(binary);
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let mut child = command.spawn().map_err(|e| Error::Spawn(e.to_string()))?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let work = async {
        let write = async {
            stdin.write_all(input).await?;
            stdin.shutdown().await?;
            drop(stdin);
            Ok::<_, Error>(())
        };
        let wait = async { Ok::<_, Error>(child.wait().await?) };
        let (_, stdout, stderr, status) = tokio::try_join!(
            write,
            bounded(stdout, max, Stream::Stdout, &observer),
            bounded(stderr, max, Stream::Stderr, &observer),
            wait
        )?;
        Ok(Captured {
            code: status.code(),
            stdout,
            stderr,
        })
    };
    let result = match tokio::time::timeout(deadline, work).await {
        Ok(r) => r,
        Err(_) => Err(Error::TimedOut),
    };
    if result.is_err() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    result
}
