use super::{Store, transitions::event};
use crate::error::{Error, Result};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowDelivery {
    pub id: String,
    pub run_id: String,
    pub sequence: i64,
    pub source_attempt_id: Option<String>,
    pub target: String,
    pub payload_artifact_id: Option<String>,
    pub state: String,
    pub attempt_id: Option<String>,
    pub routing: Option<Value>,
}

fn delivery(row: &rusqlite::Row<'_>) -> rusqlite::Result<FlowDelivery> {
    let routing: Option<String> = row.get(8)?;
    Ok(FlowDelivery {
        id: row.get(0)?,
        run_id: row.get(1)?,
        sequence: row.get(2)?,
        source_attempt_id: row.get(3)?,
        target: row.get(4)?,
        payload_artifact_id: row.get(5)?,
        state: row.get(6)?,
        attempt_id: row.get(7)?,
        routing: routing
            .map(|text| {
                serde_json::from_str(&text).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        8,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })
            })
            .transpose()?,
    })
}

const DELIVERY_COLUMNS: &str = "id,run_id,sequence,source_attempt_id,target_key,payload_artifact_id,state,attempt_id,routing_json";

impl Store {
    pub fn bind_flow_attempt(
        &mut self,
        delivery_id: &str,
        generation: i64,
        attempt_id: &str,
    ) -> Result<()> {
        let changed = self.db.execute(
            "UPDATE flow_deliveries SET attempt_id=?3 WHERE id=?1 AND state='running' AND attempt_id IS NULL
             AND EXISTS(SELECT 1 FROM runs r WHERE r.id=flow_deliveries.run_id AND r.state='running' AND r.owner_generation=?2)
             AND EXISTS(SELECT 1 FROM attempts a JOIN node_runs n ON n.id=a.node_run_id
                        WHERE a.id=?3 AND a.state IN ('starting','running') AND n.run_id=flow_deliveries.run_id AND n.node_key=flow_deliveries.target_key)",
            params![delivery_id, generation, attempt_id],
        )?;
        if changed != 1 {
            return Err(Error::Conflict);
        }
        Ok(())
    }

    pub fn flow_source_changeset(&self, delivery_id: &str) -> Result<Option<String>> {
        Ok(self.db.query_row(
            "SELECT a.changeset_artifact_id FROM flow_deliveries d LEFT JOIN attempts a ON a.id=d.source_attempt_id WHERE d.id=?1",
            [delivery_id], |row| row.get(0),
        )?)
    }

    pub fn fail_flow_run(&mut self, run: &str, generation: i64, code: &str) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx.execute(
            "UPDATE runs SET state='failed',revision=revision+1 WHERE id=?1 AND state='running' AND owner_generation=?2",
            params![run, generation],
        )?;
        if changed != 1 {
            return Err(Error::Conflict);
        }
        tx.execute(
            "UPDATE flow_deliveries SET state='failed' WHERE run_id=?1 AND state='running'",
            [run],
        )?;
        tx.execute("UPDATE flow_deliveries SET state='skipped_due_failure' WHERE run_id=?1 AND state='queued'", [run])?;
        event(
            &tx,
            run,
            "workflow.failed",
            None,
            None,
            json!({"error_code":code}),
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn flow_deliveries(&self, run: &str) -> Result<Vec<FlowDelivery>> {
        let mut query = self.db.prepare(&format!(
            "SELECT {DELIVERY_COLUMNS} FROM flow_deliveries WHERE run_id=?1 ORDER BY sequence"
        ))?;
        Ok(query
            .query_map([run], delivery)?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn enqueue_flow_entry(&mut self, run: &str, generation: i64, target: &str) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let active: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM runs WHERE id=?1 AND state='running' AND owner_generation=?2)", params![run,generation], |row| row.get(0))?;
        if !active {
            return Err(Error::Conflict);
        }
        let inserted = tx.execute("INSERT OR IGNORE INTO flow_deliveries(id,run_id,sequence,target_key,state) VALUES(?1,?2,1,?3,'queued')", params![uuid::Uuid::new_v4().to_string(),run,target])?;
        if inserted == 1 {
            event(
                &tx,
                run,
                "flow.delivery_queued",
                None,
                None,
                json!({"target":target,"sequence":1}),
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn claim_flow_delivery(
        &mut self,
        run: &str,
        generation: i64,
    ) -> Result<Option<FlowDelivery>> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let active: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM runs WHERE id=?1 AND state='running' AND owner_generation=?2)", params![run,generation], |row| row.get(0))?;
        if !active {
            return Err(Error::Conflict);
        }
        let running: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM flow_deliveries WHERE run_id=?1 AND state='running')",
            [run],
            |row| row.get(0),
        )?;
        if running {
            tx.commit()?;
            return Ok(None);
        }
        let id: Option<String> = tx.query_row("SELECT id FROM flow_deliveries WHERE run_id=?1 AND state='queued' ORDER BY sequence LIMIT 1", [run], |row| row.get(0)).optional()?;
        let Some(id) = id else {
            tx.commit()?;
            return Ok(None);
        };
        tx.execute(
            "UPDATE flow_deliveries SET state='running' WHERE id=?1 AND state='queued'",
            [&id],
        )?;
        let item = tx.query_row(
            &format!("SELECT {DELIVERY_COLUMNS} FROM flow_deliveries WHERE id=?1"),
            [&id],
            delivery,
        )?;
        event(
            &tx,
            run,
            "flow.delivery_claimed",
            None,
            None,
            json!({"delivery_id":id,"target":item.target,"sequence":item.sequence}),
        )?;
        tx.commit()?;
        Ok(Some(item))
    }

    pub fn prepare_flow_node(&mut self, run: &str, target: &str, generation: i64) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let active: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM runs WHERE id=?1 AND state='running' AND owner_generation=?2)", params![run,generation], |row| row.get(0))?;
        if !active {
            return Err(Error::Conflict);
        }
        tx.execute("UPDATE node_runs SET state='pending',revision=revision+1 WHERE run_id=?1 AND node_key=?2 AND state='succeeded'", params![run,target])?;
        tx.commit()?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn finish_flow_delivery(
        &mut self,
        delivery_id: &str,
        generation: i64,
        attempt_id: &str,
        result_artifact_id: &str,
        changeset_artifact_id: Option<&str>,
        output: &Value,
        thread_id: &str,
        next: &[String],
        complete: bool,
    ) -> Result<()> {
        if thread_id.trim().is_empty()
            || (complete && !next.is_empty())
            || (!complete && next.is_empty())
        {
            return Err(Error::Preflight("invalid flow routing decision".into()));
        }
        let unique: std::collections::BTreeSet<_> = next.iter().collect();
        if unique.len() != next.len() {
            return Err(Error::Preflight("duplicate flow target".into()));
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (run,target): (String,String) = tx.query_row(
            "SELECT d.run_id,d.target_key FROM flow_deliveries d JOIN runs r ON r.id=d.run_id WHERE d.id=?1 AND d.state='running' AND d.attempt_id=?3 AND r.state='running' AND r.owner_generation=?2",
            params![delivery_id,generation,attempt_id], |row| Ok((row.get(0)?,row.get(1)?)),
        ).map_err(|_| Error::Conflict)?;
        let (node,attempt_revision): (String,i64) = tx.query_row(
            "SELECT n.id,a.revision FROM attempts a JOIN node_runs n ON n.id=a.node_run_id WHERE a.id=?1 AND n.run_id=?2 AND n.node_key=?3 AND a.state='running' AND a.owner_generation=?4 AND n.state='running'",
            params![attempt_id,run,target,generation], |row| Ok((row.get(0)?,row.get(1)?)),
        ).map_err(|_| Error::Conflict)?;
        for next_target in next {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM node_runs WHERE run_id=?1 AND node_key=?2)",
                params![run, next_target],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(Error::Preflight(format!(
                    "unknown flow target: {next_target}"
                )));
            }
        }
        let changed = tx.execute("UPDATE attempts SET state='succeeded',revision=revision+1,output_json=?4,thread_id=?5,result_artifact_id=?6,changeset_artifact_id=?7
            WHERE id=?1 AND revision=?2 AND owner_generation=?3 AND state='running'
            AND EXISTS(SELECT 1 FROM artifacts WHERE id=?6 AND attempt_id=?1)
            AND (?7 IS NULL OR EXISTS(SELECT 1 FROM artifacts WHERE id=?7 AND attempt_id=?1))",
            params![attempt_id,attempt_revision,generation,output.to_string(),thread_id,result_artifact_id,changeset_artifact_id])?;
        if changed != 1 {
            return Err(Error::Conflict);
        }
        tx.execute("UPDATE node_runs SET state='succeeded',revision=revision+1 WHERE id=?1 AND state='running'", [&node])?;
        let routing = json!({"decision":if complete {"complete"} else {"continue"},"next":next});
        tx.execute("UPDATE flow_deliveries SET state='completed',attempt_id=?2,routing_json=?3 WHERE id=?1 AND state='running'", params![delivery_id,attempt_id,routing.to_string()])?;
        event(
            &tx,
            &run,
            "node.succeeded",
            Some(&node),
            Some(attempt_id),
            json!({"owner_generation":generation,"thread_id":thread_id,"result_artifact_id":result_artifact_id}),
        )?;
        event(
            &tx,
            &run,
            "flow.delivery_completed",
            Some(&node),
            Some(attempt_id),
            json!({"delivery_id":delivery_id,"routing":routing}),
        )?;
        if complete {
            tx.execute("UPDATE flow_deliveries SET state='skipped_due_terminal' WHERE run_id=?1 AND state='queued'", [&run])?;
            tx.execute("UPDATE runs SET state='completed',outcome='approved',revision=revision+1 WHERE id=?1 AND state='running' AND owner_generation=?2", params![run,generation])?;
            event(
                &tx,
                &run,
                "workflow.completed",
                None,
                None,
                json!({"outcome":"approved"}),
            )?;
        } else {
            let mut sequence: i64 = tx.query_row(
                "SELECT COALESCE(MAX(sequence),0) FROM flow_deliveries WHERE run_id=?1",
                [&run],
                |row| row.get(0),
            )?;
            for next_target in next {
                sequence += 1;
                let id = uuid::Uuid::new_v4().to_string();
                tx.execute("INSERT INTO flow_deliveries(id,run_id,sequence,source_attempt_id,target_key,payload_artifact_id,state) VALUES(?1,?2,?3,?4,?5,?6,'queued')",
                    params![id,run,sequence,attempt_id,next_target,result_artifact_id])?;
                event(
                    &tx,
                    &run,
                    "flow.delivery_queued",
                    None,
                    Some(attempt_id),
                    json!({"delivery_id":id,"target":next_target,"sequence":sequence}),
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}
