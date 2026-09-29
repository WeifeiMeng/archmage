use crate::{
    error::{Error, Result},
    process::{ProcessIdentity, Stream, capture, capture_observed, capture_observed_with_identity},
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

pub const REQUIRED_FLAGS: &[&str] = &[
    "--cd",
    "--sandbox",
    "--json",
    "--output-schema",
    "--output-last-message",
];

pub fn validate_help(help: &str) -> Result<()> {
    let missing: Vec<_> = REQUIRED_FLAGS
        .iter()
        .filter(|flag| !help.split_whitespace().any(|word| word == **flag))
        .copied()
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(Error::Preflight(format!(
            "Codex missing required flags: {}",
            missing.join(", ")
        )))
    }
}

#[derive(Debug, Serialize)]
pub struct LocalTools {
    pub codex_binary: PathBuf,
    pub codex_version: String,
    pub git_binary: PathBuf,
    pub git_version: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sandbox {
    ReadOnly,
    WorkspaceWrite,
}

impl Sandbox {
    fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::WorkspaceWrite => "workspace-write",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CodexInvocation {
    pub repository: PathBuf,
    pub schema_path: PathBuf,
    pub result_path: PathBuf,
    pub diagnostics_dir: PathBuf,
    pub sandbox: Sandbox,
    pub model: Option<String>,
    use_builtin_sandbox: bool,
    pub deadline: Duration,
    pub max_line_bytes: usize,
    pub max_stream_bytes: usize,
    pub max_result_bytes: usize,
    pub extra_args: Vec<OsString>,
}

impl CodexInvocation {
    pub fn new(
        repository: PathBuf,
        schema_path: PathBuf,
        result_path: PathBuf,
        diagnostics_dir: PathBuf,
        sandbox: Sandbox,
    ) -> Self {
        Self {
            repository,
            schema_path,
            result_path,
            diagnostics_dir,
            sandbox,
            model: None,
            use_builtin_sandbox: true,
            deadline: Duration::from_secs(30 * 60),
            max_line_bytes: 8 * 1024 * 1024,
            max_stream_bytes: 128 * 1024 * 1024,
            max_result_bytes: 8 * 1024 * 1024,
            extra_args: Vec::new(),
        }
    }

    pub fn argv(&self) -> Vec<OsString> {
        let mut args = vec![
            "exec".into(),
            "--cd".into(),
            self.repository.as_os_str().into(),
        ];
        if let Some(model) = &self.model {
            args.extend(["--model".into(), model.into()]);
        }
        if self.use_builtin_sandbox {
            args.extend(["--sandbox".into(), self.sandbox.as_str().into()]);
        }
        args.extend(self.extra_args.iter().cloned());
        args.extend([
            "--json".into(),
            "--output-schema".into(),
            self.schema_path.as_os_str().into(),
            "--output-last-message".into(),
            self.result_path.as_os_str().into(),
            "-".into(),
        ]);
        args
    }

    pub fn use_custom_permissions(&mut self) {
        self.use_builtin_sandbox = false;
    }
}

#[derive(Debug, Serialize)]
pub struct CodexOutcome {
    pub thread_id: String,
    pub exit_code: i32,
    pub result: serde_json::Value,
    pub events_path: PathBuf,
    pub stderr_path: PathBuf,
}

pub async fn invoke(
    tools: &LocalTools,
    invocation: &CodexInvocation,
    prompt: &[u8],
) -> Result<CodexOutcome> {
    invoke_with_identity(tools, invocation, prompt, |_| Ok(())).await
}

pub async fn invoke_with_identity(
    tools: &LocalTools,
    invocation: &CodexInvocation,
    prompt: &[u8],
    on_spawn: impl FnOnce(&ProcessIdentity) -> Result<()> + Send,
) -> Result<CodexOutcome> {
    if !invocation.repository.is_dir() || !invocation.schema_path.is_file() {
        return Err(Error::Preflight(
            "Codex invocation repository and schema must exist".into(),
        ));
    }
    private_directory(&invocation.diagnostics_dir)?;
    let events_path = invocation.diagnostics_dir.join("events.jsonl");
    let stderr_path = invocation.diagnostics_dir.join("stderr.log");
    let metadata_path = invocation.diagnostics_dir.join("metadata.json");
    let mut event_log = private_file(&events_path)?;
    let mut stderr_log = private_file(&stderr_path)?;
    let mut decoder = super::codex_event::Decoder::new(invocation.max_line_bytes);
    let mut events = Vec::new();
    let args = invocation.argv();
    let captured = capture_observed_with_identity(
        &tools.codex_binary,
        &args,
        Some(&invocation.repository),
        prompt,
        invocation.deadline,
        invocation.max_stream_bytes,
        on_spawn,
        |stream, bytes| {
            match stream {
                Stream::Stdout => {
                    event_log.write_all(bytes)?;
                    events.extend(decoder.push(bytes)?);
                }
                Stream::Stderr => stderr_log.write_all(bytes)?,
            }
            Ok(())
        },
    )
    .await;
    event_log.sync_all()?;
    stderr_log.sync_all()?;
    let metadata = serde_json::json!({
        "codex_version": tools.codex_version,
        "argv": args.iter().map(|part| part.to_string_lossy()).collect::<Vec<_>>(),
        "exit_code": captured.as_ref().ok().and_then(|value| value.code),
        "capture_error": captured.as_ref().err().map(ToString::to_string),
        "sandbox": if invocation.use_builtin_sandbox { invocation.sandbox.as_str() } else { "custom-permissions" },
    });
    let mut metadata_file = private_file(&metadata_path)?;
    metadata_file.write_all(metadata.to_string().as_bytes())?;
    metadata_file.sync_all()?;
    let captured = captured?;
    if captured.code != Some(0) {
        return Err(Error::Preflight(format!(
            "Codex invocation failed with exit {:?}; inspect diagnostics at {}",
            captured.code,
            invocation.diagnostics_dir.display()
        )));
    }
    events.extend(decoder.finish()?);
    if events
        .iter()
        .any(|event| matches!(event["type"].as_str(), Some("turn.failed" | "error")))
        || !events.iter().any(|event| event["type"] == "turn.completed")
    {
        return Err(Error::Preflight(
            "Codex invocation did not complete successfully".into(),
        ));
    }
    let thread_id = events
        .iter()
        .find(|event| event["type"] == "thread.started")
        .and_then(|event| event["thread_id"].as_str())
        .filter(|id| !id.is_empty())
        .ok_or_else(|| Error::Preflight("missing Codex thread ID".into()))?
        .to_owned();
    let mut bytes = Vec::new();
    fs::File::open(&invocation.result_path)?
        .take(invocation.max_result_bytes as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > invocation.max_result_bytes {
        return Err(Error::Limit("Codex result limit exceeded".into()));
    }
    let result = serde_json::from_slice(&bytes)
        .map_err(|_| Error::Preflight("invalid Codex result JSON".into()))?;
    Ok(CodexOutcome {
        thread_id,
        exit_code: 0,
        result,
        events_path,
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

pub fn find_executable(name: &str) -> Result<PathBuf> {
    let path = std::env::var_os("PATH").ok_or_else(|| Error::Preflight("PATH not set".into()))?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if std::fs::metadata(&candidate)?.permissions().mode() & 0o111 == 0 {
                    continue;
                }
            }
            // Keep executable symlinks: some launchers depend on argv[0].
            return Ok(std::path::absolute(candidate)?);
        }
    }
    Err(Error::Preflight(format!(
        "{name} executable not found on PATH"
    )))
}

async fn text_command(binary: &Path, args: &[&str]) -> Result<String> {
    let args: Vec<OsString> = args.iter().map(Into::into).collect();
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
            "tool inspection returned non-zero status".into(),
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|_| Error::Preflight("tool inspection was not UTF-8".into()))
}

pub async fn inspect() -> Result<LocalTools> {
    inspect_at(None).await
}

pub async fn inspect_at(codex_binary: Option<&Path>) -> Result<LocalTools> {
    let codex_binary = match codex_binary {
        Some(path) => {
            if !path.is_file() {
                return Err(Error::Preflight(format!(
                    "configured Codex executable does not exist: {}",
                    path.display()
                )));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if std::fs::metadata(path)?.permissions().mode() & 0o111 == 0 {
                    return Err(Error::Preflight(format!(
                        "configured Codex path is not executable: {}",
                        path.display()
                    )));
                }
            }
            std::path::absolute(path)?
        }
        None => find_executable("codex")?,
    };
    let git_binary = find_executable("git")?;
    let codex_version = text_command(&codex_binary, &["--version"])
        .await?
        .trim()
        .to_owned();
    validate_help(&text_command(&codex_binary, &["exec", "--help"]).await?)?;
    let git_version = text_command(&git_binary, &["--version"])
        .await?
        .trim()
        .to_owned();
    Ok(LocalTools {
        codex_binary,
        codex_version,
        git_binary,
        git_version,
    })
}

#[derive(Debug, Serialize)]
pub struct ProbeResult {
    pub thread_id: String,
    pub exit_code: i32,
    pub result: serde_json::Value,
    pub diagnostics_dir: Option<PathBuf>,
}

/// Bounded, read-only connectivity check; does not execute a product workflow.
pub async fn readonly_probe(tools: &LocalTools) -> Result<ProbeResult> {
    let temp = tempfile::tempdir()?;
    probe_in(tools, temp.path(), None, None).await
}

/// Retain each probe in a fresh private directory, including failure diagnostics.
pub async fn readonly_probe_logged(tools: &LocalTools, root: &Path) -> Result<ProbeResult> {
    readonly_probe_logged_with_model(tools, root, None).await
}

pub async fn readonly_probe_logged_with_model(
    tools: &LocalTools,
    root: &Path,
    model: Option<&str>,
) -> Result<ProbeResult> {
    let dir = tempfile::Builder::new()
        .prefix("probe-")
        .tempdir_in(root)?
        .keep();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    }
    let result = probe_in(tools, &dir, Some(dir.clone()), model).await;
    if let Err(error) = &result {
        let message = format!("{error}\nDiagnostics: {}", dir.display());
        return Err(Error::Preflight(message));
    }
    result
}

async fn probe_in(
    tools: &LocalTools,
    dir: &Path,
    diagnostics_dir: Option<PathBuf>,
    model: Option<&str>,
) -> Result<ProbeResult> {
    use super::codex_event::Decoder;
    use std::io::Write;
    fn private_file(path: &Path) -> Result<std::fs::File> {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        Ok(options.open(path)?)
    }
    let repo = dir.join("repo");
    std::fs::create_dir(&repo)?;
    let git = capture(
        &tools.git_binary,
        &["init".into(), "--quiet".into(), repo.as_os_str().into()],
        None,
        b"",
        Duration::from_secs(10),
        1024 * 1024,
    )
    .await?;
    if git.code != Some(0) {
        return Err(Error::Preflight("probe Git init failed".into()));
    }
    let schema = dir.join("schema.json");
    let output = dir.join("result.json");
    private_file(&output)?;
    let mut event_log = private_file(&dir.join("events.jsonl"))?;
    let mut stderr_log = private_file(&dir.join("stderr.log"))?;
    std::fs::write(&schema, br#"{"type":"object","properties":{"ok":{"type":"boolean"}},"required":["ok"],"additionalProperties":false}"#)?;
    let mut args: Vec<OsString> = vec!["exec".into(), "--cd".into(), repo.as_os_str().into()];
    if let Some(model) = model {
        args.extend(["--model".into(), model.into()]);
    }
    args.extend([
        "--sandbox".into(),
        "read-only".into(),
        "--json".into(),
        "--output-schema".into(),
        schema.as_os_str().into(),
        "--output-last-message".into(),
        output.as_os_str().into(),
        "-".into(),
    ]);
    let mut decoder = Decoder::new(8 * 1024 * 1024);
    let mut events = Vec::new();
    let mut metadata = private_file(&dir.join("metadata.json"))?;
    let captured = capture_observed(
        &tools.codex_binary,
        &args,
        Some(&repo),
        b"Return the JSON object {\"ok\":true}. Do not edit files or run commands.",
        Duration::from_secs(120),
        8 * 1024 * 1024,
        |stream, bytes| {
            match stream {
                Stream::Stdout => {
                    event_log.write_all(bytes)?;
                    events.extend(decoder.push(bytes)?);
                }
                Stream::Stderr => stderr_log.write_all(bytes)?,
            }
            Ok(())
        },
    )
    .await;
    event_log.sync_all()?;
    stderr_log.sync_all()?;
    let record = serde_json::json!({
        "codex_version": tools.codex_version,
        "argv": args.iter().map(|a| a.to_string_lossy()).collect::<Vec<_>>(),
        "exit_code": captured.as_ref().ok().and_then(|c| c.code),
        "capture_error": captured.as_ref().err().map(ToString::to_string),
        "sandbox": "read-only",
    });
    metadata.write_all(record.to_string().as_bytes())?;
    metadata.sync_all()?;
    let captured = captured?;
    if captured.code != Some(0) {
        // Do not print untrusted stderr (which can contain credentials/configuration).
        return Err(Error::Preflight(format!(
            "Codex probe failed with exit {:?}; verify local Codex login/configuration",
            captured.code
        )));
    }
    events.extend(decoder.finish()?);
    if events
        .iter()
        .any(|e| matches!(e["type"].as_str(), Some("turn.failed" | "error")))
    {
        return Err(Error::Preflight(
            "Codex probe emitted a failure event".into(),
        ));
    }
    if !events.iter().any(|e| e["type"] == "turn.completed") {
        return Err(Error::Preflight(
            "Codex probe did not complete a turn".into(),
        ));
    }
    let thread_id = events
        .iter()
        .find(|e| e["type"] == "thread.started")
        .and_then(|e| e["thread_id"].as_str())
        .filter(|id| !id.is_empty())
        .ok_or_else(|| Error::Preflight("missing Codex thread ID".into()))?
        .to_owned();
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(output)?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(Error::Limit("probe result too large".into()));
    }
    let result: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| Error::Preflight("invalid probe result JSON".into()))?;
    if result != serde_json::json!({"ok": true}) {
        return Err(Error::Preflight(
            "probe result does not satisfy contract".into(),
        ));
    }
    Ok(ProbeResult {
        thread_id,
        exit_code: 0,
        result,
        diagnostics_dir,
    })
}
