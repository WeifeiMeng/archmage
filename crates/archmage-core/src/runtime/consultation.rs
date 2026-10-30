use super::{
    Runtime,
    dispatch::{dsh_sidecar_config, provider_execution},
};
use crate::{
    capability::{CapabilityConfiguration, CapabilityKind, GUIDANCE_NOTE},
    contract::{CONSULTATION_SCHEMA, validate_consultation},
    error::{Error, Result},
    provider::{
        codex::Sandbox,
        executor::{self, ProviderLifecycle, ProviderSession},
    },
};
use serde_json::{Value, json};
use std::fs;
use tokio::task::JoinSet;

fn kind(key: &str) -> Result<CapabilityKind> {
    match key {
        "designer" => Ok(CapabilityKind::Designer),
        "coder" => Ok(CapabilityKind::Coder),
        "tester" => Ok(CapabilityKind::Tester),
        "reviewer" => Ok(CapabilityKind::Reviewer),
        _ => Err(Error::Preflight(format!(
            "unknown consultation capability: {key}"
        ))),
    }
}

pub fn render_consultation_prompt(
    target: &str,
    source: &str,
    source_attempt_id: &str,
    goal: &str,
    source_output: &Value,
    target_configuration: Option<&CapabilityConfiguration>,
) -> Result<Vec<u8>> {
    let stage = match kind(source)? {
        CapabilityKind::Designer => {
            "This is a design-stage deliverable. Judge whether the design is feasible, clear, and testable. Implementation, ChangeSet, test execution, and final review do not exist yet; their absence is NEVER a reason to reject this design."
        }
        CapabilityKind::Coder => {
            "This is an implementation-stage deliverable. Judge the implementation against the goal and design. A Tester main result does not exist yet; its absence is NEVER a reason to reject this implementation."
        }
        CapabilityKind::Tester => {
            "This is a testing-stage deliverable. Judge whether the test evidence is adequate for the implemented change. Final Reviewer main output does not exist yet; its absence is NEVER a reason to reject this test result."
        }
        CapabilityKind::Reviewer => "Judge only the current Reviewer deliverable.",
    };
    let persona = kind(target)?.prompt();
    serde_json::to_vec(&json!({
        "mode":"read_only_consultation",
        "persona_for_perspective_only":persona,
        "extra_instructions":target_configuration.map(|item| item.extra_instructions.as_str()).unwrap_or(""),
        "skills":target_configuration.map(|item| item.skills.as_slice()).unwrap_or(&[]),
        "tools":target_configuration.map(|item| item.tools.as_slice()).unwrap_or(&[]),
        "guidance_note":GUIDANCE_NOTE,
        "task":"This is a stage-specific consultation, not your main capability task. Evaluate ONLY the current upstream deliverable. Do not require artifacts or completed tasks from later stages. Approve when the upstream deliverable is sufficient for its own stage, even though the entire Run is not finished. Do not perform your main task or modify files. Return only schema-valid JSON. If rejecting, identify a defect that the upstream capability can fix NOW in its own deliverable and provide specific actionable suggestions.",
        "stage_rule":stage,
        "target_capability":target,
        "source_capability":source,
        "source_attempt_id":source_attempt_id,
        "goal":goal,
        "source_output":source_output,
    })).map_err(|error| Error::Preflight(error.to_string()))
}

impl Runtime {
    pub(crate) async fn consult_missing(
        &self,
        run_id: &str,
        source: &str,
        source_attempt_id: &str,
        targets: &[String],
        generation: i64,
    ) -> Result<()> {
        let mut jobs = JoinSet::new();
        for target in targets {
            let runtime = self.clone();
            let run_id = run_id.to_owned();
            let source = source.to_owned();
            let source_attempt_id = source_attempt_id.to_owned();
            let target = target.clone();
            jobs.spawn(async move {
                runtime
                    .consult_edge(&run_id, &source, &source_attempt_id, &target, generation)
                    .await
            });
        }
        let mut first_error = None;
        while let Some(result) = jobs.join_next().await {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(Error::Preflight(format!(
                            "consultation task failed: {error}"
                        )));
                    }
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    async fn consult_edge(
        &self,
        run_id: &str,
        source: &str,
        source_attempt_id: &str,
        target: &str,
        generation: i64,
    ) -> Result<()> {
        let run = self.snapshot(run_id)?;
        let producer = run
            .nodes
            .iter()
            .find(|node| node.key == source)
            .ok_or(Error::Conflict)?;
        if producer.state != "succeeded"
            || producer
                .result
                .as_ref()
                .map(|item| item.attempt_id.as_str())
                != Some(source_attempt_id)
        {
            return Err(Error::Conflict);
        }
        let binding = run
            .agent_bindings
            .get(target)
            .ok_or_else(|| Error::Preflight(format!("missing frozen Agent for {target}")))?;
        let prompt = render_consultation_prompt(
            target,
            source,
            source_attempt_id,
            &run.goal,
            &producer.result.as_ref().ok_or(Error::Conflict)?.output,
            run.capability_configurations.get(target),
        )?;
        let workspace = run
            .workspace
            .ok_or_else(|| Error::Preflight("run workspace is not ready".into()))?;
        let attempt_dir = self
            .config
            .data_root
            .join("consultations")
            .join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&attempt_dir)?;
        let schema_path = attempt_dir.join("schema.json");
        fs::write(&schema_path, CONSULTATION_SCHEMA)?;
        let process_id = uuid::Uuid::new_v4().to_string();
        let registration = self.clone();
        let registered_id = process_id.clone();
        let registered_run = run_id.to_owned();
        let registered_attempt = source_attempt_id.to_owned();
        let registered_target = target.to_owned();
        let result = if binding.provider_kind == crate::agent::ProviderKind::DeepseekHarness {
            let schema: serde_json::Value = serde_json::from_str(CONSULTATION_SCHEMA)
                .map_err(|error| Error::Preflight(error.to_string()))?;
            let sidecar = dsh_sidecar_config(
                binding,
                workspace,
                attempt_dir.join("diagnostics"),
                crate::provider::deepseek_harness::PermissionMode::ReadOnly,
                attempt_dir.join("sessions"),
                &self.config,
            )?;
            self.provider_pool
                .invoke_temporary(
                    sidecar,
                    source_attempt_id,
                    &prompt,
                    &schema,
                    move |identity| {
                        registration.store()?.register_consultation_process(
                            &registered_id,
                            &registered_run,
                            &registered_attempt,
                            &registered_target,
                            generation,
                            identity,
                        )
                    },
                )
                .await
        } else {
            let invocation = provider_execution(
                binding,
                workspace,
                schema_path,
                attempt_dir.join("result.json"),
                attempt_dir.join("diagnostics"),
                Sandbox::ReadOnly,
                ProviderSession::None,
                &self.config,
            );
            executor::invoke(&invocation, &prompt, move |event| match event {
                ProviderLifecycle::Spawned(identity) => {
                    registration.store()?.register_consultation_process(
                        &registered_id,
                        &registered_run,
                        &registered_attempt,
                        &registered_target,
                        generation,
                        &identity,
                    )
                }
                ProviderLifecycle::ConversationPersisted { .. } => Ok(()),
            })
            .await
        };
        self.store()?.finish_consultation_process(&process_id)?;
        let outcome = result?;
        let raw = serde_json::to_vec(&outcome.result)
            .map_err(|error| Error::Preflight(error.to_string()))?;
        let decision = validate_consultation(&raw)?;
        let mut store = self.store()?;
        let round = store.attempt(source_attempt_id)?.number - 1;
        store.record_consultation(
            run_id,
            source,
            target,
            source_attempt_id,
            round,
            generation,
            &decision.decision,
            &decision.summary,
            &decision.suggestions,
            &outcome.conversation_id,
            &outcome.result,
        )?;
        Ok(())
    }
}
