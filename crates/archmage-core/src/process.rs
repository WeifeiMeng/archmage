//! Bounded process capture with Unix process-group ownership.
use crate::error::{Error, Result};
use std::{
    ffi::OsString,
    path::Path,
    process::{ExitStatus, Stdio},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub pgid: i32,
    pub started: String,
}

pub struct OwnedProcess {
    child: tokio::process::Child,
    identity: ProcessIdentity,
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command) {}

#[cfg(unix)]
fn signal_group(pgid: i32, signal: i32) -> Result<()> {
    if pgid <= 0 {
        return Err(Error::Preflight("invalid process group identity".into()));
    }
    let rc = unsafe { libc::kill(-pgid, signal) };
    if rc == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().into())
    }
}

#[cfg(unix)]
fn group_exists(pgid: i32) -> bool {
    unsafe { libc::kill(-pgid, 0) == 0 }
}

#[cfg(unix)]
fn process_started(pid: u32) -> Result<String> {
    let result = std::process::Command::new("/bin/ps")
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .output()?;
    if !result.status.success() {
        return Err(Error::Preflight(
            "process identity cannot be confirmed".into(),
        ));
    }
    let started = String::from_utf8_lossy(&result.stdout).trim().to_owned();
    if started.is_empty() {
        return Err(Error::Preflight(
            "process identity cannot be confirmed".into(),
        ));
    }
    Ok(started)
}

#[cfg(unix)]
pub fn identity_is_alive(identity: &ProcessIdentity) -> bool {
    process_started(identity.pid).is_ok_and(|started| started == identity.started)
        && group_exists(identity.pgid)
}

#[cfg(not(unix))]
pub fn identity_is_alive(_identity: &ProcessIdentity) -> bool {
    false
}

#[cfg(unix)]
pub async fn cancel_process(identity: &ProcessIdentity, grace: Duration) -> Result<()> {
    if !identity_is_alive(identity) {
        return Ok(());
    }
    signal_group(identity.pgid, libc::SIGTERM)?;
    let deadline = Instant::now() + grace;
    while Instant::now() < deadline && group_exists(identity.pgid) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    if group_exists(identity.pgid) {
        signal_group(identity.pgid, libc::SIGKILL)?;
    }
    for _ in 0..50 {
        if !group_exists(identity.pgid) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Err(Error::Preflight(
        "cancelled process group did not exit".into(),
    ))
}

#[cfg(not(unix))]
pub async fn cancel_process(_identity: &ProcessIdentity, _grace: Duration) -> Result<()> {
    Err(Error::Preflight(
        "process cancellation is supported on Unix in S0".into(),
    ))
}

pub async fn spawn_owned(
    binary: &Path,
    args: &[OsString],
    cwd: Option<&Path>,
) -> Result<OwnedProcess> {
    let mut command = Command::new(binary);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    configure_process_group(&mut command);
    let child = command.spawn().map_err(|e| Error::Spawn(e.to_string()))?;
    let pid = child
        .id()
        .ok_or_else(|| Error::Spawn("child PID unavailable".into()))?;
    #[cfg(unix)]
    let identity = ProcessIdentity {
        pid,
        pgid: pid as i32,
        started: process_started(pid)?,
    };
    #[cfg(not(unix))]
    let identity = ProcessIdentity {
        pid,
        pgid: pid as i32,
        started: String::new(),
    };
    Ok(OwnedProcess { child, identity })
}

impl OwnedProcess {
    pub fn identity(&self) -> &ProcessIdentity {
        &self.identity
    }

    #[cfg(unix)]
    pub async fn cancel(
        &mut self,
        expected: &ProcessIdentity,
        grace: Duration,
    ) -> Result<ExitStatus> {
        if expected != &self.identity || process_started(expected.pid)? != expected.started {
            return Err(Error::Preflight(
                "process identity mismatch; refusing to signal".into(),
            ));
        }
        signal_group(expected.pgid, libc::SIGTERM)?;
        let deadline = Instant::now() + grace;
        let mut status = None;
        while Instant::now() < deadline {
            if status.is_none() {
                status = self.child.try_wait()?;
            }
            if !group_exists(expected.pgid) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        if group_exists(expected.pgid) {
            signal_group(expected.pgid, libc::SIGKILL)?;
        }
        match status {
            Some(status) => Ok(status),
            None => Ok(self.child.wait().await?),
        }
    }
}

#[derive(Debug)]
pub struct Captured {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[cfg(unix)]
fn exit_signal(status: &ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn exit_signal(_status: &ExitStatus) -> Option<i32> {
    None
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
    observer: &std::sync::Mutex<impl FnMut(Stream, &[u8]) -> Result<()> + Send>,
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
        (observer.lock().expect("process observer lock"))(kind, &chunk[..count])?;
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

pub async fn capture_clean(
    binary: &Path,
    args: &[OsString],
    cwd: &Path,
    deadline: Duration,
    max: usize,
    environment: &[(OsString, OsString)],
) -> Result<Captured> {
    let observer = std::sync::Mutex::new(|_: Stream, _: &[u8]| Ok(()));
    let mut command = Command::new(binary);
    command
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(environment.iter().cloned())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    configure_process_group(&mut command);
    let mut child = command.spawn().map_err(|e| Error::Spawn(e.to_string()))?;
    let pgid = child.id().map(|id| id as i32);
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let work = async {
        let (stdout, stderr, status) = tokio::try_join!(
            bounded(stdout, max, Stream::Stdout, &observer),
            bounded(stderr, max, Stream::Stderr, &observer),
            async { Ok::<_, Error>(child.wait().await?) }
        )?;
        Ok(Captured {
            code: status.code(),
            signal: exit_signal(&status),
            stdout,
            stderr,
        })
    };
    let result = match tokio::time::timeout(deadline, work).await {
        Ok(result) => result,
        Err(_) => Err(Error::TimedOut),
    };
    if result.is_err() {
        #[cfg(unix)]
        if let Some(pgid) = pgid {
            let _ = signal_group(pgid, libc::SIGTERM);
            tokio::time::sleep(Duration::from_millis(100)).await;
            if group_exists(pgid) {
                let _ = signal_group(pgid, libc::SIGKILL);
            }
        }
        let _ = child.wait().await;
    }
    result
}

/// Observer runs while pipes are drained, before child exit. Errors stop capture.
pub async fn capture_observed(
    binary: &Path,
    args: &[OsString],
    cwd: Option<&Path>,
    input: &[u8],
    deadline: Duration,
    max: usize,
    observer: impl FnMut(Stream, &[u8]) -> Result<()> + Send,
) -> Result<Captured> {
    capture_observed_with_identity(
        binary,
        args,
        cwd,
        input,
        deadline,
        max,
        |_| Ok(()),
        observer,
    )
    .await
}

/// Captures a process while publishing its verified OS identity before stdin is written.
pub async fn capture_observed_with_identity(
    binary: &Path,
    args: &[OsString],
    cwd: Option<&Path>,
    input: &[u8],
    deadline: Duration,
    max: usize,
    on_spawn: impl FnOnce(&ProcessIdentity) -> Result<()> + Send,
    observer: impl FnMut(Stream, &[u8]) -> Result<()> + Send,
) -> Result<Captured> {
    let observer = std::sync::Mutex::new(observer);
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
    configure_process_group(&mut command);
    let mut child = command.spawn().map_err(|e| Error::Spawn(e.to_string()))?;
    let pid = child
        .id()
        .ok_or_else(|| Error::Spawn("child PID unavailable".into()))?;
    #[cfg(unix)]
    let identity = ProcessIdentity {
        pid,
        pgid: pid as i32,
        started: process_started(pid)?,
    };
    #[cfg(not(unix))]
    let identity = ProcessIdentity {
        pid,
        pgid: pid as i32,
        started: String::new(),
    };
    on_spawn(&identity)?;
    let pgid = Some(identity.pgid);
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
            signal: exit_signal(&status),
            stdout,
            stderr,
        })
    };
    let result = match tokio::time::timeout(deadline, work).await {
        Ok(r) => r,
        Err(_) => Err(Error::TimedOut),
    };
    if result.is_err() {
        #[cfg(unix)]
        if let Some(pgid) = pgid {
            let _ = signal_group(pgid, libc::SIGTERM);
            tokio::time::sleep(Duration::from_millis(100)).await;
            if group_exists(pgid) {
                let _ = signal_group(pgid, libc::SIGKILL);
            }
        }
        #[cfg(not(unix))]
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    result
}
