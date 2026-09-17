use super::{Attempt, Store, attempt_row};
use crate::error::Result;
impl Store {
    /// Read-only inventory; never assumes that an unobserved process has stopped.
    pub fn active_attempts(&self) -> Result<Vec<Attempt>> {
        let mut query=self.db.prepare("SELECT id,node_run_id,attempt_number,state,revision,output_json FROM attempts WHERE state IN ('starting','running') ORDER BY id")?;
        Ok(query
            .query_map([], attempt_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }
}
