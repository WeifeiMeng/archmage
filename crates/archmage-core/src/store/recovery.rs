use super::{Attempt, Store, attempt_row};
use crate::{error::Result, process::ProcessIdentity};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

#[derive(Clone, Debug)]
pub struct ActiveProcess {
    pub attempt_id: String,
    pub identity: ProcessIdentity,
}
impl Store {
    pub fn running_run_ids(&self) -> Result<Vec<String>> {
        let mut query = self
            .db
            .prepare("SELECT id FROM runs WHERE state='running' ORDER BY created_at,id")?;
        Ok(query
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn interrupt_orphaned_run(&mut self, run: &str) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx.execute("UPDATE runs SET state='interrupted',revision=revision+1 WHERE id=?1 AND state='running'", [run])?;
        if changed == 0 {
            tx.commit()?;
            return Ok(());
        }
        tx.execute("UPDATE attempts SET state='interrupted',revision=revision+1,error_code='interrupted'
            WHERE node_run_id IN (SELECT id FROM node_runs WHERE run_id=?1) AND state IN ('starting','running')", [run])?;
        tx.execute("UPDATE node_runs SET state='interrupted',revision=revision+1 WHERE run_id=?1 AND state IN ('ready','running')", [run])?;
        tx.execute("UPDATE consultation_processes SET state='cancelled' WHERE run_id=?1 AND state='running'", [run])?;
        super::transitions::event(
            &tx,
            run,
            "workflow.interrupted",
            None,
            None,
            serde_json::json!({"reason":"owner_lost"}),
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Read-only inventory; never assumes that an unobserved process has stopped.
    pub fn active_attempts(&self) -> Result<Vec<Attempt>> {
        let mut query=self.db.prepare("SELECT id,node_run_id,attempt_number,state,revision,output_json FROM attempts WHERE state IN ('starting','running') ORDER BY id")?;
        Ok(query
            .query_map([], attempt_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn active_process_for_run(&self, run: &str) -> Result<Option<ActiveProcess>> {
        Ok(self
            .db
            .query_row(
                "SELECT a.id,a.process_pid,a.process_pgid,a.process_started
             FROM attempts a JOIN node_runs n ON n.id=a.node_run_id
             WHERE n.run_id=?1 AND a.state='running' AND a.process_pid IS NOT NULL
             ORDER BY a.attempt_number DESC LIMIT 1",
                [run],
                |row| {
                    Ok(ActiveProcess {
                        attempt_id: row.get(0)?,
                        identity: ProcessIdentity {
                            pid: row.get(1)?,
                            pgid: row.get(2)?,
                            started: row.get(3)?,
                        },
                    })
                },
            )
            .optional()?)
    }

    pub fn process_for_attempt(&self, attempt: &str) -> Result<Option<ActiveProcess>> {
        Ok(self
            .db
            .query_row(
                "SELECT id,process_pid,process_pgid,process_started FROM attempts
             WHERE id=?1 AND state='running' AND process_pid IS NOT NULL",
                [attempt],
                |row| {
                    Ok(ActiveProcess {
                        attempt_id: row.get(0)?,
                        identity: ProcessIdentity {
                            pid: row.get(1)?,
                            pgid: row.get(2)?,
                            started: row.get(3)?,
                        },
                    })
                },
            )
            .optional()?)
    }

    pub fn run_id_for_attempt(&self, attempt: &str) -> Result<String> {
        Ok(self.db.query_row(
            "SELECT n.run_id FROM attempts a JOIN node_runs n ON n.id=a.node_run_id WHERE a.id=?1",
            [attempt],
            |row| row.get(0),
        )?)
    }

    pub fn request_cancel(&mut self, run: &str) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO control_requests(id,run_id,kind,state)
             SELECT ?2,?1,'cancel','requested' WHERE EXISTS(
               SELECT 1 FROM runs WHERE id=?1 AND state IN ('created','provisioning','running','interrupted')
             )",
            params![run, uuid::Uuid::new_v4().to_string()],
        )?;
        if inserted == 1 {
            super::transitions::event(
                &tx,
                run,
                "cancel.requested",
                None,
                None,
                serde_json::json!({}),
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn apply_cancel(&mut self, run: &str) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx.execute(
            "UPDATE runs SET state='cancelled',revision=revision+1 WHERE id=?1 AND state IN ('created','provisioning','running','interrupted')",
            [run],
        )?;
        if changed == 0 {
            tx.commit()?;
            return Ok(());
        }
        tx.execute(
            "UPDATE attempts SET state='cancelled',revision=revision+1,error_code='cancelled'
             WHERE state IN ('starting','running') AND node_run_id IN (SELECT id FROM node_runs WHERE run_id=?1)",
            [run],
        )?;
        tx.execute("UPDATE consultation_processes SET state='cancelled' WHERE run_id=?1 AND state='running'", [run])?;
        tx.execute(
            "UPDATE node_runs SET state='cancelled',revision=revision+1
             WHERE run_id=?1 AND state IN ('pending','ready','running','interrupted')",
            [run],
        )?;
        tx.execute(
            "UPDATE control_requests SET state='applied',applied_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE run_id=?1 AND kind='cancel' AND state='requested'",
            [run],
        )?;
        super::transitions::event(
            &tx,
            run,
            "workflow.cancelled",
            None,
            None,
            serde_json::json!({}),
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn interrupt_attempt(&mut self, attempt: &str, revision: i64) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (node, run): (String, String) = tx.query_row(
            "SELECT n.id,n.run_id FROM attempts a JOIN node_runs n ON n.id=a.node_run_id WHERE a.id=?1",
            [attempt],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let changed = tx.execute(
            "UPDATE attempts SET state='interrupted',revision=revision+1,error_code='interrupted'
             WHERE id=?1 AND revision=?2 AND state IN ('starting','running')",
            params![attempt, revision],
        )?;
        if changed == 0 {
            tx.commit()?;
            return Ok(());
        }
        tx.execute(
            "UPDATE node_runs SET state='interrupted',revision=revision+1 WHERE id=?1 AND state='running'",
            [&node],
        )?;
        tx.execute(
            "UPDATE runs SET state='interrupted',revision=revision+1 WHERE id=?1 AND state='running'",
            [&run],
        )?;
        super::transitions::event(
            &tx,
            &run,
            "workflow.interrupted",
            Some(&node),
            Some(attempt),
            serde_json::json!({}),
        )?;
        tx.commit()?;
        Ok(())
    }
}
