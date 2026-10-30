use crate::{
    error::{Error, Result},
    process::{OwnedPipedProcess, ProcessIdentity, capture, spawn_owned_piped},
    provider::{codex::find_executable, executor::ProviderOutcome},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

const BRIDGE: &str = include_str!("../../resources/deepseek_harness_bridge.py");
const CORDIS: &str = include_str!("../../resources/deepseek_harness.cordis.yml");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SdkInspection {
    pub python_binary: PathBuf,
    pub sdk_version: String,
    pub runtime_version: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionMode {
    ReadOnly,
    WorkspaceWrite,
}

impl PermissionMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::WorkspaceWrite => "workspace-write",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SidecarConfig {
    pub python_binary: PathBuf,
    pub data_root: PathBuf,
    pub repository: PathBuf,
    pub provider_route: String,
    pub model: String,
    pub session_id: String,
    pub permission_mode: PermissionMode,
    pub session_root: PathBuf,
    pub diagnostics_dir: PathBuf,
    pub deadline: Duration,
    pub max_line_bytes: usize,
    pub max_stream_bytes: usize,
    pub max_result_bytes: usize,
    pub environment: BTreeMap<OsString, OsString>,
}

pub struct DeepseekHarnessSidecar {
    process: OwnedPipedProcess,
    config: SidecarConfig,
    poisoned: Option<String>,
    stderr_path: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InspectionFrame {
    sdk_version: String,
    runtime_version: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeFrame {
    op: String,
    id: Option<String>,
    session_id: Option<String>,
    text: Option<String>,
    message: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartFrame<'a> {
    op: &'static str,
    protocol: u8,
    cwd: &'a Path,
    provider: &'a str,
    model: &'a str,
    session_id: &'a str,
    cordis: &'a Path,
    session_root: &'a Path,
    max_result_bytes: usize,
    request_timeout_seconds: u64,
}

pub async fn inspect_at(python_binary: Option<&Path>) -> Result<SdkInspection> {
    let python_binary = match python_binary {
        Some(path) if path.is_file() => std::path::absolute(path)?,
        Some(path) => {
            return Err(Error::Preflight(format!(
                "configured Python executable does not exist: {}",
                path.display()
            )));
        }
        None => find_executable("python3")?,
    };
    let probe = r#"import json
from importlib.metadata import version
import deepseek_harness
print(json.dumps({"sdkVersion":version("deepseek-harness-sdk"),"runtimeVersion":version("deepseek-harness-runtime-bin")}))"#;
    let output = capture(
        &python_binary,
        &["-c".into(), probe.into()],
        None,
        b"",
        Duration::from_secs(10),
        1024 * 1024,
    )
    .await?;
    if output.code != Some(0) {
        return Err(Error::Preflight(
            "DeepSeek Harness Python SDK inspection failed".into(),
        ));
    }
    let frame: InspectionFrame = serde_json::from_slice(&output.stdout)
        .map_err(|_| Error::Preflight("invalid DeepSeek Harness SDK inspection output".into()))?;
    if frame.sdk_version != frame.runtime_version {
        return Err(Error::Preflight(
            "DeepSeek Harness SDK/runtime version mismatch".into(),
        ));
    }
    Ok(SdkInspection {
        python_binary,
        sdk_version: frame.sdk_version,
        runtime_version: frame.runtime_version,
    })
}

impl DeepseekHarnessSidecar {
    pub async fn start(config: SidecarConfig) -> Result<Self> {
        if !config.repository.is_dir() {
            return Err(Error::Preflight(
                "DeepSeek Harness repository must exist".into(),
            ));
        }
        private_directory(&config.session_root)?;
        private_directory(&config.diagnostics_dir)?;
        let (bridge, cordis) = materialize_resources(&config.data_root)?;
        let mut environment = config.environment.clone();
        environment.insert(
            "DSH_PERMISSION_MODE".into(),
            config.permission_mode.as_str().into(),
        );
        let args = vec![OsString::from("-u"), bridge.as_os_str().to_owned()];
        let mut process = spawn_owned_piped(
            &config.python_binary,
            &args,
            Some(&config.repository),
            &environment.into_iter().collect::<Vec<_>>(),
            config.max_stream_bytes,
        )
        .await?;
        let start = StartFrame {
            op: "start",
            protocol: 1,
            cwd: &config.repository,
            provider: &config.provider_route,
            model: &config.model,
            session_id: &config.session_id,
            cordis: &cordis,
            session_root: &config.session_root,
            max_result_bytes: config.max_result_bytes,
            request_timeout_seconds: config.deadline.as_secs().max(1),
        };
        write_frame(&mut process, &start).await?;
        let ready = read_frame(&mut process, config.deadline, config.max_line_bytes).await?;
        if ready.op != "ready" || ready.session_id.as_deref() != Some(&config.session_id) {
            let _ = process.cancel(Duration::from_secs(1)).await;
            return Err(Error::Preflight(
                "DeepSeek Harness bridge returned an invalid ready frame".into(),
            ));
        }
        let stderr_path = config.diagnostics_dir.join("stderr.log");
        Ok(Self {
            process,
            config,
            poisoned: None,
            stderr_path,
        })
    }

    pub fn identity(&self) -> &ProcessIdentity {
        self.process.identity()
    }

    pub async fn invoke(
        &mut self,
        id: &str,
        task: &[u8],
        schema: &serde_json::Value,
    ) -> Result<ProviderOutcome> {
        if let Some(error) = &self.poisoned {
            return Err(Error::Preflight(error.clone()));
        }
        let result = self.invoke_inner(id, task, schema).await;
        if let Err(error) = &result {
            self.poisoned = Some(match error {
                Error::Preflight(message) => message.clone(),
                _ => error.to_string(),
            });
        }
        result
    }

    async fn invoke_inner(
        &mut self,
        id: &str,
        task: &[u8],
        schema: &serde_json::Value,
    ) -> Result<ProviderOutcome> {
        let task = std::str::from_utf8(task)
            .map_err(|_| Error::Preflight("DeepSeek Harness task is not UTF-8".into()))?;
        let frame = serde_json::json!({"op":"invoke", "id":id, "task":task, "schema":schema});
        write_frame(&mut self.process, &frame).await?;
        let response = read_frame(
            &mut self.process,
            self.config.deadline,
            self.config.max_line_bytes,
        )
        .await?;
        if response.op == "error" {
            return Err(Error::Preflight(
                response
                    .message
                    .unwrap_or_else(|| "DeepSeek Harness bridge failed".into()),
            ));
        }
        if response.op != "result"
            || response.id.as_deref() != Some(id)
            || response.session_id.as_deref() != Some(&self.config.session_id)
        {
            return Err(Error::Preflight(
                "DeepSeek Harness bridge protocol mismatch".into(),
            ));
        }
        let text = response
            .text
            .ok_or_else(|| Error::Preflight("DeepSeek Harness result text is missing".into()))?;
        if text.len() > self.config.max_result_bytes {
            return Err(Error::Limit(
                "DeepSeek Harness result limit exceeded".into(),
            ));
        }
        let result: serde_json::Value = serde_json::from_str(&text)
            .map_err(|_| Error::Preflight("invalid DeepSeek Harness result JSON".into()))?;
        if !result.is_object() {
            return Err(Error::Preflight(
                "DeepSeek Harness result must be one JSON object".into(),
            ));
        }
        Ok(ProviderOutcome {
            conversation_id: self.config.session_id.clone(),
            result,
        })
    }

    pub async fn close(mut self) -> Result<()> {
        write_frame(&mut self.process, &serde_json::json!({"op":"close"})).await?;
        let frame = read_frame(
            &mut self.process,
            Duration::from_secs(5),
            self.config.max_line_bytes,
        )
        .await?;
        if frame.op != "closed" {
            let _ = self.process.cancel(Duration::from_secs(1)).await;
            return Err(Error::Preflight(
                "DeepSeek Harness bridge did not close cleanly".into(),
            ));
        }
        let (status, stderr) = self.process.wait().await?;
        write_private(&self.stderr_path, &stderr)?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::Preflight(
                "DeepSeek Harness bridge exited unsuccessfully".into(),
            ))
        }
    }

    pub async fn cancel(mut self) -> Result<()> {
        self.process.cancel(Duration::from_secs(1)).await
    }
}

async fn write_frame(process: &mut OwnedPipedProcess, frame: &impl Serialize) -> Result<()> {
    let mut bytes =
        serde_json::to_vec(frame).map_err(|error| Error::Preflight(error.to_string()))?;
    bytes.push(b'\n');
    process.stdin().write_all(&bytes).await?;
    process.stdin().flush().await?;
    Ok(())
}

async fn read_frame(
    process: &mut OwnedPipedProcess,
    deadline: Duration,
    max_line_bytes: usize,
) -> Result<BridgeFrame> {
    let mut line = Vec::new();
    let count = tokio::time::timeout(deadline, process.stdout().read_until(b'\n', &mut line))
        .await
        .map_err(|_| Error::TimedOut)??;
    if count == 0 {
        return Err(Error::Preflight(
            "DeepSeek Harness bridge exited before responding".into(),
        ));
    }
    if line.len() > max_line_bytes {
        return Err(Error::Limit(
            "DeepSeek Harness bridge line limit exceeded".into(),
        ));
    }
    serde_json::from_slice(&line)
        .map_err(|_| Error::Preflight("invalid DeepSeek Harness bridge frame".into()))
}

fn materialize_resources(data_root: &Path) -> Result<(PathBuf, PathBuf)> {
    let digest = hex_digest([BRIDGE.as_bytes(), CORDIS.as_bytes()].concat());
    let root = data_root
        .join("provider-resources/deepseek-harness")
        .join(digest);
    private_directory(&root)?;
    let bridge = root.join("bridge.py");
    let cordis = root.join("cordis.yml");
    write_private_if_changed(&bridge, BRIDGE.as_bytes())?;
    write_private_if_changed(&cordis, CORDIS.as_bytes())?;
    Ok((bridge, cordis))
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    let digest = Sha256::digest(bytes.as_ref());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
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

fn write_private_if_changed(path: &Path, bytes: &[u8]) -> Result<()> {
    if fs::read(path).is_ok_and(|current| current == bytes) {
        return Ok(());
    }
    write_private(path, bytes)
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

pub fn python_version_label(inspection: &SdkInspection) -> String {
    format!(
        "SDK {} / runtime {}",
        inspection.sdk_version, inspection.runtime_version
    )
}

pub fn path_as_os_string(path: &Path) -> OsString {
    OsStr::new(path).to_owned()
}
