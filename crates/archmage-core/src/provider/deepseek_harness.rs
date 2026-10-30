use crate::{
    error::{Error, Result},
    process::{Stream, capture, capture_observed_with_identity_env},
    provider::codex::{Sandbox, find_executable},
};
use serde::Serialize;
use std::{
    ffi::OsString,
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

pub const REQUIRED_FLAGS: &[&str] = &["--session-id", "--resume-session-id"];

pub fn validate_help(help: &str) -> Result<()> {
    let missing = REQUIRED_FLAGS
        .iter()
        .filter(|flag| !help.split_whitespace().any(|word| word == **flag))
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(Error::Preflight(format!(
            "DeepSeek Harness missing required flags: {}",
            missing.join(", ")
        )))
    }
}

#[derive(Debug, Serialize)]
pub struct LocalTools {
    pub dsh_binary: PathBuf,
    pub dsh_version: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HarnessSession {
    Fresh,
    Create(String),
    Resume(String),
}

impl HarnessSession {
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Fresh => None,
            Self::Create(id) | Self::Resume(id) => Some(id),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DeepseekHarnessInvocation {
    pub repository: PathBuf,
    pub diagnostics_dir: PathBuf,
    pub sandbox: Sandbox,
    pub session: HarnessSession,
    pub deadline: Duration,
    pub max_stream_bytes: usize,
    pub max_result_bytes: usize,
}

impl DeepseekHarnessInvocation {
    pub fn new(
        repository: PathBuf,
        diagnostics_dir: PathBuf,
        sandbox: Sandbox,
        session: HarnessSession,
    ) -> Self {
        Self {
            repository,
            diagnostics_dir,
            sandbox,
            session,
            deadline: Duration::from_secs(30 * 60),
            max_stream_bytes: 128 * 1024 * 1024,
            max_result_bytes: 8 * 1024 * 1024,
        }
    }

    pub fn argv(&self, task: &str) -> Vec<OsString> {
        let mut args = vec!["--profile".into(), "headless".into()];
        match &self.session {
            HarnessSession::Fresh => {}
            HarnessSession::Create(id) => args.extend(["--session-id".into(), id.into()]),
            HarnessSession::Resume(id) => {
                args.extend(["--resume-session-id".into(), id.into()]);
            }
        }
        args.push(task.into());
        args
    }
}

#[derive(Debug)]
pub struct DeepseekHarnessOutcome {
    pub session_id: Option<String>,
    pub result: serde_json::Value,
    pub stdout_path: PathBuf,
    pub stderr_path: PathBuf,
}

pub fn render_task(context: &[u8], schema: &str) -> Result<String> {
    let context = std::str::from_utf8(context)
        .map_err(|_| Error::Preflight("DeepSeek Harness context is not UTF-8".into()))?;
    serde_json::from_str::<serde_json::Value>(schema)
        .map_err(|error| Error::Preflight(format!("invalid output schema: {error}")))?;
    Ok(format!(
        "{context}\n\nOutput requirements:\nReturn exactly one single JSON object matching this JSON Schema:\n{schema}\nDo not use Markdown or code fences. Do not add prose before or after the JSON object."
    ))
}

pub fn parse_result(bytes: &[u8], max: usize) -> Result<serde_json::Value> {
    if bytes.len() > max {
        return Err(Error::Limit(
            "DeepSeek Harness result limit exceeded".into(),
        ));
    }
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| Error::Preflight("invalid DeepSeek Harness result JSON".into()))?;
    if !value.is_object() {
        return Err(Error::Preflight(
            "DeepSeek Harness result must be one JSON object".into(),
        ));
    }
    Ok(value)
}

async fn text_command(binary: &Path, args: &[&str]) -> Result<String> {
    let args = args.iter().map(OsString::from).collect::<Vec<_>>();
    let output = capture(
        binary,
        &args,
        None,
        b"",
        Duration::from_secs(10),
        1024 * 1024,
    )
    .await?;
    if output.code != Some(0) {
        return Err(Error::Preflight(
            "DeepSeek Harness inspection returned non-zero status".into(),
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|_| Error::Preflight("DeepSeek Harness inspection was not UTF-8".into()))
}

pub async fn inspect_at(dsh_binary: Option<&Path>) -> Result<LocalTools> {
    let dsh_binary = match dsh_binary {
        Some(path) => {
            if !path.is_file() {
                return Err(Error::Preflight(format!(
                    "configured dsh executable does not exist: {}",
                    path.display()
                )));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if fs::metadata(path)?.permissions().mode() & 0o111 == 0 {
                    return Err(Error::Preflight(format!(
                        "configured dsh path is not executable: {}",
                        path.display()
                    )));
                }
            }
            std::path::absolute(path)?
        }
        None => find_executable("dsh")?,
    };
    let dsh_version = text_command(&dsh_binary, &["--version"])
        .await?
        .trim()
        .to_owned();
    let help = text_command(&dsh_binary, &["--profile", "headless", "--help"]).await?;
    validate_help(&help)?;
    Ok(LocalTools {
        dsh_binary,
        dsh_version,
    })
}

pub async fn invoke_with_identity(
    tools: &LocalTools,
    invocation: &DeepseekHarnessInvocation,
    context: &[u8],
    schema: &str,
    on_spawn: impl FnOnce(&crate::process::ProcessIdentity) -> Result<()> + Send,
    on_persisted: impl FnOnce() -> Result<()> + Send,
) -> Result<DeepseekHarnessOutcome> {
    if !invocation.repository.is_dir() {
        return Err(Error::Preflight(
            "DeepSeek Harness repository must exist".into(),
        ));
    }
    private_directory(&invocation.diagnostics_dir)?;
    let stdout_path = invocation.diagnostics_dir.join("stdout.log");
    let stderr_path = invocation.diagnostics_dir.join("stderr.log");
    let metadata_path = invocation.diagnostics_dir.join("metadata.json");
    let mut stdout_log = private_file(&stdout_path)?;
    let mut stderr_log = private_file(&stderr_path)?;
    let task = render_task(context, schema)?;
    let args = invocation.argv(&task);
    let permission = match invocation.sandbox {
        Sandbox::ReadOnly => "read-only",
        Sandbox::WorkspaceWrite => "workspace-write",
    };
    let environment = vec![("DSH_PERMISSION_MODE".into(), permission.into())];
    let captured = capture_observed_with_identity_env(
        &tools.dsh_binary,
        &args,
        Some(&invocation.repository),
        b"",
        invocation.deadline,
        invocation.max_stream_bytes,
        &environment,
        on_spawn,
        |stream, bytes| {
            match stream {
                Stream::Stdout => stdout_log.write_all(bytes)?,
                Stream::Stderr => stderr_log.write_all(bytes)?,
            }
            Ok(())
        },
    )
    .await;
    stdout_log.sync_all()?;
    stderr_log.sync_all()?;
    let metadata = serde_json::json!({
        "dsh_version": tools.dsh_version,
        "argv": args.iter().take(args.len().saturating_sub(1)).map(|part| part.to_string_lossy()).collect::<Vec<_>>(),
        "exit_code": captured.as_ref().ok().and_then(|value| value.code),
        "capture_error": captured.as_ref().err().map(ToString::to_string),
        "permission": permission,
        "session_id": invocation.session.id(),
    });
    let mut metadata_file = private_file(&metadata_path)?;
    metadata_file.write_all(metadata.to_string().as_bytes())?;
    metadata_file.sync_all()?;
    let captured = captured?;
    if captured.code != Some(0) {
        return Err(Error::Preflight(format!(
            "DeepSeek Harness invocation failed with exit {:?}; inspect diagnostics at {}",
            captured.code,
            invocation.diagnostics_dir.display()
        )));
    }
    on_persisted()?;
    let result = parse_result(&captured.stdout, invocation.max_result_bytes)?;
    Ok(DeepseekHarnessOutcome {
        session_id: invocation.session.id().map(ToOwned::to_owned),
        result,
        stdout_path,
        stderr_path,
    })
}

fn private_directory(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    Ok(())
}

fn private_file(path: &Path) -> Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    Ok(options.open(path)?)
}
