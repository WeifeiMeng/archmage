use crate::{
    artifact::ArtifactStore,
    error::{Error, Result},
    process::capture_clean,
    store::{Store, transitions},
    workspace::{GitRepository, snapshot},
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
#[cfg(unix)]
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixListener,
};

#[derive(Clone, Debug)]
pub struct FrozenCheck {
    pub check_id: String,
    pub command: String,
    pub timeout: Duration,
}
impl FrozenCheck {
    pub fn new(id: impl Into<String>, command: impl Into<String>, timeout: Duration) -> Self {
        Self {
            check_id: id.into(),
            command: command.into(),
            timeout,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckEvidence {
    pub evidence_id: String,
    pub attempt_id: String,
    pub check_id: String,
    pub command: String,
    pub cwd: String,
    pub timeout_ms: u128,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub timed_out: bool,
    pub started_ms: u128,
    pub ended_ms: u128,
    pub duration_ms: u128,
    pub stdout_hash: String,
    pub stderr_hash: String,
    pub stdout_artifact: String,
    pub stderr_artifact: String,
    pub workspace_before: String,
    pub workspace_after: String,
}

pub struct CheckRunner<'a> {
    store: &'a mut Store,
    artifacts: &'a ArtifactStore,
    attempt: &'a str,
    generation: i64,
    repo: &'a GitRepository,
    checks: BTreeMap<String, FrozenCheck>,
    codex: PathBuf,
}

impl<'a> CheckRunner<'a> {
    pub fn new(
        store: &'a mut Store,
        artifacts: &'a ArtifactStore,
        attempt: &'a str,
        generation: i64,
        repo: &'a GitRepository,
        checks: Vec<FrozenCheck>,
    ) -> Result<Self> {
        let mut frozen = BTreeMap::new();
        for check in checks {
            if check.check_id.trim().is_empty()
                || check.command.trim().is_empty()
                || frozen.insert(check.check_id.clone(), check).is_some()
            {
                return Err(Error::Preflight(
                    "check IDs and commands must be nonblank and unique".into(),
                ));
            }
        }
        let codex = find_codex()?;
        Ok(Self {
            store,
            artifacts,
            attempt,
            generation,
            repo,
            checks: frozen,
            codex,
        })
    }

    pub async fn run(&mut self, check_id: &str) -> Result<CheckEvidence> {
        self.ensure_active()?;
        let check = self
            .checks
            .get(check_id)
            .ok_or_else(|| Error::Preflight("unknown frozen check ID".into()))?
            .clone();
        if let Some(existing) = self.existing(check_id)? {
            return Ok(existing);
        }
        let evidence_id = uuid::Uuid::new_v4().to_string();
        let tx = self
            .store
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted = tx.execute(
            "INSERT INTO check_runs(evidence_id,attempt_id,check_id,command,state)
             SELECT ?1,?2,?3,?4,'in_progress'
             WHERE EXISTS(SELECT 1 FROM attempts a JOIN node_runs n ON n.id=a.node_run_id
                          JOIN runs r ON r.id=n.run_id WHERE a.id=?2 AND a.state='running'
                          AND a.owner_generation=?5 AND r.owner_generation=?5 AND r.state='running')",
            params![evidence_id, self.attempt, check_id, check.command, self.generation],
        )?;
        if inserted != 1 {
            return Err(Error::Conflict);
        }
        let (node, run): (String, String) = tx.query_row(
            "SELECT n.id,n.run_id FROM attempts a JOIN node_runs n ON n.id=a.node_run_id WHERE a.id=?1",
            [self.attempt],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        transitions::event(
            &tx,
            &run,
            "check.started",
            Some(&node),
            Some(self.attempt),
            serde_json::json!({
                "evidence_id": evidence_id,
                "check_id": check_id,
                "shell": "/bin/sh",
                "command": check.command,
                "cwd": self.repo.root,
                "timeout_ms": check.timeout.as_millis()
            }),
        )?;
        tx.commit()?;
        let before = snapshot(self.repo)?.id;
        let started_ms = now_ms()?;
        let config = permission_config(&self.repo.root)?;
        let args = vec![
            "sandbox".into(),
            "-c".into(),
            config.into(),
            "-P".into(),
            "archmage".into(),
            "-C".into(),
            self.repo.root.as_os_str().to_owned(),
            "--".into(),
            "/bin/sh".into(),
            "-c".into(),
            check.command.clone().into(),
        ];
        let environment = allowed_environment();
        let captured = capture_clean(
            &self.codex,
            &args,
            &self.repo.root,
            check.timeout,
            128 * 1024 * 1024,
            &environment,
        )
        .await;
        let ended_ms = now_ms()?;
        let (exit_code, signal, timed_out, stdout, stderr) = match captured {
            Ok(value) => (value.code, value.signal, false, value.stdout, value.stderr),
            Err(Error::TimedOut) => (None, None, true, Vec::new(), Vec::new()),
            Err(error) => return Err(error),
        };
        let after = snapshot(self.repo)?.id;
        let stdout_hash = hash(&stdout);
        let stderr_hash = hash(&stderr);
        let stdout_artifact =
            self.artifacts
                .publish(self.store, self.attempt, &stdout, "text/plain")?;
        let stderr_artifact =
            self.artifacts
                .publish(self.store, self.attempt, &stderr, "text/plain")?;
        let evidence = CheckEvidence {
            evidence_id: evidence_id.clone(),
            attempt_id: self.attempt.into(),
            check_id: check_id.into(),
            command: check.command,
            cwd: self.repo.root.to_string_lossy().into_owned(),
            timeout_ms: check.timeout.as_millis(),
            exit_code,
            signal,
            timed_out,
            started_ms,
            ended_ms,
            duration_ms: ended_ms.saturating_sub(started_ms),
            stdout_hash,
            stderr_hash,
            stdout_artifact,
            stderr_artifact,
            workspace_before: before,
            workspace_after: after,
        };
        let json = serde_json::to_string(&evidence).map_err(|e| Error::Preflight(e.to_string()))?;
        let tx = self
            .store
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx.execute(
            "UPDATE check_runs SET state='completed',evidence_json=?2
             WHERE evidence_id=?1 AND state='in_progress'
             AND EXISTS(SELECT 1 FROM attempts a JOIN node_runs n ON n.id=a.node_run_id
                        JOIN runs r ON r.id=n.run_id WHERE a.id=check_runs.attempt_id
                        AND a.state='running' AND a.owner_generation=?3
                        AND r.owner_generation=?3 AND r.state='running')",
            params![evidence_id, json, self.generation],
        )?;
        if changed != 1 {
            return Err(Error::Conflict);
        }
        let (node,run):(String,String)=tx.query_row("SELECT n.id,n.run_id FROM attempts a JOIN node_runs n ON n.id=a.node_run_id WHERE a.id=?1",[self.attempt],|r|Ok((r.get(0)?,r.get(1)?)))?;
        transitions::event(
            &tx,
            &run,
            "check.completed",
            Some(&node),
            Some(self.attempt),
            serde_json::json!({"evidence_id":evidence_id,"check_id":check_id,"exit_code":exit_code,"signal":signal,"timed_out":timed_out}),
        )?;
        tx.commit()?;
        Ok(evidence)
    }

    fn ensure_active(&self) -> Result<()> {
        let valid: bool = self.store.db.query_row("SELECT EXISTS(SELECT 1 FROM attempts a JOIN node_runs n ON n.id=a.node_run_id JOIN runs r ON r.id=n.run_id WHERE a.id=?1 AND a.state='running' AND a.owner_generation=?2 AND r.owner_generation=?2 AND r.state='running')",params![self.attempt,self.generation],|r|r.get(0))?;
        if valid { Ok(()) } else { Err(Error::Conflict) }
    }
    fn existing(&self, check_id: &str) -> Result<Option<CheckEvidence>> {
        let row: Option<(String, Option<String>)> = self
            .store
            .db
            .query_row(
                "SELECT state,evidence_json FROM check_runs WHERE attempt_id=?1 AND check_id=?2",
                params![self.attempt, check_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        match row {
            None => Ok(None),
            Some((state, Some(json))) if state == "completed" => Ok(Some(
                serde_json::from_str(&json).map_err(|e| Error::Preflight(e.to_string()))?,
            )),
            Some(_) => Err(Error::Conflict),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct CheckRequest {
    attempt_id: String,
    check_id: String,
}
#[derive(Serialize, Deserialize)]
struct CheckResponse {
    evidence: Option<CheckEvidence>,
    error: Option<String>,
}

#[cfg(unix)]
pub async fn serve_check_once(listener: &UnixListener, runner: &mut CheckRunner<'_>) -> Result<()> {
    let (mut stream, _) = listener.accept().await?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).await?;
    let request: CheckRequest =
        serde_json::from_slice(&bytes).map_err(|e| Error::Preflight(e.to_string()))?;
    let response = if request.attempt_id != runner.attempt {
        CheckResponse {
            evidence: None,
            error: Some("attempt ownership mismatch".into()),
        }
    } else {
        match runner.run(&request.check_id).await {
            Ok(evidence) => CheckResponse {
                evidence: Some(evidence),
                error: None,
            },
            Err(error) => CheckResponse {
                evidence: None,
                error: Some(error.to_string()),
            },
        }
    };
    stream
        .write_all(&serde_json::to_vec(&response).map_err(|e| Error::Preflight(e.to_string()))?)
        .await?;
    stream.shutdown().await?;
    Ok(())
}

#[cfg(unix)]
pub async fn request_check(endpoint: &Path, attempt: &str, check: &str) -> Result<CheckEvidence> {
    let mut stream = tokio::net::UnixStream::connect(endpoint).await?;
    let request = CheckRequest {
        attempt_id: attempt.into(),
        check_id: check.into(),
    };
    stream
        .write_all(&serde_json::to_vec(&request).map_err(|e| Error::Preflight(e.to_string()))?)
        .await?;
    stream.shutdown().await?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).await?;
    let response: CheckResponse =
        serde_json::from_slice(&bytes).map_err(|e| Error::Preflight(e.to_string()))?;
    response.evidence.ok_or_else(|| {
        Error::Preflight(
            response
                .error
                .unwrap_or_else(|| "empty check response".into()),
        )
    })
}

pub fn load_evidence(store: &Store, attempt: &str, check: &str) -> Result<CheckEvidence> {
    let json: String = store.db.query_row(
        "SELECT evidence_json FROM check_runs WHERE attempt_id=?1 AND check_id=?2 AND state='completed'",
        params![attempt, check],
        |r| r.get(0),
    )?;
    serde_json::from_str(&json).map_err(|e| Error::Preflight(e.to_string()))
}

fn find_codex() -> Result<PathBuf> {
    let output = std::process::Command::new("/usr/bin/which")
        .arg("codex")
        .output()?;
    if !output.status.success() {
        return Err(Error::Preflight("codex executable not found".into()));
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim(),
    ))
}
fn permission_config(worktree: &Path) -> Result<String> {
    let root = serde_json::to_string(&worktree.to_string_lossy())
        .map_err(|e| Error::Preflight(e.to_string()))?;
    let codex_home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".codex")));
    let deny = codex_home
        .map(|p| {
            format!(
                ",{}=\"deny\"",
                serde_json::to_string(&p.to_string_lossy()).unwrap()
            )
        })
        .unwrap_or_default();
    Ok(format!(
        "permissions.archmage.filesystem={{\"/\"=\"read\"{deny},{root}=\"write\"}}"
    ))
}
fn allowed_environment() -> Vec<(OsString, OsString)> {
    ["PATH", "HOME", "LANG", "LC_ALL", "TMPDIR"]
        .into_iter()
        .filter_map(|key| std::env::var_os(key).map(|value| (key.into(), value)))
        .collect()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn now_ms() -> Result<u128> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| Error::Preflight(e.to_string()))?
        .as_millis())
}
