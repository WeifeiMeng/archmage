use super::Runtime;
use crate::{
    error::Result,
    process::{cancel_process, identity_is_alive},
};
use std::time::Duration;

impl Runtime {
    pub async fn cancel(&self, run_id: &str) -> Result<()> {
        let mut store = self.store()?;
        store.request_cancel(run_id)?;
        if let Some(process) = store.active_process_for_run(run_id)? {
            cancel_process(&process.identity, Duration::from_secs(5)).await?;
        }
        store.apply_cancel(run_id)
    }

    pub fn reconcile_interrupted(&self) -> Result<()> {
        let mut store = self.store()?;
        for attempt in store.active_attempts()? {
            let run_id = store.run_id_for_attempt(&attempt.id)?;
            let Ok(_owner) = super::control::RunOwnerLock::acquire(
                &self.config.data_root.join("locks"),
                &run_id,
            ) else {
                continue;
            };
            let alive = store
                .process_for_attempt(&attempt.id)?
                .is_some_and(|process| identity_is_alive(&process.identity));
            if !alive {
                store.interrupt_attempt(&attempt.id, attempt.revision)?;
            }
        }
        Ok(())
    }
}
