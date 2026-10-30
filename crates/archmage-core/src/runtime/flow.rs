use super::{
    Runtime,
    dispatch::{
        capability, dsh_sidecar_config, error_code, permission_mode, private_dir,
        provider_execution,
    },
};
use crate::{
    artifact::ArtifactStore,
    capability::CapabilityKind,
    context::{ContextRef, resolve_context_for_flow},
    contract::{ContractContext, flow_output_schema, validate_flow_output},
    error::{Error, Result},
    provider::{
        codex::Sandbox,
        executor::{self, ProviderLifecycle},
        pool::SidecarKey,
    },
    store::{AttemptOutcome, FlowDelivery, Store},
    workspace::{inspect_repository, publish_changeset, snapshot},
};
use serde_json::json;
use std::{collections::HashMap, fs, path::Path};

impl Runtime {
    pub(super) async fn run_flow_to_terminal(
        &self,
        run_id: &str,
        generation: i64,
    ) -> Result<super::RunSnapshot> {
        let initial = self.snapshot(run_id)?;
        let graph = &initial.team.as_ref().ok_or(Error::Conflict)?.graph;
        let entry = graph.entry.as_deref().ok_or(Error::Conflict)?;
        self.store()?
            .enqueue_flow_entry(run_id, generation, entry)?;

        loop {
            let run = self.snapshot(run_id)?;
            if run.state == "completed" {
                return Ok(run);
            }
            if run.state != "running" {
                return Err(Error::Preflight(format!(
                    "flow run {run_id} is {}",
                    run.state
                )));
            }
            if step_limit_reached(&run.flow_deliveries, run.flow_step_limit) {
                self.store()?
                    .fail_flow_run(run_id, generation, "flow_step_limit_reached")?;
                return Err(Error::Limit("flow_step_limit_reached".into()));
            }
            let delivery = self.store()?.claim_flow_delivery(run_id, generation)?;
            let Some(delivery) = delivery else {
                let code = if run
                    .flow_deliveries
                    .iter()
                    .any(|item| item.state == "running")
                {
                    "flow_delivery_stuck"
                } else {
                    "flow_exhausted_without_terminal"
                };
                self.store()?.fail_flow_run(run_id, generation, code)?;
                return Err(Error::Preflight(code.into()));
            };
            if let Err(error) = self
                .dispatch_flow_delivery(&run, &delivery, generation)
                .await
            {
                if self.snapshot(run_id)?.state == "running" {
                    let _ = self
                        .store()?
                        .fail_flow_run(run_id, generation, error_code(&error));
                }
                return Err(error);
            }
        }
    }

    async fn dispatch_flow_delivery(
        &self,
        run: &super::RunSnapshot,
        delivery: &FlowDelivery,
        generation: i64,
    ) -> Result<()> {
        let mut store = self.store()?;
        store.prepare_flow_node(&run.id, &delivery.target, generation)?;
        let node = store
            .nodes(&run.id)?
            .into_iter()
            .find(|node| node.key == delivery.target)
            .ok_or(Error::Conflict)?;
        store.ready_node(&node.id, node.revision)?;
        let attempt = store.begin_attempt(&node.id, node.revision + 1)?;
        store.bind_flow_attempt(&delivery.id, generation, &attempt.id)?;
        let result = self
            .execute_flow_attempt(
                run,
                delivery,
                &node.id,
                &attempt.id,
                attempt.revision,
                generation,
                &mut store,
            )
            .await;
        if let Err(error) = &result
            && let Ok(active) = store.attempt(&attempt.id)
        {
            let outcome = AttemptOutcome::Failed(error_code(error).into());
            if active.state == "running" {
                let _ =
                    store.finish_attempt_owned(&attempt.id, active.revision, generation, outcome);
            } else if active.state == "starting" {
                let _ = store.finish_attempt(&attempt.id, active.revision, outcome);
            }
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_flow_attempt(
        &self,
        run: &super::RunSnapshot,
        delivery: &FlowDelivery,
        node_id: &str,
        attempt_id: &str,
        attempt_revision: i64,
        generation: i64,
        store: &mut Store,
    ) -> Result<()> {
        let kind = capability(&delivery.target)?;
        let graph = &run.team.as_ref().ok_or(Error::Conflict)?.graph;
        let binding = run.agent_bindings.get(kind.key()).ok_or(Error::Conflict)?;
        let workspace = store.workspace_for_run(&run.id)?;
        let repository = inspect_repository(Path::new(&workspace.path))?;
        let artifacts = ArtifactStore::open(&self.config.data_root.join("artifacts"))?;
        let repo_view = serde_json::to_value(snapshot(&repository)?)
            .map_err(|error| Error::Preflight(error.to_string()))?;
        let mut inputs = HashMap::from([
            (
                "goal".into(),
                ContextRef::WorkflowInput {
                    value: json!(run.goal),
                },
            ),
            (
                "repo_view".into(),
                ContextRef::WorkspaceSnapshot { value: repo_view },
            ),
        ]);
        if let Some(artifact_id) = &delivery.payload_artifact_id {
            inputs.insert(
                "incoming".into(),
                ContextRef::CommittedNodeOutput {
                    artifact_id: artifact_id.clone(),
                },
            );
        }
        if let Some(changeset_id) = store.flow_source_changeset(&delivery.id)? {
            inputs.insert(
                "changeset".into(),
                ContextRef::Artifact {
                    artifact_id: changeset_id,
                },
            );
        }
        let previous = store.optional_node_result(&run.id, kind.key())?;
        if let Some(previous) = &previous {
            inputs.insert(
                "previous_result".into(),
                ContextRef::CommittedNodeOutput {
                    artifact_id: previous.result_artifact_id.clone(),
                },
            );
        }
        let configuration = run.capability_configurations.get(kind.key());
        let neighbors = graph
            .edges
            .iter()
            .filter(|edge| edge.from == kind.key())
            .map(|edge| edge.to.as_str())
            .collect::<Vec<_>>();
        let role = format!(
            "{}\nThis is a message-flow Team. Incoming delivery sequence: {}. Outgoing recipients: {}. Terminal capability: {}. Return the role deliverable and routing envelope together. Choose continue with at least one outgoing recipient, or complete only if you are the terminal capability. Do not invent handoffs or tests that are not in the supplied context.",
            configuration
                .map(|item| item.extra_instructions.as_str())
                .unwrap_or(""),
            delivery.sequence,
            neighbors.join(", "),
            graph.terminal.as_deref().unwrap_or("")
        );
        let context = resolve_context_for_flow(
            kind,
            configuration
                .map(|item| item.prompt_version.as_str())
                .unwrap_or(crate::capability::BUILTIN_PROMPT_VERSION),
            &role,
            configuration
                .map(|item| item.skills.as_slice())
                .unwrap_or(&[]),
            configuration
                .map(|item| item.tools.as_slice())
                .unwrap_or(&[]),
            previous.as_ref().map(|result| &result.output),
            inputs,
            &artifacts,
            store,
        )?;
        let context_id = store.persist_context(&run.id, node_id, &context)?;
        store.bind_attempt_context(attempt_id, &context_id)?;

        let attempt_dir = self.config.data_root.join("attempts").join(attempt_id);
        private_dir(&attempt_dir)?;
        let schema_path = attempt_dir.join("schema.json");
        fs::write(&schema_path, flow_output_schema(kind)?)?;
        let sandbox = match kind {
            CapabilityKind::Designer | CapabilityKind::Reviewer => Sandbox::ReadOnly,
            CapabilityKind::Coder | CapabilityKind::Tester => Sandbox::WorkspaceWrite,
        };
        let diagnostics_dir = attempt_dir.join("diagnostics");
        let outcome = if binding.provider_kind == crate::agent::ProviderKind::DeepseekHarness {
            let schema: serde_json::Value = serde_json::from_str(&flow_output_schema(kind)?)
                .map_err(|error| Error::Preflight(error.to_string()))?;
            let sidecar = dsh_sidecar_config(
                binding,
                repository.root.clone(),
                diagnostics_dir,
                permission_mode(sandbox),
                self.config
                    .data_root
                    .join("provider-sessions")
                    .join(&run.id)
                    .join(kind.key()),
                &self.config,
            )?;
            self.provider_pool
                .invoke_main(
                    SidecarKey::new(&run.id, kind.key()),
                    sidecar,
                    attempt_id,
                    &context.rendered,
                    &schema,
                    |identity| {
                        store.attach_attempt_process(
                            attempt_id,
                            attempt_revision,
                            generation,
                            identity,
                        )
                    },
                )
                .await?
        } else {
            let invocation = provider_execution(
                binding,
                repository.root.clone(),
                schema_path,
                attempt_dir.join("result.json"),
                diagnostics_dir,
                sandbox,
                &self.config,
            );
            executor::invoke(&invocation, &context.rendered, |event| match event {
                ProviderLifecycle::Spawned(identity) => store.attach_attempt_process(
                    attempt_id,
                    attempt_revision,
                    generation,
                    &identity,
                ),
            })
            .await?
        };
        let bytes = serde_json::to_vec(&outcome.result)
            .map_err(|error| Error::Preflight(error.to_string()))?;
        let contract = flow_contract_context(kind, delivery, store)?;
        let (deliverable, routing) = validate_flow_output(kind, &bytes, &contract, graph)?;
        let result_artifact = artifacts.publish(store, attempt_id, &bytes, "application/json")?;
        let changeset = if kind == CapabilityKind::Coder {
            Some(publish_changeset(&artifacts, store, attempt_id, &repository)?.manifest_artifact)
        } else {
            None
        };
        store.finish_flow_delivery(
            &delivery.id,
            generation,
            attempt_id,
            &result_artifact,
            changeset.as_deref(),
            &deliverable,
            &outcome.conversation_id,
            &routing.next,
            routing.decision == "complete",
        )
    }
}

fn step_limit_reached(deliveries: &[FlowDelivery], limit: usize) -> bool {
    deliveries
        .iter()
        .filter(|item| item.state == "completed")
        .count()
        >= limit
}

fn flow_contract_context(
    kind: CapabilityKind,
    delivery: &FlowDelivery,
    store: &Store,
) -> Result<ContractContext> {
    if kind != CapabilityKind::Reviewer {
        return Ok(ContractContext::default());
    }
    let tester_accepted = if let Some(source) = &delivery.source_attempt_id {
        let attempt = store.attempt(source)?;
        attempt
            .output
            .as_ref()
            .is_none_or(|value| value["status"] != "failed" && value["status"] != "blocked")
    } else {
        true
    };
    Ok(ContractContext { tester_accepted })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_delivery_is_stopped_after_twenty_four_completed_steps() {
        let mut deliveries = (0..25)
            .map(|sequence| FlowDelivery {
                id: sequence.to_string(),
                run_id: "run".into(),
                sequence,
                source_attempt_id: None,
                target: "coder".into(),
                payload_artifact_id: None,
                state: "completed".into(),
                attempt_id: None,
                routing: None,
                output: None,
            })
            .collect::<Vec<_>>();
        deliveries[24].state = "queued".into();
        assert!(step_limit_reached(&deliveries, 24));
        deliveries[23].state = "queued".into();
        assert!(!step_limit_reached(&deliveries, 24));
    }
}
