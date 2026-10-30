use super::Runtime;
use crate::{
    error::Result,
    process::{cancel_process, cancel_process_blocking},
};
use std::time::Duration;

impl Runtime {
    pub async fn cancel(&self, run_id: &str) -> Result<()> {
        let mut store = self.store()?;
        store.request_cancel(run_id)?;
        drop(store);
        self.provider_pool.cancel_run(run_id).await?;
        let mut store = self.store()?;
        if let Some(process) = store.active_process_for_run(run_id)? {
            cancel_process(&process.identity, Duration::from_secs(5)).await?;
        }
        for identity in store.active_consultation_processes(run_id)? {
            cancel_process(&identity, Duration::from_secs(5)).await?;
        }
        store.apply_cancel(run_id)
    }

    pub fn reconcile_interrupted(&self) -> Result<()> {
        let mut store = self.store()?;
        for run_id in store.running_run_ids()? {
            let Ok(_owner) = super::control::RunOwnerLock::acquire(
                &self.config.data_root.join("locks"),
                &run_id,
            ) else {
                continue;
            };
            if let Some(process) = store.active_process_for_run(&run_id)? {
                cancel_process_blocking(&process.identity, Duration::from_secs(5))?;
            }
            for identity in store.active_consultation_processes(&run_id)? {
                cancel_process_blocking(&identity, Duration::from_secs(5))?;
            }
            store.interrupt_orphaned_run(&run_id)?;
        }
        Ok(())
    }
}
