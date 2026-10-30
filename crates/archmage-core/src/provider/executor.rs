use crate::{
    agent::ProviderKind,
    error::{Error, Result},
    process::ProcessIdentity,
    provider::codex,
};
use std::{path::PathBuf, sync::Mutex, time::Duration};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderSession {
    None,
    Create(String),
    Resume(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderLifecycle {
    Spawned(ProcessIdentity),
    ConversationPersisted { session_id: String },
}

#[derive(Clone, Debug)]
pub struct ProviderExecution {
    pub provider_kind: ProviderKind,
    pub executable_path: Option<PathBuf>,
    pub model: Option<String>,
    pub repository: PathBuf,
    pub schema_path: PathBuf,
    pub result_path: PathBuf,
    pub diagnostics_dir: PathBuf,
    pub sandbox: codex::Sandbox,
    pub session: ProviderSession,
    pub deadline: Duration,
    pub max_line_bytes: usize,
    pub max_stream_bytes: usize,
    pub max_result_bytes: usize,
}

#[derive(Debug)]
pub struct ProviderOutcome {
    pub conversation_id: String,
    pub result: serde_json::Value,
}

pub async fn invoke(
    execution: &ProviderExecution,
    prompt: &[u8],
    observer: impl FnMut(ProviderLifecycle) -> Result<()> + Send,
) -> Result<ProviderOutcome> {
    let observer = Mutex::new(observer);
    match execution.provider_kind {
        ProviderKind::Codex => {
            let tools = codex::inspect_at(execution.executable_path.as_deref()).await?;
            let mut invocation = codex::CodexInvocation::new(
                execution.repository.clone(),
                execution.schema_path.clone(),
                execution.result_path.clone(),
                execution.diagnostics_dir.clone(),
                execution.sandbox,
            );
            invocation.model = execution.model.clone();
            invocation.deadline = execution.deadline;
            invocation.max_line_bytes = execution.max_line_bytes;
            invocation.max_stream_bytes = execution.max_stream_bytes;
            invocation.max_result_bytes = execution.max_result_bytes;
            let outcome = codex::invoke_with_identity(&tools, &invocation, prompt, |identity| {
                (observer.lock().expect("provider observer lock"))(ProviderLifecycle::Spawned(
                    identity.clone(),
                ))
            })
            .await?;
            Ok(ProviderOutcome {
                conversation_id: outcome.thread_id,
                result: outcome.result,
            })
        }
        ProviderKind::DeepseekHarness => Err(Error::Preflight(
            "DeepSeek Harness execution requires the Runtime sidecar pool".into(),
        )),
        kind => Err(Error::Preflight(format!(
            "{kind} provider is not supported yet"
        ))),
    }
}
