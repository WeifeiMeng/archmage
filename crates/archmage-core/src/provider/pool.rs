use crate::{
    error::{Error, Result},
    process::ProcessIdentity,
    provider::{
        deepseek_harness::{DeepseekHarnessSidecar, SidecarConfig},
        executor::ProviderOutcome,
        qoder::{QoderSession, QoderSessionConfig},
    },
};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{Mutex, Notify};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct SidecarKey {
    pub run_id: String,
    pub capability: String,
}

impl SidecarKey {
    pub fn new(run_id: impl Into<String>, capability: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            capability: capability.into(),
        }
    }
}

pub enum ProviderSessionConfig {
    DeepseekHarness(SidecarConfig),
    Qoder(QoderSessionConfig),
}

impl From<SidecarConfig> for ProviderSessionConfig {
    fn from(value: SidecarConfig) -> Self {
        Self::DeepseekHarness(value)
    }
}

impl From<QoderSessionConfig> for ProviderSessionConfig {
    fn from(value: QoderSessionConfig) -> Self {
        Self::Qoder(value)
    }
}

enum PersistentProviderSession {
    DeepseekHarness(DeepseekHarnessSidecar),
    Qoder(QoderSession),
}

impl PersistentProviderSession {
    fn identity(&self) -> &ProcessIdentity {
        match self {
            Self::DeepseekHarness(session) => session.identity(),
            Self::Qoder(session) => session.identity(),
        }
    }

    async fn invoke(
        &mut self,
        invocation_id: &str,
        task: &[u8],
        schema: &serde_json::Value,
    ) -> Result<ProviderOutcome> {
        match self {
            Self::DeepseekHarness(session) => session.invoke(invocation_id, task, schema).await,
            Self::Qoder(session) => session.prompt(invocation_id, task).await,
        }
    }

    async fn close(self) -> Result<()> {
        match self {
            Self::DeepseekHarness(session) => session.close().await,
            Self::Qoder(session) => session.close().await,
        }
    }

    async fn cancel(self) -> Result<()> {
        match self {
            Self::DeepseekHarness(session) => session.cancel().await,
            Self::Qoder(session) => session.cancel().await,
        }
    }

    fn qoder_cancel_handle(&self) -> Option<crate::provider::qoder::QoderCancelHandle> {
        match self {
            Self::Qoder(session) => Some(session.cancellation_handle()),
            Self::DeepseekHarness(_) => None,
        }
    }
}

enum PoolEntry {
    Live(Box<PersistentProviderSession>),
    Poisoned(String),
    Closing,
}

pub struct ProviderPool {
    entries: Mutex<HashMap<SidecarKey, Arc<PoolSlot>>>,
}

struct PoolSlot {
    entry: Mutex<PoolEntry>,
    qoder_cancel: Mutex<Option<crate::provider::qoder::QoderCancelHandle>>,
    cancelled: AtomicBool,
    cancel_notify: Notify,
}

impl Default for ProviderPool {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderPool {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub async fn invoke_main(
        &self,
        key: SidecarKey,
        config: impl Into<ProviderSessionConfig>,
        invocation_id: &str,
        task: &[u8],
        schema: &serde_json::Value,
        on_spawn: impl FnOnce(&ProcessIdentity) -> Result<()>,
    ) -> Result<ProviderOutcome> {
        let mut config = config.into();
        let entry = {
            let mut entries = self.entries.lock().await;
            entries
                .entry(key)
                .or_insert_with(|| {
                    Arc::new(PoolSlot {
                        entry: Mutex::new(PoolEntry::Closing),
                        qoder_cancel: Mutex::new(None),
                        cancelled: AtomicBool::new(false),
                        cancel_notify: Notify::new(),
                    })
                })
                .clone()
        };
        let mut guard = entry.entry.lock().await;
        if matches!(*guard, PoolEntry::Closing) {
            if let ProviderSessionConfig::DeepseekHarness(config) = &mut config {
                config.session_id = format!("archmage-{}", uuid::Uuid::new_v4());
            }
            let start_result =
                start_session_cancellable(config, &entry.cancelled, &entry.cancel_notify).await;
            match start_result {
                Ok(sidecar) => {
                    *entry.qoder_cancel.lock().await = sidecar.qoder_cancel_handle();
                    *guard = PoolEntry::Live(Box::new(sidecar));
                }
                Err(error) => {
                    let message = stable_message(&error);
                    *guard = PoolEntry::Poisoned(message.clone());
                    return Err(Error::Preflight(message));
                }
            }
        }
        if entry.cancelled.load(Ordering::Acquire) {
            if let PoolEntry::Live(session) = std::mem::replace(
                &mut *guard,
                PoolEntry::Poisoned("provider session was cancelled".into()),
            ) {
                let _ = session.cancel().await;
            }
            *entry.qoder_cancel.lock().await = None;
            return Err(Error::Preflight("provider session was cancelled".into()));
        }
        let result = match &mut *guard {
            PoolEntry::Live(session) => match on_spawn(session.identity()) {
                Ok(()) => session.invoke(invocation_id, task, schema).await,
                Err(error) => Err(error),
            },
            PoolEntry::Poisoned(message) => Err(Error::Preflight(message.clone())),
            PoolEntry::Closing => Err(Error::Conflict),
        };
        let cancelled = entry.cancelled.load(Ordering::Acquire);
        if result.is_err() || cancelled {
            let message = if cancelled {
                "provider session was cancelled".into()
            } else {
                stable_message(result.as_ref().expect_err("checked error"))
            };
            let poisoned = PoolEntry::Poisoned(message);
            if let PoolEntry::Live(session) = std::mem::replace(&mut *guard, poisoned) {
                let _ = session.cancel().await;
            }
            *entry.qoder_cancel.lock().await = None;
        }
        if cancelled {
            Err(Error::Preflight("provider session was cancelled".into()))
        } else {
            result
        }
    }

    pub async fn invoke_temporary(
        &self,
        config: impl Into<ProviderSessionConfig>,
        invocation_id: &str,
        task: &[u8],
        schema: &serde_json::Value,
        on_spawn: impl FnOnce(&ProcessIdentity) -> Result<()>,
    ) -> Result<ProviderOutcome> {
        let mut config = config.into();
        if let ProviderSessionConfig::DeepseekHarness(config) = &mut config {
            config.session_id = format!("archmage-consult-{}", uuid::Uuid::new_v4());
        }
        let mut session = start_session(config).await?;
        on_spawn(session.identity())?;
        match session.invoke(invocation_id, task, schema).await {
            Ok(outcome) => {
                session.close().await?;
                Ok(outcome)
            }
            Err(error) => {
                let _ = session.cancel().await;
                Err(error)
            }
        }
    }

    pub async fn close_run(&self, run_id: &str) -> Result<()> {
        let removed = {
            let mut entries = self.entries.lock().await;
            let keys = entries
                .keys()
                .filter(|key| key.run_id == run_id)
                .cloned()
                .collect::<Vec<_>>();
            keys.into_iter()
                .filter_map(|key| entries.remove(&key))
                .collect::<Vec<_>>()
        };
        let mut first_error = None;
        for entry in removed {
            let mut guard = entry.entry.lock().await;
            if let PoolEntry::Live(session) = std::mem::replace(&mut *guard, PoolEntry::Closing)
                && let Err(error) = session.close().await
                && first_error.is_none()
            {
                first_error = Some(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    pub async fn cancel_run(&self, run_id: &str) -> Result<()> {
        let keys = self
            .entries
            .lock()
            .await
            .keys()
            .filter(|key| key.run_id == run_id)
            .cloned()
            .collect::<Vec<_>>();
        for key in keys {
            self.cancel_capability(&key.run_id, &key.capability).await?;
        }
        Ok(())
    }

    pub async fn cancel_capability(&self, run_id: &str, capability: &str) -> Result<()> {
        let key = SidecarKey::new(run_id, capability);
        let entry = self.entries.lock().await.get(&key).cloned();
        let Some(entry) = entry else {
            return Ok(());
        };
        entry.cancelled.store(true, Ordering::Release);
        entry.cancel_notify.notify_one();
        let qoder_cancel = entry.qoder_cancel.lock().await.clone();
        if let Some(handle) = qoder_cancel {
            handle.cancel().await?;
            if let Ok(mut guard) = entry.entry.try_lock()
                && let PoolEntry::Live(session) = std::mem::replace(
                    &mut *guard,
                    PoolEntry::Poisoned("provider session was cancelled".into()),
                )
            {
                *entry.qoder_cancel.lock().await = None;
                session.cancel().await?;
            }
            return Ok(());
        }
        let mut guard = entry.entry.lock().await;
        if let PoolEntry::Live(session) = std::mem::replace(
            &mut *guard,
            PoolEntry::Poisoned("provider session was cancelled".into()),
        ) {
            session.cancel().await?;
        }
        Ok(())
    }
}

async fn start_session(config: ProviderSessionConfig) -> Result<PersistentProviderSession> {
    match config {
        ProviderSessionConfig::DeepseekHarness(config) => {
            Ok(PersistentProviderSession::DeepseekHarness(
                DeepseekHarnessSidecar::start(config).await?,
            ))
        }
        ProviderSessionConfig::Qoder(config) => Ok(PersistentProviderSession::Qoder(
            QoderSession::start(config).await?,
        )),
    }
}

async fn start_session_cancellable(
    config: ProviderSessionConfig,
    cancelled: &AtomicBool,
    cancel_notify: &Notify,
) -> Result<PersistentProviderSession> {
    if cancelled.load(Ordering::Acquire) {
        return Err(Error::Preflight("provider session was cancelled".into()));
    }
    match config {
        ProviderSessionConfig::DeepseekHarness(config) => {
            Ok(PersistentProviderSession::DeepseekHarness(
                DeepseekHarnessSidecar::start(config).await?,
            ))
        }
        ProviderSessionConfig::Qoder(config) => Ok(PersistentProviderSession::Qoder(
            QoderSession::start_cancellable(config, cancelled, cancel_notify).await?,
        )),
    }
}

fn stable_message(error: &Error) -> String {
    match error {
        Error::Preflight(message) => message.clone(),
        _ => error.to_string(),
    }
}
