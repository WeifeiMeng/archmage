use crate::{
    error::{Error, Result},
    process::{
        Captured, OwnedPipedProcess, ProcessIdentity, cancel_process, capture,
        capture_observed_with_identity_env, spawn_owned_piped,
    },
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
    TerminalOutputRequest, TerminalOutputResponse, ToolKind, WaitForTerminalExitRequest,
    WaitForTerminalExitResponse, WriteTextFileRequest, WriteTextFileResponse,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    sync::Notify,
};

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
    supports_session_close: bool,
    cancellation: QoderCancelHandle,
    resume: Option<QoderResume>,
    resumed: bool,
}

#[derive(Clone, Debug)]
struct QoderResume {
    session_id: String,
    workspace_snapshot_id: String,
}

#[derive(Clone)]
pub struct QoderCancelHandle {
    state: Arc<QoderCancelState>,
}

struct QoderCancelState {
    cancelled: AtomicBool,
    notify: tokio::sync::Notify,
    acknowledged: tokio::sync::Notify,
    identity: ProcessIdentity,
}

impl QoderCancelHandle {
    fn new(identity: ProcessIdentity) -> Self {
        Self {
            state: Arc::new(QoderCancelState {
                cancelled: AtomicBool::new(false),
                notify: tokio::sync::Notify::new(),
                acknowledged: tokio::sync::Notify::new(),
                identity,
            }),
        }
    }

    pub async fn cancel(&self) -> Result<()> {
        self.state.cancelled.store(true, Ordering::Release);
        // `notify_one` stores a permit when cancellation races with construction of
        // the `notified()` future; `notify_waiters` would lose that wake-up.
        self.state.notify.notify_one();
        // Give an active ACP request a short bounded opportunity to emit
        // `session/cancel`; an idle session has no reader to acknowledge it.
        let _ = tokio::time::timeout(
            Duration::from_millis(100),
            self.state.acknowledged.notified(),
        )
        .await;
        cancel_process(&self.state.identity, Duration::from_secs(1)).await
    }
}

struct TerminalRecord {
    output: Arc<std::sync::Mutex<TerminalOutputState>>,
    exit_status: Option<TerminalExitStatus>,
    identity: ProcessIdentity,
    task: Option<tokio::task::JoinHandle<Result<Captured>>>,
}

#[derive(Default)]
struct TerminalOutputState {
    bytes: Vec<u8>,
    truncated: bool,
}

impl TerminalOutputState {
    fn append(&mut self, chunk: &[u8], limit: usize) {
        self.bytes.extend_from_slice(chunk);
        if self.bytes.len() > limit {
            let discard = self.bytes.len() - limit;
            self.bytes.drain(..discard);
            while self
                .bytes
                .first()
                .is_some_and(|byte| byte & 0b1100_0000 == 0b1000_0000)
            {
                self.bytes.remove(0);
            }
            self.truncated = true;
        }
    }
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
        Self::start_inner(config, None, None).await
    }

    pub(crate) async fn start_cancellable(
        config: QoderSessionConfig,
        cancelled: &AtomicBool,
        cancel_notify: &Notify,
    ) -> Result<Self> {
        Self::start_inner(config, None, Some((cancelled, cancel_notify))).await
    }

    pub async fn resume_if_eligible(
        config: QoderSessionConfig,
        session_id: impl Into<String>,
        workspace_snapshot_id: impl Into<String>,
    ) -> Result<Self> {
        Self::start_inner(
            config,
            Some(QoderResume {
                session_id: session_id.into(),
                workspace_snapshot_id: workspace_snapshot_id.into(),
            }),
            None,
        )
        .await
    }

    async fn start_inner(
        config: QoderSessionConfig,
        mut resume: Option<QoderResume>,
        external_cancel: Option<(&AtomicBool, &Notify)>,
    ) -> Result<Self> {
        if !config.repository.is_dir() {
            return Err(Error::Preflight("Qoder repository must exist".into()));
        }
        if !config.provider_config.is_object() {
            return Err(Error::Preflight(
                "Qoder provider configuration must be an object".into(),
            ));
        }
        private_directory(&config.diagnostics_dir)?;
        if let Some(candidate) = &resume {
            let repository = crate::workspace::inspect_repository(&config.repository)?;
            if crate::workspace::snapshot(&repository)?.id != candidate.workspace_snapshot_id {
                resume = None;
            }
        }
        let isolated_config = prepare_isolated_config(&config.diagnostics_dir)?;
        let policy = AcpWorkspacePolicy::new(&config.repository, config.access)?;
        let mut args = vec![
            OsString::from("--acp"),
            OsString::from("--config-dir"),
            isolated_config.into_os_string(),
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
        let mut environment = config.environment.clone();
        let temp_dir = config.diagnostics_dir.join("tmp");
        private_directory(&temp_dir)?;
        environment.insert("TMPDIR".into(), temp_dir.into_os_string());
        let (launch_binary, args) = sandboxed_qoder_command(&config, args)?;
        let process = spawn_owned_piped(
            &launch_binary,
            &args,
            Some(&config.repository),
            &environment.into_iter().collect::<Vec<_>>(),
            config.max_stream_bytes,
        )
        .await?;
        let stderr_path = config.diagnostics_dir.join("stderr.log");
        let cancellation = QoderCancelHandle::new(process.identity().clone());
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
            supports_session_close: false,
            cancellation,
            resume,
            resumed: false,
        };
        let initialize_result = match external_cancel {
            Some((cancelled, _)) if cancelled.load(Ordering::Acquire) => {
                Err(Error::Preflight("provider session was cancelled".into()))
            }
            Some((_, cancel_notify)) => {
                tokio::select! {
                    result = session.initialize() => result,
                    () = cancel_notify.notified() => {
                        Err(Error::Preflight("provider session was cancelled".into()))
                    }
                }
            }
            None => session.initialize().await,
        };
        if let Err(error) = initialize_result {
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

    pub fn cancellation_handle(&self) -> QoderCancelHandle {
        self.cancellation.clone()
    }

    pub fn was_resumed(&self) -> bool {
        self.resumed
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
        self.supports_session_close = result
            .pointer("/agentCapabilities/sessionCapabilities/close")
            .is_some();
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
        if result
            .pointer("/agentCapabilities/loadSession")
            .and_then(Value::as_bool)
            == Some(true)
            && let Some(resume) = self.resume.clone()
        {
            let load = self
                .request_with_remote_error(
                    "session/load",
                    json!({
                        "sessionId": resume.session_id,
                        "cwd": self.config.repository,
                        "mcpServers": mcp_servers
                    }),
                    None,
                )
                .await?;
            if load.is_ok() {
                self.session_id = resume.session_id;
                self.resumed = true;
                return Ok(());
            }
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
        let _ = self.terminate_terminals().await;
        self.process.cancel(Duration::from_secs(1)).await
    }

    pub async fn close(mut self) -> Result<()> {
        if self.supports_session_close {
            let session_id = self.session_id.clone();
            if let Err(error) = self
                .request("session/close", json!({"sessionId": session_id}), None)
                .await
            {
                let _ = self.terminate_terminals().await;
                let _ = self.process.cancel(Duration::from_secs(1)).await;
                return Err(error);
            }
        }
        if let Err(error) = self.terminate_terminals().await {
            let _ = self.process.cancel(Duration::from_secs(1)).await;
            return Err(error);
        }
        self.process.shutdown_stdin().await?;
        let (status, stderr) =
            match tokio::time::timeout(Duration::from_secs(2), self.process.wait()).await {
                Ok(result) => result?,
                Err(_) if self.supports_session_close => {
                    self.process.cancel(Duration::from_secs(1)).await?;
                    return Ok(());
                }
                Err(_) => return Err(Error::TimedOut),
            };
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
        agent_text: Option<&mut String>,
    ) -> Result<Value> {
        self.request_with_remote_error(method, params, agent_text)
            .await?
            .map_err(|error| Error::Preflight(format!("Qoder ACP request failed: {error}")))
    }

    async fn request_with_remote_error(
        &mut self,
        method: &str,
        params: Value,
        mut agent_text: Option<&mut String>,
    ) -> Result<std::result::Result<Value, Value>> {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        write_frame(
            &mut self.process,
            &json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}),
        )
        .await?;
        loop {
            let frame = self.read_frame().await?;
            if frame.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
                return Err(Error::Preflight(
                    "Qoder ACP frame has an invalid JSON-RPC version".into(),
                ));
            }
            let frame_method = frame.get("method").and_then(Value::as_str);
            if frame_method.is_none() && frame.get("id").and_then(Value::as_u64) != Some(id) {
                return Err(Error::Preflight(
                    "Qoder ACP response ID does not match the pending request".into(),
                ));
            }
            if frame_method.is_none() {
                if let Some(error) = frame.get("error") {
                    return Ok(Err(error.clone()));
                }
                return Ok(Ok(frame.get("result").cloned().ok_or_else(|| {
                    Error::Preflight("Qoder ACP response has no result".into())
                })?));
            }
            if frame_method == Some("session/update") {
                if frame.pointer("/params/sessionId").and_then(Value::as_str)
                    != Some(self.session_id.as_str())
                {
                    return Err(Error::Preflight(
                        "Qoder ACP update has the wrong session ID".into(),
                    ));
                }
                if let Some(target) = agent_text.as_deref_mut() {
                    append_agent_text(&frame, target, self.config.max_result_bytes)?;
                }
                continue;
            }
            if frame.get("id").is_some() && frame_method.is_some() {
                self.handle_client_request(frame).await?;
            }
        }
    }

    async fn read_frame(&mut self) -> Result<Value> {
        if self.cancellation.state.cancelled.load(Ordering::Acquire) {
            return self.cancel_running_turn().await;
        }
        let mut line = Vec::new();
        let read = self.process.stdout().read_until(b'\n', &mut line);
        let notified = self.cancellation.state.notify.notified();
        let count = tokio::select! {
            result = tokio::time::timeout(self.config.deadline, read) => {
                result.map_err(|_| Error::TimedOut)??
            }
            () = notified => return self.cancel_running_turn().await,
        };
        if count == 0 {
            if let Ok((_, stderr)) = self.process.wait().await {
                let _ = write_private(&self.stderr_path, &stderr);
            }
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

    async fn cancel_running_turn<T>(&mut self) -> Result<T> {
        if !self.session_id.is_empty() {
            let frame = json!({
                "jsonrpc":"2.0",
                "method":"session/cancel",
                "params":{"sessionId":self.session_id}
            });
            let _ = write_frame(&mut self.process, &frame).await;
        }
        self.cancellation.state.acknowledged.notify_one();
        let _ = self.terminate_terminals().await;
        self.process.cancel(Duration::from_secs(1)).await?;
        Err(Error::Preflight("Qoder session was cancelled".into()))
    }

    async fn terminate_terminals(&mut self) -> Result<()> {
        let terminals = std::mem::take(&mut self.terminals);
        let mut first_error = None;
        for (_, mut record) in terminals {
            if let Some(task) = record.task.take() {
                if let Err(error) = cancel_process(&record.identity, Duration::from_secs(1)).await
                    && first_error.is_none()
                {
                    first_error = Some(error);
                }
                match task.await {
                    Ok(Ok(_)) => {}
                    Ok(Err(error)) if first_error.is_none() => first_error = Some(error),
                    Err(error) if first_error.is_none() => {
                        first_error = Some(Error::Preflight(format!(
                            "ACP terminal task failed: {error}"
                        )))
                    }
                    _ => {}
                }
            }
        }
        first_error.map_or(Ok(()), Err)
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
                let preferred = if self.permission_is_safe(&request) {
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
                self.refresh_terminal(&request.terminal_id.to_string(), false)
                    .await?;
                let record = self.terminal(&request.terminal_id.to_string())?;
                let output = record.output.lock().expect("terminal output lock");
                let mut response = TerminalOutputResponse::new(
                    String::from_utf8_lossy(&output.bytes).into_owned(),
                    output.truncated,
                );
                if let Some(status) = &record.exit_status {
                    response = response.exit_status(status.clone());
                }
                to_value(response)
            }
            "terminal/wait_for_exit" => {
                let request: WaitForTerminalExitRequest = parse_params(params)?;
                self.require_session(&request.session_id.to_string())?;
                self.refresh_terminal(&request.terminal_id.to_string(), true)
                    .await?;
                let record = self.terminal(&request.terminal_id.to_string())?;
                to_value(WaitForTerminalExitResponse::new(
                    record
                        .exit_status
                        .clone()
                        .ok_or_else(|| Error::Preflight("ACP terminal is still running".into()))?,
                ))
            }
            "terminal/kill" => {
                let request: KillTerminalRequest = parse_params(params)?;
                self.require_session(&request.session_id.to_string())?;
                let identity = self
                    .terminal(&request.terminal_id.to_string())?
                    .identity
                    .clone();
                cancel_process(&identity, Duration::from_secs(1)).await?;
                self.refresh_terminal(&request.terminal_id.to_string(), true)
                    .await?;
                to_value(KillTerminalResponse::new())
            }
            "terminal/release" => {
                let request: ReleaseTerminalRequest = parse_params(params)?;
                self.require_session(&request.session_id.to_string())?;
                let mut record = self
                    .terminals
                    .remove(&request.terminal_id.to_string())
                    .ok_or_else(|| Error::Preflight("unknown ACP terminal".into()))?;
                if let Some(task) = record.task.take() {
                    cancel_process(&record.identity, Duration::from_secs(1)).await?;
                    let _ = task.await;
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
        let mut environment = request
            .env
            .into_iter()
            .map(|item| (OsString::from(item.name), OsString::from(item.value)))
            .collect::<BTreeMap<_, _>>();
        environment.insert(
            "TMPDIR".into(),
            self.config.diagnostics_dir.join("tmp").into_os_string(),
        );
        environment.insert("PWD".into(), cwd.as_os_str().to_owned());
        let args = request
            .args
            .into_iter()
            .map(OsString::from)
            .collect::<Vec<_>>();
        let (command, args) =
            sandboxed_terminal_command(&self.config, Path::new(&request.command), args)?;
        let (identity_tx, identity_rx) = tokio::sync::oneshot::channel();
        let identity_tx = std::sync::Mutex::new(Some(identity_tx));
        let deadline = self.config.deadline;
        let max_stream_bytes = self.config.max_stream_bytes;
        let environment = environment.into_iter().collect::<Vec<_>>();
        let output = Arc::new(std::sync::Mutex::new(TerminalOutputState::default()));
        let observed_output = output.clone();
        let task = tokio::spawn(async move {
            capture_observed_with_identity_env(
                &command,
                &args,
                Some(&cwd),
                b"",
                deadline,
                max_stream_bytes,
                &environment,
                |identity| {
                    identity_tx
                        .lock()
                        .expect("terminal identity lock")
                        .take()
                        .ok_or(Error::Conflict)?
                        .send(identity.clone())
                        .map_err(|_| Error::Conflict)
                },
                move |_, chunk| {
                    observed_output
                        .lock()
                        .expect("terminal output lock")
                        .append(chunk, limit);
                    Ok(())
                },
            )
            .await
        });
        let identity = tokio::time::timeout(self.config.deadline, identity_rx)
            .await
            .map_err(|_| Error::TimedOut)?
            .map_err(|_| Error::Preflight("ACP terminal failed before spawn".into()))?;
        self.next_terminal = self.next_terminal.saturating_add(1);
        let terminal_id = format!("archmage-{}", self.next_terminal);
        self.terminals.insert(
            terminal_id.clone(),
            TerminalRecord {
                output,
                exit_status: None,
                identity,
                task: Some(task),
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

    async fn refresh_terminal(&mut self, terminal_id: &str, wait: bool) -> Result<()> {
        let (task, identity) = {
            let record = self
                .terminals
                .get_mut(terminal_id)
                .ok_or_else(|| Error::Preflight("unknown ACP terminal".into()))?;
            if !wait && record.task.as_ref().is_some_and(|task| !task.is_finished()) {
                return Ok(());
            }
            (record.task.take(), record.identity.clone())
        };
        let Some(mut task) = task else {
            return Ok(());
        };
        let joined = if wait {
            let notified = self.cancellation.state.notify.notified();
            tokio::select! {
                result = &mut task => result,
                () = notified => {
                    let _ = cancel_process(&identity, Duration::from_secs(1)).await;
                    let _ = task.await;
                    return Err(Error::Preflight("Qoder session was cancelled".into()));
                }
            }
        } else {
            task.await
        };
        let output = joined
            .map_err(|error| Error::Preflight(format!("ACP terminal task failed: {error}")))??;
        let record = self
            .terminals
            .get_mut(terminal_id)
            .ok_or_else(|| Error::Preflight("unknown ACP terminal".into()))?;
        finish_terminal(record, output);
        Ok(())
    }

    fn permission_is_safe(&self, request: &RequestPermissionRequest) -> bool {
        if self.config.access == AcpAccess::ReadOnly {
            return false;
        }
        let fields = &request.tool_call.fields;
        let locations_safe = |write| {
            fields.locations.as_ref().is_some_and(|locations| {
                !locations.is_empty()
                    && locations
                        .iter()
                        .all(|location| self.policy.resolve(&location.path, write).is_ok())
            })
        };
        match fields.kind {
            Some(ToolKind::Edit | ToolKind::Move) => locations_safe(true),
            Some(ToolKind::Delete) => false,
            Some(ToolKind::Execute) => fields
                .raw_input
                .as_ref()
                .is_some_and(|input| self.permission_terminal_is_safe(input)),
            Some(ToolKind::Read | ToolKind::Search) => locations_safe(false),
            Some(ToolKind::Think) => true,
            _ => false,
        }
    }

    fn permission_terminal_is_safe(&self, input: &Value) -> bool {
        let Some(command_line) = input.get("command").and_then(Value::as_str) else {
            return false;
        };
        if command_line.contains([';', '|', '&', '$', '`', '>', '<', '\n']) {
            return false;
        }
        let mut words = command_line.split_whitespace();
        let Some(command) = words.next() else {
            return false;
        };
        let args = words.map(str::to_owned).collect::<Vec<_>>();
        let cwd = input
            .get("cwd")
            .and_then(Value::as_str)
            .map(Path::new)
            .unwrap_or(self.policy.root());
        self.policy.validate_terminal(cwd, command, &args).is_ok()
    }
}

fn finish_terminal(record: &mut TerminalRecord, output: Captured) {
    record.exit_status = Some(
        TerminalExitStatus::new()
            .exit_code(output.code.and_then(|code| u32::try_from(code).ok()))
            .signal(output.signal.map(|signal| signal.to_string())),
    );
}

#[cfg(unix)]
fn prepare_isolated_config(diagnostics_dir: &Path) -> Result<PathBuf> {
    use std::os::unix::fs::symlink;
    let target = diagnostics_dir.join("qoder-config");
    private_directory(&target)?;
    let source = std::env::var_os("QODER_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".qoder")));
    let Some(source) = source else {
        return Ok(target);
    };
    for name in [
        ".auth",
        "installation_id",
        "state.json",
        "umid-cache.json",
        ".models",
    ] {
        let source_item = source.join(name);
        let target_item = target.join(name);
        if source_item.exists() && !target_item.exists() {
            symlink(source_item, target_item)?;
        }
    }
    Ok(target)
}

#[cfg(not(unix))]
fn prepare_isolated_config(diagnostics_dir: &Path) -> Result<PathBuf> {
    let target = diagnostics_dir.join("qoder-config");
    private_directory(&target)?;
    Ok(target)
}

#[cfg(target_os = "macos")]
fn macos_path_rule(operation: &str, path: &Path) -> String {
    let escaped = path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    if path.is_dir() {
        format!("(allow {operation} (subpath \"{escaped}\"))")
    } else {
        format!("(allow {operation} (literal \"{escaped}\"))")
    }
}

#[cfg(target_os = "macos")]
fn macos_ancestor_traversal_rules(path: &Path) -> String {
    let mut rules = String::new();
    let mut current = path.parent();
    while let Some(ancestor) = current {
        if ancestor == Path::new("/") {
            break;
        }
        let escaped = ancestor
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        rules.push_str(&format!(
            "(allow file-read-metadata file-read-data (literal \"{escaped}\"))"
        ));
        current = ancestor.parent();
    }
    rules
}

#[cfg(target_os = "macos")]
fn macos_sandbox_profile(
    config: &QoderSessionConfig,
    extra_read_paths: &[PathBuf],
) -> Result<String> {
    let repository = config.repository.canonicalize()?;
    let diagnostics = config.diagnostics_dir.canonicalize()?;
    let mut profile = String::from(
        "(version 1)(allow default)\
         (deny file-read* (subpath \"/Users\") (subpath \"/Volumes\") \
         (subpath \"/tmp\") (subpath \"/var/folders\") \
         (subpath \"/private/tmp\") (subpath \"/private/var/folders\"))\
         (deny file-write*)\
         (allow file-write* (literal \"/dev/null\") (literal \"/dev/dtracehelper\"))",
    );
    for path in [&config.repository, &repository] {
        profile.push_str(&macos_ancestor_traversal_rules(path));
        profile.push_str(&macos_path_rule("file-read*", path));
    }
    for path in [&config.diagnostics_dir, &diagnostics] {
        profile.push_str(&macos_ancestor_traversal_rules(path));
        profile.push_str(&macos_path_rule("file-read*", path));
        profile.push_str(&macos_path_rule("file-write*", path));
    }
    if config.access == AcpAccess::WorkspaceWrite {
        profile.push_str(&macos_path_rule("file-write*", &config.repository));
        profile.push_str(&macos_path_rule("file-write*", &repository));
    }
    for path in extra_read_paths {
        profile.push_str(&macos_ancestor_traversal_rules(path));
        profile.push_str(&macos_path_rule("file-read*", path));
    }
    Ok(profile)
}

#[cfg(target_os = "macos")]
fn qoder_runtime_read_paths(config: &QoderSessionConfig) -> Vec<PathBuf> {
    let mut paths = vec![config.binary.clone()];
    if let Ok(binary) = config.binary.canonicalize() {
        paths.push(binary);
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        paths.extend([
            home.join(".qoder/entry"),
            home.join(".qoder/bin"),
            home.join(".local/bin/qodercli"),
        ]);
    }
    let isolated = config.diagnostics_dir.join("qoder-config");
    for name in [
        ".auth",
        "installation_id",
        "state.json",
        "umid-cache.json",
        ".models",
    ] {
        let item = isolated.join(name);
        if let Ok(target) = item.canonicalize() {
            paths.push(target);
        }
    }
    paths.retain(|path| path.exists());
    paths.sort();
    paths.dedup();
    paths
}

#[cfg(target_os = "macos")]
fn sandboxed_qoder_command(
    config: &QoderSessionConfig,
    qoder_args: Vec<OsString>,
) -> Result<(PathBuf, Vec<OsString>)> {
    let profile = macos_sandbox_profile(config, &qoder_runtime_read_paths(config))?;
    let mut args = vec![
        OsString::from("-p"),
        OsString::from(profile),
        config.binary.as_os_str().to_owned(),
    ];
    args.extend(qoder_args);
    Ok((PathBuf::from("/usr/bin/sandbox-exec"), args))
}

#[cfg(target_os = "macos")]
fn sandboxed_terminal_command(
    config: &QoderSessionConfig,
    command: &Path,
    command_args: Vec<OsString>,
) -> Result<(PathBuf, Vec<OsString>)> {
    let read_paths = command
        .canonicalize()
        .map_or_else(|_| vec![command.to_path_buf()], |path| vec![path]);
    let profile = macos_sandbox_profile(config, &read_paths)?;
    let mut args = vec![
        OsString::from("-p"),
        OsString::from(profile),
        command.as_os_str().to_owned(),
    ];
    args.extend(command_args);
    Ok((PathBuf::from("/usr/bin/sandbox-exec"), args))
}

#[cfg(not(target_os = "macos"))]
fn sandboxed_terminal_command(
    config: &QoderSessionConfig,
    command: &Path,
    command_args: Vec<OsString>,
) -> Result<(PathBuf, Vec<OsString>)> {
    let _ = (config, command, command_args);
    Err(Error::Preflight(
        "Qoder terminals require an available OS sandbox".into(),
    ))
}

#[cfg(not(target_os = "macos"))]
fn sandboxed_qoder_command(
    config: &QoderSessionConfig,
    qoder_args: Vec<OsString>,
) -> Result<(PathBuf, Vec<OsString>)> {
    let _ = (config, qoder_args);
    Err(Error::Preflight(
        "Qoder execution requires an available OS sandbox".into(),
    ))
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

#[cfg(test)]
mod tests {
    use super::TerminalOutputState;

    #[test]
    fn terminal_tail_truncation_starts_at_a_utf8_boundary() {
        let mut output = TerminalOutputState::default();
        output.append("你好".as_bytes(), 4);
        assert_eq!(String::from_utf8(output.bytes).unwrap(), "好");
        assert!(output.truncated);
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn qoder_execution_fails_closed_without_an_os_sandbox() {
        use super::{AcpAccess, QoderSessionConfig, sandboxed_qoder_command};
        use std::{collections::BTreeMap, path::PathBuf, time::Duration};

        for access in [AcpAccess::ReadOnly, AcpAccess::WorkspaceWrite] {
            let config = QoderSessionConfig {
                binary: PathBuf::from("qoder"),
                repository: PathBuf::from("repository"),
                model: None,
                agent_name: None,
                provider_config: serde_json::json!({}),
                access,
                diagnostics_dir: PathBuf::from("diagnostics"),
                deadline: Duration::from_secs(1),
                max_line_bytes: 1,
                max_stream_bytes: 1,
                max_result_bytes: 1,
                environment: BTreeMap::new(),
            };
            assert!(sandboxed_qoder_command(&config, Vec::new()).is_err());
        }
    }
}
