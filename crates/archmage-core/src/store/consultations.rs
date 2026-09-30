use super::{Store, transitions::event};
use crate::error::{Error, Result};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::json;

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
    pub thread_id: String,
}

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub fn record_consultation(&mut self, run: &str, source: &str, target: &str, attempt: &str,
        round: i64, generation: i64, decision: &str, summary: &str,
        suggestions: &[String], thread_id: &str) -> Result<Consultation> {
        if !matches!(decision, "approved" | "changes_requested") || summary.trim().is_empty()
            || thread_id.trim().is_empty() || round < 0
            || suggestions.iter().any(|item| item.trim().is_empty())
            || (decision == "changes_requested" && suggestions.is_empty())
            || (decision == "approved" && !suggestions.is_empty()) {
            return Err(Error::Preflight("invalid consultation decision".into()));
        }
        let tx = self.db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let node: Option<String> = tx.query_row(
            "SELECT n.id FROM attempts a JOIN node_runs n ON n.id=a.node_run_id JOIN runs r ON r.id=n.run_id
             WHERE a.id=?1 AND a.state='succeeded' AND n.state='succeeded' AND n.node_key=?2
               AND n.run_id=?3 AND r.state='running' AND r.owner_generation=?4
               AND a.attempt_number=(SELECT MAX(latest.attempt_number) FROM attempts latest WHERE latest.node_run_id=n.id)",
            params![attempt,source,run,generation], |row| row.get(0),
        ).optional()?;
        let node = node.ok_or(Error::Conflict)?;
        let id = uuid::Uuid::new_v4().to_string();
        tx.execute("INSERT INTO consultations(id,run_id,source_node_id,source_attempt_id,target_capability,round,decision,summary,suggestions_json,thread_id)
                    VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![id,run,node,attempt,target,round,decision,summary,serde_json::to_string(suggestions).map_err(|error| Error::Preflight(error.to_string()))?,thread_id])?;
        event(&tx, run, "consultation.completed", Some(&node), Some(attempt),
            json!({"consultation_id":id,"target_capability":target,"round":round,"decision":decision,"summary":summary,"suggestions":suggestions}))?;
        tx.commit()?;
        self.consultations_for_attempt(attempt)?.into_iter().find(|item| item.id == id).ok_or(Error::Conflict)
    }

    pub fn consultations_for_attempt(&self, attempt: &str) -> Result<Vec<Consultation>> {
        let mut query = self.db.prepare("SELECT id,run_id,source_node_id,source_attempt_id,target_capability,round,decision,summary,suggestions_json,thread_id
            FROM consultations WHERE source_attempt_id=?1 ORDER BY target_capability")?;
        let rows = query.query_map([attempt], |row| {
            let suggestions: String = row.get(8)?;
            Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?,suggestions,row.get(9)?))
        })?.collect::<rusqlite::Result<Vec<(String,String,String,String,String,i64,String,String,String,String)>>>()?;
        rows.into_iter().map(|(id,run_id,source_node_id,source_attempt_id,target_capability,round,decision,summary,suggestions,thread_id)| {
            Ok(Consultation { id,run_id,source_node_id,source_attempt_id,target_capability,round,decision,summary,
                suggestions: serde_json::from_str(&suggestions).map_err(|error| Error::Preflight(format!("invalid stored consultation: {error}")))?,thread_id })
        }).collect()
    }

    pub fn reopen_node_for_feedback(&mut self, node: &str, generation: i64, suggestions: &[String]) -> Result<()> {
        if suggestions.is_empty() || suggestions.iter().any(|item| item.trim().is_empty()) {
            return Err(Error::Preflight("revision feedback must be actionable".into()));
        }
        let tx = self.db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (run,source_attempt): (String,String) = tx.query_row(
            "SELECT n.run_id,a.id FROM node_runs n JOIN runs r ON r.id=n.run_id JOIN attempts a ON a.node_run_id=n.id
             WHERE n.id=?1 AND n.state='succeeded' AND r.state='running' AND r.owner_generation=?2
             AND NOT EXISTS(SELECT 1 FROM node_runs d WHERE d.run_id=n.run_id AND d.ordinal>n.ordinal AND d.state!='pending')
             AND a.state='succeeded' ORDER BY a.attempt_number DESC LIMIT 1",
            params![node,generation], |row| Ok((row.get(0)?,row.get(1)?)),
        ).map_err(|_| Error::Conflict)?;
        let changed = tx.execute("UPDATE node_runs SET state='pending',revision=revision+1 WHERE id=?1 AND state='succeeded'", [node])?;
        if changed != 1 { return Err(Error::Conflict); }
        tx.execute("INSERT INTO revision_feedback(id,node_id,source_attempt_id,suggestions_json) VALUES(?1,?2,?3,?4)",
            params![uuid::Uuid::new_v4().to_string(),node,source_attempt,serde_json::to_string(suggestions).map_err(|error| Error::Preflight(error.to_string()))?])?;
        event(&tx, &run, "node.revision_requested", Some(node), Some(&source_attempt), json!({"suggestions":suggestions}))?;
        tx.commit()?;
        Ok(())
    }

    pub fn feedback_for_node(&self, node: &str) -> Result<Vec<String>> {
        let text: Option<String> = self.db.query_row(
            "SELECT suggestions_json FROM revision_feedback WHERE node_id=?1 ORDER BY rowid DESC LIMIT 1",
            [node], |row| row.get(0),
        ).optional()?;
        text.map(|value| serde_json::from_str(&value).map_err(|error| Error::Preflight(error.to_string())))
            .unwrap_or_else(|| Ok(Vec::new()))
    }
}
