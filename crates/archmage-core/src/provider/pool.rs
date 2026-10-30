use crate::{
    error::{Error, Result},
    process::ProcessIdentity,
    provider::{
        deepseek_harness::{DeepseekHarnessSidecar, SidecarConfig},
        executor::ProviderOutcome,
    },
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

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

enum PoolEntry {
    Live(DeepseekHarnessSidecar),
    Poisoned(String),
    Closing,
}

pub struct ProviderPool {
    entries: Mutex<HashMap<SidecarKey, Arc<Mutex<PoolEntry>>>>,
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
        mut config: SidecarConfig,
        invocation_id: &str,
        task: &[u8],
        schema: &serde_json::Value,
        on_spawn: impl FnOnce(&ProcessIdentity) -> Result<()>,
    ) -> Result<ProviderOutcome> {
        let entry = {
            let mut entries = self.entries.lock().await;
            entries
                .entry(key)
                .or_insert_with(|| Arc::new(Mutex::new(PoolEntry::Closing)))
                .clone()
        };
        let mut guard = entry.lock().await;
        if matches!(*guard, PoolEntry::Closing) {
            config.session_id = format!("archmage-{}", uuid::Uuid::new_v4());
            match DeepseekHarnessSidecar::start(config).await {
                Ok(sidecar) => *guard = PoolEntry::Live(sidecar),
                Err(error) => {
                    let message = stable_message(&error);
                    *guard = PoolEntry::Poisoned(message.clone());
                    return Err(Error::Preflight(message));
                }
            }
        }
        match &mut *guard {
            PoolEntry::Live(sidecar) => {
                on_spawn(sidecar.identity())?;
                sidecar.invoke(invocation_id, task, schema).await
            }
            PoolEntry::Poisoned(message) => Err(Error::Preflight(message.clone())),
            PoolEntry::Closing => Err(Error::Conflict),
        }
    }

    pub async fn invoke_temporary(
        &self,
        mut config: SidecarConfig,
        invocation_id: &str,
        task: &[u8],
        schema: &serde_json::Value,
        on_spawn: impl FnOnce(&ProcessIdentity) -> Result<()>,
    ) -> Result<ProviderOutcome> {
        config.session_id = format!("archmage-consult-{}", uuid::Uuid::new_v4());
        let mut sidecar = DeepseekHarnessSidecar::start(config).await?;
        on_spawn(sidecar.identity())?;
        match sidecar.invoke(invocation_id, task, schema).await {
            Ok(outcome) => {
                sidecar.close().await?;
                Ok(outcome)
            }
            Err(error) => {
                let _ = sidecar.cancel().await;
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
            let mut guard = entry.lock().await;
            if let PoolEntry::Live(sidecar) = std::mem::replace(&mut *guard, PoolEntry::Closing)
                && let Err(error) = sidecar.close().await
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
        let mut guard = entry.lock().await;
        if let PoolEntry::Live(sidecar) = std::mem::replace(
            &mut *guard,
            PoolEntry::Poisoned("DeepSeek Harness sidecar was cancelled".into()),
        ) {
            sidecar.cancel().await?;
        }
        Ok(())
    }
}

fn stable_message(error: &Error) -> String {
    match error {
        Error::Preflight(message) => message.clone(),
        _ => error.to_string(),
    }
}
