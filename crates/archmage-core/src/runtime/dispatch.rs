use super::{Runtime, scheduler};
use crate::{
    artifact::ArtifactStore,
    capability::CapabilityKind,
    context::{ContextRef, resolve_context},
    contract::{ContractContext, validate_output},
    error::{Error, Result},
    provider::codex::{self, CodexInvocation, Sandbox},
    store::{AttemptOutcome, Store},
    workspace::{GitRepository, inspect_repository, publish_changeset, snapshot},
};
use serde_json::json;
use std::{collections::HashMap, fs, path::Path, time::Duration};

impl Runtime {
    pub async fn run_to_terminal(&self, run_id: &str) -> Result<super::RunSnapshot> {
        let _owner =
            super::control::RunOwnerLock::acquire(&self.config.data_root.join("locks"), run_id)?;
        let mut store = self.store()?;
        let generation = store.claim_run_owner(run_id)?;
        drop(store);

        loop {
            let current = self.snapshot(run_id)?;
            match scheduler::terminal_decision(&current.nodes) {
                scheduler::TerminalDecision::Completed => {
                    let reviewer = self.store()?.node_result(run_id, "reviewer")?;
                    let outcome = reviewer.output["status"]
                        .as_str()
                        .ok_or_else(|| Error::Preflight("reviewer status is missing".into()))?;
                    self.store()?
                        .complete_run_with_outcome(run_id, current.revision, outcome)?;
                    return self.snapshot(run_id);
                }
                scheduler::TerminalDecision::Failed => {
                    self.store()?
                        .fail_run(run_id, current.revision, "node_failed")?;
                    return Err(Error::Preflight(format!(
                        "run {run_id} failed; inspect persisted events"
                    )));
                }
                scheduler::TerminalDecision::Cancelled => {
                    return Err(Error::Preflight(format!("run {run_id} was cancelled")));
                }
                scheduler::TerminalDecision::Interrupted => {
                    return Err(Error::Preflight(format!("run {run_id} was interrupted")));
                }
                scheduler::TerminalDecision::Continue => {}
            }

            let ready = scheduler::ready_keys(&current.nodes);
            let Some(key) = ready.first().copied() else {
                return Err(Error::Conflict);
            };
            let node = current
                .nodes
                .iter()
                .find(|node| node.key == key)
                .ok_or(Error::Conflict)?;
            self.store()?.ready_node(&node.id, node.revision)?;
            if let Err(error) = self.dispatch_node(run_id, key, generation).await {
                let failed = self.snapshot(run_id)?;
                if failed.state == "running"
                    && failed.nodes.iter().any(|node| node.state == "failed")
                {
                    let _ = self
                        .store()?
                        .fail_run(run_id, failed.revision, error_code(&error));
                }
                return Err(error);
            }
        }
    }

    async fn dispatch_node(&self, run_id: &str, node_key: &str, generation: i64) -> Result<()> {
        let kind = capability(node_key)?;
        let mut store = self.store()?;
        let node = store
            .nodes(run_id)?
            .into_iter()
            .find(|node| node.key == node_key)
            .ok_or(Error::Conflict)?;
        let attempt = store.begin_attempt(&node.id, node.revision)?;
        let result = self
            .execute_attempt(
                run_id,
                kind,
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
    async fn execute_attempt(
        &self,
        run_id: &str,
        kind: CapabilityKind,
        attempt_id: &str,
        attempt_revision: i64,
        generation: i64,
        store: &mut Store,
    ) -> Result<()> {
        let run = self.snapshot(run_id)?;
        let binding = run.agent_bindings.get(kind.key()).ok_or_else(|| {
            Error::Preflight(format!("missing frozen Agent binding for {}", kind.key()))
        })?;
        let tools = codex::inspect_at(binding.executable_path.as_deref()).await?;
        let workspace_record = store.workspace_for_run(run_id)?;
        let repository = inspect_repository(Path::new(&workspace_record.path))?;
        let artifacts = ArtifactStore::open(&self.config.data_root.join("artifacts"))?;
        let inputs = context_inputs(kind, &run, store, &artifacts, &repository)?;
        let context = resolve_context(kind, "v1", inputs, &artifacts, store)?;
        let context_id =
            store.persist_context(run_id, &node_for(store, run_id, kind)?.id, &context)?;
        store.bind_attempt_context(attempt_id, &context_id)?;

        let attempt_dir = self.config.data_root.join("attempts").join(attempt_id);
        private_dir(&attempt_dir)?;
        let schema_path = attempt_dir.join("schema.json");
        fs::write(&schema_path, kind.schema())?;
        let result_path = attempt_dir.join("result.json");
        let diagnostics_dir = attempt_dir.join("diagnostics");
        let sandbox = match kind {
            CapabilityKind::Designer | CapabilityKind::Reviewer => Sandbox::ReadOnly,
            CapabilityKind::Coder | CapabilityKind::Tester => Sandbox::WorkspaceWrite,
        };
        let mut invocation = CodexInvocation::new(
            repository.root.clone(),
            schema_path,
            result_path,
            diagnostics_dir,
            sandbox,
        );
        invocation.deadline = Duration::from_secs(self.config.node_timeout_seconds);
        invocation.max_line_bytes = self.config.max_line_bytes;
        invocation.max_result_bytes = self.config.max_result_bytes;
        invocation.max_stream_bytes = self.config.max_stream_bytes;
        invocation.model = binding.model.clone();

        let outcome =
            codex::invoke_with_identity(&tools, &invocation, &context.rendered, |identity| {
                store.attach_attempt_process(attempt_id, attempt_revision, generation, identity)
            })
            .await?;

        let bytes = serde_json::to_vec(&outcome.result)
            .map_err(|error| Error::Preflight(error.to_string()))?;
        let contract = contract_context(kind, run_id, store)?;
        let validated = validate_output(kind, &bytes, &contract)?;
        if kind == CapabilityKind::Coder && validated["status"] == "blocked" {
            return Err(Error::Preflight("coder reported blocked".into()));
        }
        let result_artifact = artifacts.publish(store, attempt_id, &bytes, "application/json")?;
        let changeset_artifact = if kind == CapabilityKind::Coder {
            Some(publish_changeset(&artifacts, store, attempt_id, &repository)?.manifest_artifact)
        } else {
            None
        };
        let current = store.attempt(attempt_id)?;
        store.finish_attempt_owned_with_result(
            attempt_id,
            current.revision,
            generation,
            validated,
            &outcome.thread_id,
            &result_artifact,
            changeset_artifact.as_deref(),
        )
    }
}

fn capability(key: &str) -> Result<CapabilityKind> {
    match key {
        "designer" => Ok(CapabilityKind::Designer),
        "coder" => Ok(CapabilityKind::Coder),
        "tester" => Ok(CapabilityKind::Tester),
        "reviewer" => Ok(CapabilityKind::Reviewer),
        _ => Err(Error::Preflight(format!("unknown capability node: {key}"))),
    }
}

fn node_for(store: &Store, run_id: &str, kind: CapabilityKind) -> Result<crate::store::Node> {
    let key = format!("{kind:?}").to_lowercase();
    store
        .nodes(run_id)?
        .into_iter()
        .find(|node| node.key == key)
        .ok_or(Error::Conflict)
}

fn context_inputs(
    kind: CapabilityKind,
    run: &super::RunSnapshot,
    store: &Store,
    artifacts: &ArtifactStore,
    repository: &GitRepository,
) -> Result<HashMap<String, ContextRef>> {
    let mut inputs = HashMap::from([(
        "goal".into(),
        ContextRef::WorkflowInput {
            value: json!(run.goal),
        },
    )]);
    match kind {
        CapabilityKind::Designer => {
            inputs.insert(
                "repo_view".into(),
                ContextRef::WorkspaceSnapshot {
                    value: serde_json::to_value(snapshot(repository)?)
                        .map_err(|error| Error::Preflight(error.to_string()))?,
                },
            );
        }
        CapabilityKind::Coder => {
            insert_result(&mut inputs, store, &run.id, "designer", "design")?;
        }
        CapabilityKind::Tester => {
            insert_result(&mut inputs, store, &run.id, "designer", "design")?;
            insert_result(&mut inputs, store, &run.id, "coder", "implementation")?;
            let coder = store.node_result(&run.id, "coder")?;
            inputs.insert(
                "changeset".into(),
                ContextRef::Artifact {
                    artifact_id: coder.changeset_artifact_id.ok_or_else(|| {
                        Error::Preflight("coder changeset artifact is missing".into())
                    })?,
                },
            );
            inputs.insert(
                "workspace_manifest".into(),
                ContextRef::WorkspaceSnapshot {
                    value: serde_json::to_value(snapshot(repository)?)
                        .map_err(|error| Error::Preflight(error.to_string()))?,
                },
            );
        }
        CapabilityKind::Reviewer => {
            insert_result(&mut inputs, store, &run.id, "designer", "design")?;
            insert_result(&mut inputs, store, &run.id, "coder", "implementation")?;
            let coder = store.node_result(&run.id, "coder")?;
            inputs.insert(
                "changeset".into(),
                ContextRef::Artifact {
                    artifact_id: coder.changeset_artifact_id.ok_or_else(|| {
                        Error::Preflight("coder changeset artifact is missing".into())
                    })?,
                },
            );
            insert_result(&mut inputs, store, &run.id, "tester", "test_result")?;
        }
    }
    let _ = artifacts;
    Ok(inputs)
}

fn insert_result(
    inputs: &mut HashMap<String, ContextRef>,
    store: &Store,
    run_id: &str,
    producer: &str,
    input_key: &str,
) -> Result<()> {
    let result = store.node_result(run_id, producer)?;
    inputs.insert(
        input_key.into(),
        ContextRef::CommittedNodeOutput {
            artifact_id: result.result_artifact_id,
        },
    );
    Ok(())
}

fn contract_context(kind: CapabilityKind, run_id: &str, store: &Store) -> Result<ContractContext> {
    if kind == CapabilityKind::Reviewer {
        let tester = store.node_result(run_id, "tester")?;
        Ok(ContractContext {
            tester_accepted: tester.output["status"] == "passed",
        })
    } else {
        Ok(ContractContext::default())
    }
}

fn private_dir(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    Ok(())
}

fn error_code(error: &Error) -> &'static str {
    match error {
        Error::Storage(_) => "storage_failed",
        Error::Conflict => "state_conflict",
        Error::Preflight(_) => "preflight_failed",
        Error::Spawn(_) => "spawn_failed",
        Error::TimedOut => "timed_out",
        Error::Limit(_) => "resource_limit",
        Error::Workspace(_) => "workspace_failed",
        Error::Io(_) => "io_failed",
        Error::NotImplemented(_) => "not_implemented",
    }
}
