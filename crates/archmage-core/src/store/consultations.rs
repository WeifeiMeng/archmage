use super::{Store, transitions::event};
use crate::error::{Error, Result};
use crate::process::ProcessIdentity;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Consultation {
    pub id: String,
    pub run_id: String,
    pub source_node_id: String,
    pub source_attempt_id: String,
    pub target_capability: String,
    pub round: i64,
    pub decision: String,
    pub summary: String,
    pub suggestions: Vec<String>,
    pub result: serde_json::Value,
    pub result_sha256: String,
    pub thread_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsultationProcessSnapshot {
    pub source_node_id: String,
    pub source_attempt_id: String,
    pub target_capability: String,
}

impl Store {
    pub fn consultation_processes_for_run(
        &self,
        run: &str,
    ) -> Result<Vec<ConsultationProcessSnapshot>> {
        let mut query = self.db.prepare(
            "SELECT n.id,p.source_attempt_id,p.target_capability
            FROM consultation_processes p JOIN attempts a ON a.id=p.source_attempt_id
            JOIN node_runs n ON n.id=a.node_run_id
            WHERE p.run_id=?1 AND p.state='running'",
        )?;
        Ok(query
            .query_map([run], |row| {
                Ok(ConsultationProcessSnapshot {
                    source_node_id: row.get(0)?,
                    source_attempt_id: row.get(1)?,
                    target_capability: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn register_consultation_process(
        &mut self,
        id: &str,
        run: &str,
        attempt: &str,
        target: &str,
        generation: i64,
        identity: &ProcessIdentity,
    ) -> Result<()> {
        let changed = self.db.execute(
            "INSERT INTO consultation_processes(id,run_id,source_attempt_id,target_capability,process_pid,process_pgid,process_started,state)
             SELECT ?1,?2,?3,?4,?5,?6,?7,'running' FROM runs r JOIN attempts a ON a.id=?3
             JOIN node_runs n ON n.id=a.node_run_id
             WHERE r.id=?2 AND r.state='running' AND r.owner_generation=?8
               AND n.run_id=r.id AND n.state='succeeded' AND a.state='succeeded'
               AND NOT EXISTS(SELECT 1 FROM control_requests c WHERE c.run_id=r.id AND c.kind='cancel' AND c.state='requested')",
            params![id,run,attempt,target,identity.pid,identity.pgid,identity.started,generation],
        )?;
        if changed != 1 {
            return Err(Error::Conflict);
        }
        Ok(())
    }

    pub fn finish_consultation_process(&mut self, id: &str) -> Result<()> {
        self.db.execute(
            "UPDATE consultation_processes SET state='finished' WHERE id=?1 AND state='running'",
            [id],
        )?;
        Ok(())
    }

    pub fn active_consultation_processes(&self, run: &str) -> Result<Vec<ProcessIdentity>> {
        let mut query = self.db.prepare("SELECT process_pid,process_pgid,process_started FROM consultation_processes WHERE run_id=?1 AND state='running'")?;
        Ok(query
            .query_map([run], |row| {
                Ok(ProcessIdentity {
                    pid: row.get(0)?,
                    pgid: row.get(1)?,
                    started: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn consultations_for_run(&self, run: &str) -> Result<Vec<Consultation>> {
        let mut query = self.db.prepare(
            "SELECT source_attempt_id FROM consultations WHERE run_id=?1 ORDER BY rowid",
        )?;
        let attempts = query
            .query_map([run], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for attempt in attempts {
            if seen.insert(attempt.clone()) {
                result.extend(self.consultations_for_attempt(&attempt)?);
            }
        }
        Ok(result)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn record_consultation(
        &mut self,
        run: &str,
        source: &str,
        target: &str,
        attempt: &str,
        round: i64,
        generation: i64,
        decision: &str,
        summary: &str,
        suggestions: &[String],
        thread_id: &str,
        result: &serde_json::Value,
    ) -> Result<Consultation> {
        if !matches!(decision, "approved" | "changes_requested")
            || summary.trim().is_empty()
            || thread_id.trim().is_empty()
            || round < 0
            || suggestions.iter().any(|item| item.trim().is_empty())
            || (decision == "changes_requested" && suggestions.is_empty())
            || (decision == "approved" && !suggestions.is_empty())
            || result["decision"] != decision
            || result["summary"] != summary
            || result["suggestions"] != json!(suggestions)
        {
            return Err(Error::Preflight("invalid consultation decision".into()));
        }
        let result_bytes =
            serde_json::to_vec(result).map_err(|error| Error::Preflight(error.to_string()))?;
        let result_hash = format!("{:x}", Sha256::digest(&result_bytes));
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let node: Option<String> = tx.query_row(
            "SELECT n.id FROM attempts a JOIN node_runs n ON n.id=a.node_run_id JOIN runs r ON r.id=n.run_id
             WHERE a.id=?1 AND a.state='succeeded' AND n.state='succeeded' AND n.node_key=?2
               AND n.run_id=?3 AND r.state='running' AND r.owner_generation=?4
               AND a.attempt_number=(SELECT MAX(latest.attempt_number) FROM attempts latest WHERE latest.node_run_id=n.id)",
            params![attempt,source,run,generation], |row| row.get(0),
        ).optional()?;
        let node = node.ok_or(Error::Conflict)?;
        let id = uuid::Uuid::new_v4().to_string();
        tx.execute("INSERT INTO consultations(id,run_id,source_node_id,source_attempt_id,target_capability,round,decision,summary,suggestions_json,result_json,result_sha256,thread_id)
                    VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![id,run,node,attempt,target,round,decision,summary,serde_json::to_string(suggestions).map_err(|error| Error::Preflight(error.to_string()))?,String::from_utf8(result_bytes).map_err(|error| Error::Preflight(error.to_string()))?,result_hash,thread_id])?;
        event(
            &tx,
            run,
            "consultation.completed",
            Some(&node),
            Some(attempt),
            json!({"consultation_id":id,"target_capability":target,"round":round,"decision":decision,"summary":summary,"suggestions":suggestions,"result_sha256":result_hash}),
        )?;
        tx.commit()?;
        self.consultations_for_attempt(attempt)?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or(Error::Conflict)
    }

    pub fn consultations_for_attempt(&self, attempt: &str) -> Result<Vec<Consultation>> {
        let mut query = self.db.prepare("SELECT id,run_id,source_node_id,source_attempt_id,target_capability,round,decision,summary,suggestions_json,result_json,result_sha256,thread_id
            FROM consultations WHERE source_attempt_id=?1 ORDER BY target_capability")?;
        let rows = query
            .query_map([attempt], |row| {
                let suggestions: String = row.get(8)?;
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    suggestions,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                ))
            })?
            .collect::<rusqlite::Result<
                Vec<(
                    String,
                    String,
                    String,
                    String,
                    String,
                    i64,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                )>,
            >>()?;
        rows.into_iter()
            .map(
                |(
                    id,
                    run_id,
                    source_node_id,
                    source_attempt_id,
                    target_capability,
                    round,
                    decision,
                    summary,
                    suggestions,
                    result,
                    result_sha256,
                    thread_id,
                )| {
                    Ok(Consultation {
                        id,
                        run_id,
                        source_node_id,
                        source_attempt_id,
                        target_capability,
                        round,
                        decision,
                        summary,
                        suggestions: serde_json::from_str(&suggestions).map_err(|error| {
                            Error::Preflight(format!("invalid stored consultation: {error}"))
                        })?,
                        result: serde_json::from_str(&result).map_err(|error| {
                            Error::Preflight(format!("invalid stored consultation result: {error}"))
                        })?,
                        result_sha256,
                        thread_id,
                    })
                },
            )
            .collect()
    }

    pub fn reopen_node_for_feedback(
        &mut self,
        node: &str,
        generation: i64,
        suggestions: &[String],
    ) -> Result<()> {
        if suggestions.is_empty() || suggestions.iter().any(|item| item.trim().is_empty()) {
            return Err(Error::Preflight(
                "revision feedback must be actionable".into(),
            ));
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (run,source_attempt): (String,String) = tx.query_row(
            "SELECT n.run_id,a.id FROM node_runs n JOIN runs r ON r.id=n.run_id JOIN attempts a ON a.node_run_id=n.id
             WHERE n.id=?1 AND n.state='succeeded' AND r.state='running' AND r.owner_generation=?2
             AND NOT EXISTS(SELECT 1 FROM node_runs d WHERE d.run_id=n.run_id AND d.ordinal>n.ordinal AND d.state!='pending')
             AND a.state='succeeded' ORDER BY a.attempt_number DESC LIMIT 1",
            params![node,generation], |row| Ok((row.get(0)?,row.get(1)?)),
        ).map_err(|_| Error::Conflict)?;
        let changed = tx.execute("UPDATE node_runs SET state='pending',revision=revision+1 WHERE id=?1 AND state='succeeded'", [node])?;
        if changed != 1 {
            return Err(Error::Conflict);
        }
        tx.execute("INSERT INTO revision_feedback(id,node_id,source_attempt_id,suggestions_json) VALUES(?1,?2,?3,?4)",
            params![uuid::Uuid::new_v4().to_string(),node,source_attempt,serde_json::to_string(suggestions).map_err(|error| Error::Preflight(error.to_string()))?])?;
        event(
            &tx,
            &run,
            "node.revision_requested",
            Some(node),
            Some(&source_attempt),
            json!({"suggestions":suggestions}),
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn feedback_for_node(&self, node: &str) -> Result<Vec<String>> {
        let text: Option<String> = self.db.query_row(
            "SELECT suggestions_json FROM revision_feedback WHERE node_id=?1 ORDER BY rowid DESC LIMIT 1",
            [node], |row| row.get(0),
        ).optional()?;
        text.map(|value| {
            serde_json::from_str(&value).map_err(|error| Error::Preflight(error.to_string()))
        })
        .unwrap_or_else(|| Ok(Vec::new()))
    }

    pub fn previous_output_for_revision(&self, node: &str) -> Result<Option<serde_json::Value>> {
        let text: Option<String> = self.db.query_row(
            "SELECT a.output_json FROM attempts a WHERE a.node_run_id=?1 AND a.state='succeeded'
             AND EXISTS(SELECT 1 FROM revision_feedback f WHERE f.node_id=?1)
             ORDER BY a.attempt_number DESC LIMIT 1",
            [node], |row| row.get(0),
        ).optional()?.flatten();
        text.map(|value| {
            serde_json::from_str(&value)
                .map_err(|error| Error::Preflight(format!("invalid previous output: {error}")))
        })
        .transpose()
    }

    pub fn reopen_after_reviewer(
        &mut self,
        run: &str,
        generation: i64,
        reviewer_attempt: &str,
        suggestions: &[String],
    ) -> Result<()> {
        if suggestions.is_empty() || suggestions.iter().any(|item| item.trim().is_empty()) {
            return Err(Error::Preflight(
                "reviewer feedback must be actionable".into(),
            ));
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let valid: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM runs r JOIN node_runs n ON n.run_id=r.id
                 JOIN attempts a ON a.node_run_id=n.id
                 WHERE r.id=?1 AND r.state='running' AND r.owner_generation=?2
                 AND n.node_key='reviewer' AND n.state='succeeded' AND a.id=?3 AND a.state='succeeded'
                 AND NOT EXISTS(SELECT 1 FROM node_runs d WHERE d.run_id=r.id AND d.state!='succeeded'))",
            params![run,generation,reviewer_attempt], |row| row.get(0),
        )?;
        if !valid {
            return Err(Error::Conflict);
        }
        let target: (String, i64) = tx.query_row(
            "SELECT id,ordinal FROM node_runs WHERE run_id=?1 AND node_key!='reviewer' ORDER BY CASE WHEN node_key='coder' THEN 0 ELSE 1 END, ordinal DESC LIMIT 1",
            [run],
            |row| Ok((row.get(0)?,row.get(1)?)),
        )?;
        tx.execute("UPDATE node_runs SET state='pending',revision=revision+1 WHERE run_id=?1 AND ordinal>=?2 AND state='succeeded'", params![run,target.1])?;
        tx.execute("INSERT INTO revision_feedback(id,node_id,source_attempt_id,suggestions_json) VALUES(?1,?2,?3,?4)",
            params![uuid::Uuid::new_v4().to_string(),target.0,reviewer_attempt,serde_json::to_string(suggestions).map_err(|error| Error::Preflight(error.to_string()))?])?;
        event(
            &tx,
            run,
            "workflow.revision_requested",
            Some(&target.0),
            Some(reviewer_attempt),
            json!({"suggestions":suggestions}),
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn fail_review_limit(&mut self, node: &str, generation: i64) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let run: String = tx
            .query_row(
                "SELECT n.run_id FROM node_runs n JOIN runs r ON r.id=n.run_id
            WHERE n.id=?1 AND n.state='succeeded' AND r.state='running' AND r.owner_generation=?2",
                params![node, generation],
                |row| row.get(0),
            )
            .map_err(|_| Error::Conflict)?;
        tx.execute(
            "UPDATE node_runs SET state='failed',revision=revision+1 WHERE id=?1",
            [node],
        )?;
        event(
            &tx,
            &run,
            "node.review_limit_reached",
            Some(node),
            None,
            json!({"limit":3}),
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn fail_consultation_node(
        &mut self,
        node: &str,
        generation: i64,
        reason: &str,
    ) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let run: String = tx
            .query_row(
                "SELECT n.run_id FROM node_runs n JOIN runs r ON r.id=n.run_id
            WHERE n.id=?1 AND n.state='succeeded' AND r.state='running' AND r.owner_generation=?2",
                params![node, generation],
                |row| row.get(0),
            )
            .map_err(|_| Error::Conflict)?;
        tx.execute(
            "UPDATE node_runs SET state='failed',revision=revision+1 WHERE id=?1",
            [node],
        )?;
        event(
            &tx,
            &run,
            "consultation.failed",
            Some(node),
            None,
            json!({"reason":reason}),
        )?;
        tx.commit()?;
        Ok(())
    }
}
