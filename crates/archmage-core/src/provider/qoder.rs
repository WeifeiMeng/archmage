use crate::{
    error::{Error, Result},
    process::{OwnedPipedProcess, capture, capture_observed_with_identity_env, spawn_owned_piped},
    provider::{
        acp::{AcpAccess, AcpWorkspacePolicy},
        codex::find_executable,
        executor::ProviderOutcome,
    },
};
use agent_client_protocol::v1::schema::{
    CreateTerminalRequest, CreateTerminalResponse, KillTerminalRequest, KillTerminalResponse,
    PermissionOptionKind, ReadTextFileRequest, ReadTextFileResponse, ReleaseTerminalRequest,
    ReleaseTerminalResponse, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, TerminalExitStatus,
    TerminalOutputRequest, TerminalOutputResponse, WaitForTerminalExitRequest,
    WaitForTerminalExitResponse, WriteTextFileRequest, WriteTextFileResponse,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

const ACP_PROTOCOL_VERSION: u64 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QoderInspection {
    pub binary: PathBuf,
    pub version: String,
}

#[derive(Clone, Debug)]
pub struct QoderSessionConfig {
    pub binary: PathBuf,
    pub repository: PathBuf,
    pub model: Option<String>,
    pub agent_name: Option<String>,
    pub provider_config: Value,
    pub access: AcpAccess,
    pub diagnostics_dir: PathBuf,
    pub deadline: Duration,
    pub max_line_bytes: usize,
    pub max_stream_bytes: usize,
    pub max_result_bytes: usize,
    pub environment: BTreeMap<OsString, OsString>,
}

pub struct QoderSession {
    process: OwnedPipedProcess,
    config: QoderSessionConfig,
    session_id: String,
    next_id: u64,
    stream_bytes: usize,
    policy: AcpWorkspacePolicy,
    stderr_path: PathBuf,
    terminals: BTreeMap<String, TerminalRecord>,
    next_terminal: u64,
}

struct TerminalRecord {
    output: String,
    truncated: bool,
    exit_status: TerminalExitStatus,
}

pub async fn inspect_at(binary: Option<&Path>) -> Result<QoderInspection> {
    let binary = match binary {
        Some(path) if path.is_file() => std::path::absolute(path)?,
        Some(path) => {
            return Err(Error::Preflight(format!(
                "configured Qoder executable does not exist: {}",
                path.display()
            )));
        }
        None => find_executable("qoder")?,
    };
    let output = capture(
        &binary,
        &[OsString::from("--version")],
        None,
        b"",
        Duration::from_secs(10),
        1024 * 1024,
    )
    .await?;
    if output.code != Some(0) {
        return Err(Error::Preflight("Qoder version inspection failed".into()));
    }
    let version = std::str::from_utf8(&output.stdout)
        .map_err(|_| Error::Preflight("invalid Qoder version output".into()))?
        .trim()
        .to_owned();
    if version.is_empty() {
        return Err(Error::Preflight("Qoder version output is empty".into()));
    }
    Ok(QoderInspection { binary, version })
}

impl QoderSession {
    pub async fn start(config: QoderSessionConfig) -> Result<Self> {
        if !config.repository.is_dir() {
            return Err(Error::Preflight("Qoder repository must exist".into()));
        }
        if !config.provider_config.is_object() {
            return Err(Error::Preflight(
                "Qoder provider configuration must be an object".into(),
            ));
        }
        private_directory(&config.diagnostics_dir)?;
        let policy = AcpWorkspacePolicy::new(&config.repository, config.access)?;
        let mut args = vec![
            OsString::from("--acp"),
            OsString::from("--strict-mcp-config"),
            OsString::from("--setting-sources"),
            OsString::new(),
        ];
        if let Some(model) = &config.model {
            args.extend([OsString::from("--model"), OsString::from(model)]);
        }
        if let Some(agent_name) = &config.agent_name {
            args.extend([OsString::from("--agent"), OsString::from(agent_name)]);
        }
        append_explicit_extensions(&mut args, &config.provider_config)?;
        let environment = config.environment.clone().into_iter().collect::<Vec<_>>();
        let process = spawn_owned_piped(
            &config.binary,
            &args,
            Some(&config.repository),
            &environment,
            config.max_stream_bytes,
        )
        .await?;
        let stderr_path = config.diagnostics_dir.join("stderr.log");
        let mut session = Self {
            process,
            config,
            session_id: String::new(),
            next_id: 0,
            stream_bytes: 0,
            policy,
            stderr_path,
            terminals: BTreeMap::new(),
            next_terminal: 0,
        };
        if let Err(error) = session.initialize().await {
            let _ = session.process.cancel(Duration::from_secs(1)).await;
            return Err(error);
        }
        Ok(session)
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn identity(&self) -> &crate::process::ProcessIdentity {
        self.process.identity()
    }

    async fn initialize(&mut self) -> Result<()> {
        let result = self
            .request(
                "initialize",
                json!({
                    "protocolVersion": ACP_PROTOCOL_VERSION,
                    "clientCapabilities": {
                        "fs": {"readTextFile": true, "writeTextFile": true},
                        "terminal": true
                    },
                    "clientInfo": {"name": "archmage", "version": env!("CARGO_PKG_VERSION")}
                }),
                None,
            )
            .await?;
        if result.get("protocolVersion").and_then(Value::as_u64) != Some(ACP_PROTOCOL_VERSION) {
            return Err(Error::Preflight(
                "Qoder returned an unsupported ACP protocol version".into(),
            ));
        }
        let mcp_servers = self
            .config
            .provider_config
            .get("mcpServers")
            .cloned()
            .unwrap_or_else(|| json!([]));
        if !mcp_servers.is_array() {
            return Err(Error::Preflight(
                "Qoder mcpServers configuration must be an array".into(),
            ));
        }
        let result = self
            .request(
                "session/new",
                json!({"cwd": self.config.repository, "mcpServers": mcp_servers}),
                None,
            )
            .await?;
        self.session_id = result
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| Error::Preflight("Qoder did not return an ACP session ID".into()))?
            .to_owned();
        Ok(())
    }

    pub async fn prompt(&mut self, _attempt_id: &str, prompt: &[u8]) -> Result<ProviderOutcome> {
        let prompt = std::str::from_utf8(prompt)
            .map_err(|_| Error::Preflight("Qoder prompt is not UTF-8".into()))?;
        let mut text = String::new();
        let session_id = self.session_id.clone();
        self.request(
            "session/prompt",
            json!({
                "sessionId": session_id,
                "prompt": [{"type": "text", "text": prompt}]
            }),
            Some(&mut text),
        )
        .await?;
        if text.len() > self.config.max_result_bytes {
            return Err(Error::Limit("Qoder result limit exceeded".into()));
        }
        let result: Value = serde_json::from_str(&text)
            .map_err(|_| Error::Preflight("invalid Qoder result JSON".into()))?;
        if !result.is_object() {
            return Err(Error::Preflight(
                "Qoder result must be one JSON object".into(),
            ));
        }
        Ok(ProviderOutcome {
            conversation_id: self.session_id.clone(),
            result,
        })
    }

    pub async fn cancel(mut self) -> Result<()> {
        let frame = json!({
            "jsonrpc": "2.0",
            "method": "session/cancel",
            "params": {"sessionId": self.session_id}
        });
        let _ = write_frame(&mut self.process, &frame).await;
        self.process.cancel(Duration::from_secs(1)).await
    }

    pub async fn close(mut self) -> Result<()> {
        self.process.shutdown_stdin().await?;
        let (status, stderr) = tokio::time::timeout(Duration::from_secs(5), self.process.wait())
            .await
            .map_err(|_| Error::TimedOut)??;
        write_private(&self.stderr_path, &stderr)?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::Preflight(
                "Qoder ACP process exited unsuccessfully".into(),
            ))
        }
    }

    async fn request(
        &mut self,
        method: &str,
        params: Value,
        mut agent_text: Option<&mut String>,
    ) -> Result<Value> {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        write_frame(
            &mut self.process,
            &json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}),
        )
        .await?;
        loop {
            let frame = self.read_frame().await?;
            if frame.get("id").and_then(Value::as_u64) == Some(id) {
                if let Some(error) = frame.get("error") {
                    return Err(Error::Preflight(format!(
                        "Qoder ACP request failed: {error}"
                    )));
                }
                return frame
                    .get("result")
                    .cloned()
                    .ok_or_else(|| Error::Preflight("Qoder ACP response has no result".into()));
            }
            if frame.get("method").and_then(Value::as_str) == Some("session/update") {
                if let Some(target) = agent_text.as_deref_mut() {
                    append_agent_text(&frame, target, self.config.max_result_bytes)?;
                }
                continue;
            }
            if frame.get("id").is_some() && frame.get("method").is_some() {
                self.handle_client_request(frame).await?;
            }
        }
    }

    async fn read_frame(&mut self) -> Result<Value> {
        let mut line = Vec::new();
        let count = tokio::time::timeout(
            self.config.deadline,
            self.process.stdout().read_until(b'\n', &mut line),
        )
        .await
        .map_err(|_| Error::TimedOut)??;
        if count == 0 {
            return Err(Error::Preflight(
                "Qoder ACP process exited before responding".into(),
            ));
        }
        if line.len() > self.config.max_line_bytes {
            return Err(Error::Limit("Qoder ACP line limit exceeded".into()));
        }
        self.stream_bytes = self.stream_bytes.saturating_add(line.len());
        if self.stream_bytes > self.config.max_stream_bytes {
            return Err(Error::Limit("Qoder ACP stream limit exceeded".into()));
        }
        serde_json::from_slice(&line)
            .map_err(|_| Error::Preflight("invalid Qoder ACP JSON-RPC frame".into()))
    }

    async fn handle_client_request(&mut self, frame: Value) -> Result<()> {
        let id = frame.get("id").cloned().unwrap_or(Value::Null);
        let method = frame.get("method").and_then(Value::as_str).unwrap_or("");
        let params = frame.get("params").cloned().unwrap_or(Value::Null);
        let result = self.client_result(method, params).await;
        let response = match result {
            Ok(result) => json!({"jsonrpc":"2.0", "id":id, "result":result}),
            Err(error) => json!({
                "jsonrpc":"2.0",
                "id":id,
                "error":{"code":-32602, "message":error.to_string()}
            }),
        };
        write_frame(&mut self.process, &response).await
    }

    async fn client_result(&mut self, method: &str, params: Value) -> Result<Value> {
        match method {
            "fs/read_text_file" => {
                let request: ReadTextFileRequest = parse_params(params)?;
                self.require_session(&request.session_id.to_string())?;
                let content = self
                    .policy
                    .read_text(&request.path, request.line, request.limit)
                    .await?;
                to_value(ReadTextFileResponse::new(content))
            }
            "fs/write_text_file" => {
                let request: WriteTextFileRequest = parse_params(params)?;
                self.require_session(&request.session_id.to_string())?;
                self.policy
                    .write_text(&request.path, &request.content)
                    .await?;
                to_value(WriteTextFileResponse::new())
            }
            "session/request_permission" => {
                let request: RequestPermissionRequest = parse_params(params)?;
                self.require_session(&request.session_id.to_string())?;
                let preferred = if self.config.access == AcpAccess::WorkspaceWrite {
                    PermissionOptionKind::AllowOnce
                } else {
                    PermissionOptionKind::RejectOnce
                };
                let option = request
                    .options
                    .iter()
                    .find(|option| option.kind == preferred)
                    .or_else(|| {
                        request.options.iter().find(|option| {
                            matches!(
                                option.kind,
                                PermissionOptionKind::RejectOnce
                                    | PermissionOptionKind::RejectAlways
                            )
                        })
                    });
                let outcome = option.map_or(RequestPermissionOutcome::Cancelled, |option| {
                    RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
                        option.option_id.clone(),
                    ))
                });
                to_value(RequestPermissionResponse::new(outcome))
            }
            "terminal/create" => self.create_terminal(parse_params(params)?).await,
            "terminal/output" => {
                let request: TerminalOutputRequest = parse_params(params)?;
                self.require_session(&request.session_id.to_string())?;
                let record = self.terminal(&request.terminal_id.to_string())?;
                to_value(
                    TerminalOutputResponse::new(record.output.clone(), record.truncated)
                        .exit_status(record.exit_status.clone()),
                )
            }
            "terminal/wait_for_exit" => {
                let request: WaitForTerminalExitRequest = parse_params(params)?;
                self.require_session(&request.session_id.to_string())?;
                let record = self.terminal(&request.terminal_id.to_string())?;
                to_value(WaitForTerminalExitResponse::new(record.exit_status.clone()))
            }
            "terminal/kill" => {
                let request: KillTerminalRequest = parse_params(params)?;
                self.require_session(&request.session_id.to_string())?;
                self.terminal(&request.terminal_id.to_string())?;
                to_value(KillTerminalResponse::new())
            }
            "terminal/release" => {
                let request: ReleaseTerminalRequest = parse_params(params)?;
                self.require_session(&request.session_id.to_string())?;
                if self
                    .terminals
                    .remove(&request.terminal_id.to_string())
                    .is_none()
                {
                    return Err(Error::Preflight("unknown ACP terminal".into()));
                }
                to_value(ReleaseTerminalResponse::new())
            }
            _ => Err(Error::Preflight(format!(
                "unsupported ACP client method: {method}"
            ))),
        }
    }

    async fn create_terminal(&mut self, request: CreateTerminalRequest) -> Result<Value> {
        self.require_session(&request.session_id.to_string())?;
        let cwd = request.cwd.as_deref().unwrap_or(self.policy.root());
        self.policy
            .validate_terminal(cwd, &request.command, &request.args)?;
        let cwd = self.policy.resolve(cwd, false)?;
        let limit = request
            .output_byte_limit
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(self.config.max_stream_bytes)
            .min(self.config.max_stream_bytes);
        let environment = request
            .env
            .into_iter()
            .map(|item| (OsString::from(item.name), OsString::from(item.value)))
            .collect::<Vec<_>>();
        let args = request
            .args
            .into_iter()
            .map(OsString::from)
            .collect::<Vec<_>>();
        let output = capture_observed_with_identity_env(
            Path::new(&request.command),
            &args,
            Some(&cwd),
            b"",
            self.config.deadline,
            self.config.max_stream_bytes,
            &environment,
            |_| Ok(()),
            |_, _| Ok(()),
        )
        .await?;
        let mut bytes = output.stdout;
        bytes.extend_from_slice(&output.stderr);
        let (bytes, truncated) = retain_utf8_tail(bytes, limit);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let exit_status = TerminalExitStatus::new()
            .exit_code(output.code.and_then(|code| u32::try_from(code).ok()))
            .signal(output.signal.map(|signal| signal.to_string()));
        self.next_terminal = self.next_terminal.saturating_add(1);
        let terminal_id = format!("archmage-{}", self.next_terminal);
        self.terminals.insert(
            terminal_id.clone(),
            TerminalRecord {
                output: text,
                truncated,
                exit_status,
            },
        );
        to_value(CreateTerminalResponse::new(terminal_id))
    }

    fn require_session(&self, session_id: &str) -> Result<()> {
        if session_id == self.session_id {
            Ok(())
        } else {
            Err(Error::Preflight("ACP session ID mismatch".into()))
        }
    }

    fn terminal(&self, terminal_id: &str) -> Result<&TerminalRecord> {
        self.terminals
            .get(terminal_id)
            .ok_or_else(|| Error::Preflight("unknown ACP terminal".into()))
    }
}

fn append_explicit_extensions(args: &mut Vec<OsString>, config: &Value) -> Result<()> {
    let object = config
        .as_object()
        .ok_or_else(|| Error::Preflight("Qoder provider configuration must be an object".into()))?;
    for (key, flag) in [
        ("pluginDirs", "--plugin-dir"),
        ("allowedTools", "--allowed-tools"),
        ("disallowedTools", "--disallowed-tools"),
    ] {
        if let Some(values) = object.get(key) {
            let values = values.as_array().ok_or_else(|| {
                Error::Preflight(format!("Qoder {key} configuration must be an array"))
            })?;
            for value in values {
                let value = value
                    .as_str()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| {
                        Error::Preflight(format!("Qoder {key} entries must be nonblank strings"))
                    })?;
                args.extend([OsString::from(flag), OsString::from(value)]);
            }
        }
    }
    for (key, flag) in [("agents", "--agents"), ("settings", "--settings")] {
        if let Some(value) = object.get(key) {
            if !value.is_object() {
                return Err(Error::Preflight(format!(
                    "Qoder {key} configuration must be an object"
                )));
            }
            args.extend([
                OsString::from(flag),
                OsString::from(
                    serde_json::to_string(value)
                        .map_err(|error| Error::Preflight(error.to_string()))?,
                ),
            ]);
        }
    }
    Ok(())
}

fn parse_params<T: serde::de::DeserializeOwned>(params: Value) -> Result<T> {
    serde_json::from_value(params)
        .map_err(|error| Error::Preflight(format!("invalid ACP request parameters: {error}")))
}

fn to_value(value: impl Serialize) -> Result<Value> {
    serde_json::to_value(value)
        .map_err(|error| Error::Preflight(format!("invalid ACP response: {error}")))
}

fn retain_utf8_tail(bytes: Vec<u8>, limit: usize) -> (Vec<u8>, bool) {
    if bytes.len() <= limit {
        return (bytes, false);
    }
    let mut start = bytes.len() - limit;
    while start < bytes.len() && std::str::from_utf8(&bytes[start..]).is_err() {
        start += 1;
    }
    (bytes[start..].to_vec(), true)
}

fn append_agent_text(frame: &Value, target: &mut String, max: usize) -> Result<()> {
    let update = frame.pointer("/params/update");
    if update
        .and_then(|value| value.get("sessionUpdate"))
        .and_then(Value::as_str)
        != Some("agent_message_chunk")
    {
        return Ok(());
    }
    if let Some(text) = update
        .and_then(|value| value.pointer("/content/text"))
        .and_then(Value::as_str)
    {
        if target.len().saturating_add(text.len()) > max {
            return Err(Error::Limit("Qoder result limit exceeded".into()));
        }
        target.push_str(text);
    }
    Ok(())
}

async fn write_frame(process: &mut OwnedPipedProcess, frame: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec(frame)
        .map_err(|error| Error::Preflight(format!("invalid ACP frame: {error}")))?;
    bytes.push(b'\n');
    process.stdin().write_all(&bytes).await?;
    process.stdin().flush().await?;
    Ok(())
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

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    std::io::Write::write_all(&mut options.open(path)?, bytes)?;
    Ok(())
}
