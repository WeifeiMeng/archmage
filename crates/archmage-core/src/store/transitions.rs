use super::{Attempt, AttemptOutcome, Store};
use crate::context::ContextSnapshot;
use crate::error::{Error, Result};
use crate::process::ProcessIdentity;
use rusqlite::{Transaction, TransactionBehavior, params};
use serde_json::{Value, json};

fn changed(rows: usize) -> Result<()> {
    if rows == 1 {
        Ok(())
    } else {
        Err(Error::Conflict)
    }
}
pub(crate) fn event(
    tx: &Transaction<'_>,
    run: &str,
    kind: &str,
    node: Option<&str>,
    attempt: Option<&str>,
    payload: Value,
) -> Result<()> {
    tx.execute(
        "INSERT INTO events(run_id,event_seq,event_type,node_id,attempt_id,payload_json)
        SELECT ?1,COALESCE(MAX(event_seq),0)+1,?2,?3,?4,?5 FROM events WHERE run_id=?1",
        params![run, kind, node, attempt, payload.to_string()],
    )?;
    Ok(())
}
impl Store {
    pub fn persist_context(
        &mut self,
        run: &str,
        node: &str,
        snapshot: &ContextSnapshot,
    ) -> Result<String> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let belongs: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM node_runs WHERE id=?1 AND run_id=?2)",
            params![node, run],
            |r| r.get(0),
        )?;
        if !belongs {
            return Err(Error::Conflict);
        }
        let id = uuid::Uuid::new_v4().to_string();
        tx.execute("INSERT INTO contexts(id,run_id,consumer_node_id,content_hash,prompt_version,rendered_bytes,manifest_json) VALUES(?1,?2,?3,?4,?5,?6,?7)", params![id,run,node,snapshot.hash,snapshot.prompt_version,snapshot.rendered,snapshot.manifest.to_string()])?;
        for handoff in &snapshot.handoffs {
            let valid: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM artifacts f JOIN attempts a ON a.id=f.attempt_id WHERE f.id=?1 AND f.attempt_id=?2 AND f.content_hash=?3 AND a.state='succeeded')",params![handoff.artifact_id,handoff.producer_attempt_id,handoff.content_hash],|r|r.get(0))?;
            if !valid {
                return Err(Error::Conflict);
            }
            tx.execute("INSERT INTO handoffs(id,context_id,producer_attempt_id,artifact_id,consumer_node_id,input_key,content_hash) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![uuid::Uuid::new_v4().to_string(),id,handoff.producer_attempt_id,handoff.artifact_id,node,handoff.input_key,handoff.content_hash])?;
        }
        event(
            &tx,
            run,
            "context.created",
            Some(node),
            None,
            json!({"id":id,"sha256":snapshot.hash}),
        )?;
        tx.commit()?;
        Ok(id)
    }

    pub fn bind_attempt_context(&mut self, attempt: &str, context: &str) -> Result<()> {
        changed(self.db.execute("UPDATE attempts SET context_id=?2 WHERE id=?1 AND state='starting' AND context_id IS NULL AND EXISTS(SELECT 1 FROM contexts c WHERE c.id=?2 AND c.consumer_node_id=attempts.node_run_id)",params![attempt,context])?)
    }

    pub fn claim_run_owner(&mut self, run: &str) -> Result<i64> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        changed(tx.execute(
            "UPDATE runs SET owner_generation=owner_generation+1,revision=revision+1 WHERE id=?1 AND state='running'",
            [run],
        )?)?;
        let generation: i64 = tx.query_row(
            "SELECT owner_generation FROM runs WHERE id=?1",
            [run],
            |r| r.get(0),
        )?;
        event(
            &tx,
            run,
            "owner.claimed",
            None,
            None,
            json!({"generation":generation}),
        )?;
        tx.commit()?;
        Ok(generation)
    }

    pub fn attach_attempt_process(
        &mut self,
        id: &str,
        revision: i64,
        generation: i64,
        identity: &ProcessIdentity,
    ) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        changed(tx.execute(
            "UPDATE attempts SET state='running',revision=revision+1,owner_generation=?3,process_pid=?4,process_pgid=?5,process_started=?6
             WHERE id=?1 AND revision=?2 AND state='starting'
             AND EXISTS(SELECT 1 FROM node_runs n JOIN runs r ON r.id=n.run_id
                        WHERE n.id=attempts.node_run_id AND r.owner_generation=?3 AND r.state='running')",
            params![id, revision, generation, identity.pid, identity.pgid, identity.started],
        )?)?;
        let (node,run):(String,String)=tx.query_row("SELECT n.id,n.run_id FROM attempts a JOIN node_runs n ON n.id=a.node_run_id WHERE a.id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?)))?;
        event(
            &tx,
            &run,
            "node.started",
            Some(&node),
            Some(id),
            json!({"owner_generation":generation,"pid":identity.pid,"pgid":identity.pgid}),
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn finish_attempt_owned(
        &mut self,
        id: &str,
        revision: i64,
        generation: i64,
        outcome: AttemptOutcome,
    ) -> Result<()> {
        let (state, output, error) = outcome_values(outcome);
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        changed(tx.execute(
            "UPDATE attempts SET state=?4,revision=revision+1,output_json=?5,error_code=?6
             WHERE id=?1 AND revision=?2 AND state='running' AND owner_generation=?3
             AND EXISTS(SELECT 1 FROM node_runs n JOIN runs r ON r.id=n.run_id
                        WHERE n.id=attempts.node_run_id AND r.owner_generation=?3 AND r.state='running')",
            params![id,revision,generation,state,output,error],
        )?)?;
        let (node,run):(String,String)=tx.query_row("SELECT n.id,n.run_id FROM attempts a JOIN node_runs n ON n.id=a.node_run_id WHERE a.id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?)))?;
        changed(tx.execute(
            "UPDATE node_runs SET state=?2,revision=revision+1 WHERE id=?1 AND state='running'",
            params![node, state],
        )?)?;
        event(
            &tx,
            &run,
            &format!("node.{state}"),
            Some(&node),
            Some(id),
            json!({"error_code":error,"owner_generation":generation}),
        )?;
        tx.commit()?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn finish_attempt_owned_with_result(
        &mut self,
        id: &str,
        revision: i64,
        generation: i64,
        output: Value,
        thread_id: &str,
        result_artifact_id: &str,
        changeset_artifact_id: Option<&str>,
    ) -> Result<()> {
        if thread_id.trim().is_empty() {
            return Err(Error::Preflight("Codex thread ID is blank".into()));
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        changed(tx.execute(
            "UPDATE attempts
             SET state='succeeded',revision=revision+1,output_json=?4,thread_id=?5,
                 result_artifact_id=?6,changeset_artifact_id=?7
             WHERE id=?1 AND revision=?2 AND state='running' AND owner_generation=?3
             AND EXISTS(SELECT 1 FROM artifacts WHERE id=?6 AND attempt_id=?1)
             AND (?7 IS NULL OR EXISTS(SELECT 1 FROM artifacts WHERE id=?7 AND attempt_id=?1))
             AND EXISTS(SELECT 1 FROM node_runs n JOIN runs r ON r.id=n.run_id
                        WHERE n.id=attempts.node_run_id AND r.owner_generation=?3 AND r.state='running')",
            params![
                id,
                revision,
                generation,
                output.to_string(),
                thread_id,
                result_artifact_id,
                changeset_artifact_id
            ],
        )?)?;
        let (node, run): (String, String) = tx.query_row(
            "SELECT n.id,n.run_id FROM attempts a JOIN node_runs n ON n.id=a.node_run_id WHERE a.id=?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        changed(tx.execute(
            "UPDATE node_runs SET state='succeeded',revision=revision+1 WHERE id=?1 AND state='running'",
            [&node],
        )?)?;
        event(
            &tx,
            &run,
            "node.succeeded",
            Some(&node),
            Some(id),
            json!({"owner_generation":generation,"thread_id":thread_id,"result_artifact_id":result_artifact_id}),
        )?;
        tx.commit()?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_workspace_intent(
        &mut self,
        run: &str,
        source_root: &str,
        path: &str,
        branch: &str,
        baseline: &str,
        common_dir: &str,
    ) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        changed(tx.execute(
            "UPDATE runs SET state='provisioning',revision=revision+1 WHERE id=?1 AND state='created'",
            [run],
        )?)?;
        tx.execute(
            "INSERT INTO workspaces(id,run_id,source_root,path,branch,baseline,common_dir,state)
             VALUES(?1,?2,?3,?4,?5,?6,?7,'provisioning')",
            params![
                uuid::Uuid::new_v4().to_string(),
                run,
                source_root,
                path,
                branch,
                baseline,
                common_dir
            ],
        )?;
        event(
            &tx,
            run,
            "workspace.provisioning",
            None,
            None,
            json!({"path":path,"branch":branch,"baseline":baseline}),
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn complete_workspace(&mut self, run: &str) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        changed(tx.execute(
            "UPDATE workspaces SET state='ready',revision=revision+1 WHERE run_id=?1 AND state='provisioning'",
            [run],
        )?)?;
        changed(tx.execute(
            "UPDATE runs SET state='running',revision=revision+1 WHERE id=?1 AND state='provisioning'",
            [run],
        )?)?;
        event(&tx, run, "workspace.created", None, None, json!({}))?;
        event(&tx, run, "workflow.started", None, None, json!({}))?;
        tx.commit()?;
        Ok(())
    }

    pub fn create_run(&mut self, goal: &str, nodes: &[&str]) -> Result<String> {
        self.create_run_definition(goal, nodes, json!({"version":1,"nodes":nodes}))
    }

    pub(crate) fn create_run_definition(
        &mut self,
        goal: &str,
        nodes: &[&str],
        definition: Value,
    ) -> Result<String> {
        if goal.trim().is_empty() || nodes.is_empty() || nodes.iter().any(|s| s.trim().is_empty()) {
            return Err(Error::Preflight(
                "goal and node keys must be nonempty".into(),
            ));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO runs(id,goal,state,definition_json) VALUES(?1,?2,'created',?3)",
            params![id, goal, definition.to_string()],
        )?;
        for (ordinal, key) in nodes.iter().enumerate() {
            tx.execute("INSERT INTO node_runs(id,run_id,node_key,ordinal,state) VALUES(?1,?2,?3,?4,'pending')",
                params![uuid::Uuid::new_v4().to_string(),id,key,ordinal as i64])?;
        }
        event(&tx, &id, "workflow.created", None, None, json!({}))?;
        tx.commit()?;
        Ok(id)
    }
    pub fn start_run(&mut self, id: &str, revision: i64) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        changed(tx.execute("UPDATE runs SET state='running',revision=revision+1 WHERE id=?1 AND revision=?2 AND state='created'",params![id,revision])?)?;
        event(&tx, id, "workflow.started", None, None, json!({}))?;
        tx.commit()?;
        Ok(())
    }

    pub fn complete_run(&mut self, id: &str, revision: i64) -> Result<()> {
        self.complete_run_with_outcome(id, revision, "approved")
    }

    pub fn complete_run_with_outcome(
        &mut self,
        id: &str,
        revision: i64,
        outcome: &str,
    ) -> Result<()> {
        if !matches!(outcome, "approved" | "changes_requested") {
            return Err(Error::Preflight("invalid completion outcome".into()));
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        changed(tx.execute(
            "UPDATE runs SET state='completed',outcome=?3,revision=revision+1
             WHERE id=?1 AND revision=?2 AND state='running'
             AND NOT EXISTS(SELECT 1 FROM node_runs WHERE run_id=?1 AND state!='succeeded')",
            params![id, revision, outcome],
        )?)?;
        event(
            &tx,
            id,
            "workflow.completed",
            None,
            None,
            json!({"outcome":outcome}),
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn fail_run(&mut self, id: &str, revision: i64, code: &str) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        changed(tx.execute(
            "UPDATE runs SET state='failed',revision=revision+1
             WHERE id=?1 AND revision=?2 AND state='running'
             AND EXISTS(SELECT 1 FROM node_runs WHERE run_id=?1 AND state='failed')",
            params![id, revision],
        )?)?;
        event(
            &tx,
            id,
            "workflow.failed",
            None,
            None,
            json!({"error_code":code}),
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn ready_node(&mut self, id: &str, revision: i64) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        changed(tx.execute("UPDATE node_runs SET state='ready',revision=revision+1 WHERE id=?1 AND revision=?2 AND state='pending'
            AND EXISTS(SELECT 1 FROM runs WHERE runs.id=node_runs.run_id AND runs.state='running')",params![id,revision])?)?;
        let run: String = tx.query_row("SELECT run_id FROM node_runs WHERE id=?1", [id], |r| {
            r.get(0)
        })?;
        event(&tx, &run, "node.ready", Some(id), None, json!({}))?;
        tx.commit()?;
        Ok(())
    }
    pub fn begin_attempt(&mut self, node: &str, revision: i64) -> Result<Attempt> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        changed(tx.execute("UPDATE node_runs SET state='running',revision=revision+1 WHERE id=?1 AND revision=?2 AND state='ready'
            AND EXISTS(SELECT 1 FROM runs WHERE runs.id=node_runs.run_id AND runs.state='running')",params![node,revision])?)?;
        let run: String =
            tx.query_row("SELECT run_id FROM node_runs WHERE id=?1", [node], |r| {
                r.get(0)
            })?;
        let number: i64 = tx.query_row(
            "SELECT COALESCE(MAX(attempt_number),0)+1 FROM attempts WHERE node_run_id=?1",
            [node],
            |r| r.get(0),
        )?;
        let id = uuid::Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO attempts(id,node_run_id,attempt_number,state) VALUES(?1,?2,?3,'starting')",
            params![id, node, number],
        )?;
        event(
            &tx,
            &run,
            "attempt.created",
            Some(node),
            Some(&id),
            json!({"number":number}),
        )?;
        tx.commit()?;
        self.attempt(&id)
    }
    pub fn mark_running(&mut self, id: &str, revision: i64) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        changed(tx.execute("UPDATE attempts SET state='running',revision=revision+1 WHERE id=?1 AND revision=?2 AND state='starting'",params![id,revision])?)?;
        let (node,run):(String,String)=tx.query_row("SELECT n.id,n.run_id FROM attempts a JOIN node_runs n ON n.id=a.node_run_id WHERE a.id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?)))?;
        event(&tx, &run, "node.started", Some(&node), Some(id), json!({}))?;
        tx.commit()?;
        Ok(())
    }
    pub fn finish_attempt(
        &mut self,
        id: &str,
        revision: i64,
        outcome: AttemptOutcome,
    ) -> Result<()> {
        let (state, output, error) = outcome_values(outcome);
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        changed(tx.execute("UPDATE attempts SET state=?3,revision=revision+1,output_json=?4,error_code=?5
            WHERE id=?1 AND revision=?2 AND (state='running' OR (state='starting' AND ?3!='succeeded'))",
            params![id,revision,state,output,error])?)?;
        let (node,run):(String,String)=tx.query_row("SELECT n.id,n.run_id FROM attempts a JOIN node_runs n ON n.id=a.node_run_id WHERE a.id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?)))?;
        changed(tx.execute(
            "UPDATE node_runs SET state=?2,revision=revision+1 WHERE id=?1 AND state='running'",
            params![node, state],
        )?)?;
        event(
            &tx,
            &run,
            &format!("node.{state}"),
            Some(&node),
            Some(id),
            json!({"error_code":error}),
        )?;
        tx.commit()?;
        Ok(())
    }
}

fn outcome_values(outcome: AttemptOutcome) -> (&'static str, Option<String>, Option<String>) {
    match outcome {
        AttemptOutcome::Succeeded(v) => ("succeeded", Some(v.to_string()), None),
        AttemptOutcome::Failed(code) => ("failed", None, Some(code)),
        AttemptOutcome::Interrupted => ("interrupted", None, Some("interrupted".into())),
        AttemptOutcome::Cancelled => ("cancelled", None, Some("cancelled".into())),
    }
}
