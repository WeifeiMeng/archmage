use super::{FrozenAgentBinding, Runtime, scheduler};
use crate::{
    agent::ProviderKind,
    artifact::ArtifactStore,
    capability::CapabilityKind,
    context::{ContextRef, resolve_context_for_team, resolve_context_with_revision},
    contract::{ContractContext, validate_output},
    error::{Error, Result},
    provider::{
        codex::Sandbox,
        deepseek_harness::{PermissionMode, SidecarConfig},
        executor::{self, ProviderExecution, ProviderLifecycle},
        pool::{ProviderSessionConfig, SidecarKey},
        qoder::QoderSessionConfig,
    },
    store::{AttemptOutcome, Store},
    team::TeamMode,
    workspace::{GitRepository, inspect_repository, publish_changeset, snapshot},
};
use serde_json::json;
use std::{collections::HashMap, fs, path::Path, time::Duration};

impl Runtime {
    pub async fn run_to_terminal(&self, run_id: &str) -> Result<super::RunSnapshot> {
        let result = self.run_to_terminal_inner(run_id).await;
        let cleanup = self.provider_pool.close_run(run_id).await;
        match (result, cleanup) {
            (Ok(snapshot), Ok(())) => Ok(snapshot),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), _) => Err(error),
        }
    }

    async fn run_to_terminal_inner(&self, run_id: &str) -> Result<super::RunSnapshot> {
        let _owner =
            super::control::RunOwnerLock::acquire(&self.config.data_root.join("locks"), run_id)?;
        let mut store = self.store()?;
        let generation = store.claim_run_owner(run_id)?;
        drop(store);

        if self
            .snapshot(run_id)?
            .team
            .as_ref()
            .is_some_and(|team| team.graph.mode == TeamMode::Flow)
        {
            return self.run_flow_to_terminal(run_id, generation).await;
        }

        loop {
            let current = self.snapshot(run_id)?;
            let terminal = scheduler::terminal_decision(&current.nodes);
            if matches!(
                terminal,
                scheduler::TerminalDecision::Continue | scheduler::TerminalDecision::Completed
            ) && current.state == "running"
                && current.team.is_some()
                && self.process_review_gates(&current, generation).await?
            {
                continue;
            }
            match terminal {
                scheduler::TerminalDecision::Completed => {
                    let has_reviewer = current.nodes.iter().any(|node| node.key == "reviewer");
                    let reviewer = has_reviewer
                        .then(|| self.store()?.node_result(run_id, "reviewer"))
                        .transpose()?;
                    let outcome = reviewer
                        .as_ref()
                        .map(|item| {
                            item.output["status"].as_str().ok_or_else(|| {
                                Error::Preflight("reviewer status is missing".into())
                            })
                        })
                        .transpose()?
                        .unwrap_or("approved");
                    if current.team.is_some() && outcome == "changes_requested" {
                        let reviewer = reviewer.as_ref().ok_or(Error::Conflict)?;
                        let feedback = reviewer_feedback(&reviewer.output);
                        let mut store = self.store()?;
                        let target = current
                            .nodes
                            .iter()
                            .filter(|node| node.key != "reviewer")
                            .find(|node| node.key == "coder")
                            .or_else(|| {
                                current
                                    .nodes
                                    .iter()
                                    .rev()
                                    .find(|node| node.key != "reviewer")
                            })
                            .ok_or(Error::Conflict)?;
                        let target_result = store.node_result(run_id, &target.key)?;
                        if store.attempt(&target_result.attempt_id)?.number <= 3 {
                            store.reopen_after_reviewer(
                                run_id,
                                generation,
                                &reviewer.attempt_id,
                                &feedback,
                            )?;
                            continue;
                        }
                        let node = current
                            .nodes
                            .iter()
                            .find(|node| node.key == "reviewer")
                            .ok_or(Error::Conflict)?;
                        store.fail_review_limit(&node.id, generation)?;
                        let failed = self.snapshot(run_id)?;
                        store.fail_run(run_id, failed.revision, "review_round_limit")?;
                        return Err(Error::Preflight("review round limit reached".into()));
                    }
                    if !has_reviewer
                        && let Some(last) = current.nodes.last()
                        && last
                            .result
                            .as_ref()
                            .and_then(|result| result.output["status"].as_str())
                            .is_some_and(|status| status == "failed" || status == "blocked")
                    {
                        self.store()?.fail_run(
                            run_id,
                            current.revision,
                            "last_capability_failed",
                        )?;
                        return Err(Error::Preflight("last capability reported failure".into()));
                    }
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

            let ready = if let Some(team) = &current.team {
                scheduler::ready_keys_for_team(&current.nodes, &team.graph)
            } else {
                scheduler::ready_keys(&current.nodes)
            };
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

    async fn process_review_gates(
        &self,
        run: &super::RunSnapshot,
        generation: i64,
    ) -> Result<bool> {
        let Some(team) = &run.team else {
            return Ok(false);
        };
        for node in &run.nodes {
            if node.state != "succeeded" {
                continue;
            }
            let outgoing = team
                .graph
                .edges
                .iter()
                .filter(|edge| edge.from == node.key)
                .map(|edge| edge.to.as_str())
                .collect::<Vec<_>>();
            if outgoing.is_empty() {
                continue;
            }
            let result = node.result.as_ref().ok_or(Error::Conflict)?;
            let consultations = self
                .store()?
                .consultations_for_attempt(&result.attempt_id)?;
            let decisions = consultations
                .iter()
                .map(|item| {
                    (
                        item.target_capability.as_str(),
                        item.decision.as_str(),
                        item.suggestions.clone(),
                    )
                })
                .collect::<Vec<_>>();
            match scheduler::review_gate(&outgoing, &decisions) {
                scheduler::ReviewGate::Awaiting(targets) => {
                    if let Err(error) = self
                        .consult_missing(
                            &run.id,
                            &node.key,
                            &result.attempt_id,
                            &targets,
                            generation,
                        )
                        .await
                    {
                        let mut store = self.store()?;
                        if self.snapshot(&run.id)?.state == "running" {
                            store.fail_consultation_node(
                                &node.id,
                                generation,
                                error_code(&error),
                            )?;
                            let failed = self.snapshot(&run.id)?;
                            store.fail_run(&run.id, failed.revision, "consultation_failed")?;
                        }
                        return Err(error);
                    }
                    return Ok(true);
                }
                scheduler::ReviewGate::ChangesRequested(suggestions) => {
                    let mut store = self.store()?;
                    if store.attempt(&result.attempt_id)?.number <= 3 {
                        store.reopen_node_for_feedback(&node.id, generation, &suggestions)?;
                    } else {
                        store.fail_review_limit(&node.id, generation)?;
                        let failed = self.snapshot(&run.id)?;
                        store.fail_run(&run.id, failed.revision, "review_round_limit")?;
                        return Err(Error::Preflight("review round limit reached".into()));
                    }
                    return Ok(true);
                }
                scheduler::ReviewGate::Approved => {}
            }
        }
        Ok(false)
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
        let workspace_record = store.workspace_for_run(run_id)?;
        let repository = inspect_repository(Path::new(&workspace_record.path))?;
        let artifacts = ArtifactStore::open(&self.config.data_root.join("artifacts"))?;
        let inputs = context_inputs(kind, &run, store, &artifacts, &repository)?;
        let configuration = run.capability_configurations.get(kind.key());
        let extra_instructions = configuration
            .map(|item| item.extra_instructions.as_str())
            .unwrap_or("");
        let prompt_version = configuration
            .map(|item| item.prompt_version.as_str())
            .unwrap_or(crate::capability::BUILTIN_PROMPT_VERSION);
        let node_id = node_for(store, run_id, kind)?.id;
        let feedback = store.feedback_for_node(&node_id)?;
        let previous_output = store.previous_output_for_revision(&node_id)?;
        let resolve = if run.team.is_some() {
            resolve_context_for_team
        } else {
            resolve_context_with_revision
        };
        let role_instructions = team_role_instructions(kind, &run, extra_instructions);
        let context = resolve(
            kind,
            prompt_version,
            &role_instructions,
            configuration
                .map(|item| item.skills.as_slice())
                .unwrap_or(&[]),
            configuration
                .map(|item| item.tools.as_slice())
                .unwrap_or(&[]),
            &feedback,
            previous_output.as_ref(),
            inputs,
            &artifacts,
            store,
        )?;
        let context_id = store.persist_context(run_id, &node_id, &context)?;
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
        let outcome = if matches!(
            binding.provider_kind,
            ProviderKind::DeepseekHarness | ProviderKind::Qoder
        ) {
            let schema: serde_json::Value = serde_json::from_str(kind.schema())
                .map_err(|error| Error::Preflight(error.to_string()))?;
            let session = persistent_provider_config(
                binding,
                repository.root.clone(),
                diagnostics_dir,
                sandbox,
                self.config
                    .data_root
                    .join("provider-sessions")
                    .join(run_id)
                    .join(kind.key()),
                &self.config,
            )?;
            self.provider_pool
                .invoke_main(
                    SidecarKey::new(run_id, kind.key()),
                    session,
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
                result_path,
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
            &outcome.conversation_id,
            &result_artifact,
            changeset_artifact.as_deref(),
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn provider_execution(
    binding: &FrozenAgentBinding,
    repository: std::path::PathBuf,
    schema_path: std::path::PathBuf,
    result_path: std::path::PathBuf,
    diagnostics_dir: std::path::PathBuf,
    sandbox: Sandbox,
    config: &crate::config::Config,
) -> ProviderExecution {
    ProviderExecution {
        provider_kind: binding.provider_kind,
        executable_path: binding.executable_path.clone(),
        model: binding.model.clone(),
        repository,
        schema_path,
        result_path,
        diagnostics_dir,
        sandbox,
        deadline: Duration::from_secs(config.node_timeout_seconds),
        max_line_bytes: config.max_line_bytes,
        max_stream_bytes: config.max_stream_bytes,
        max_result_bytes: config.max_result_bytes,
    }
}

pub(super) fn dsh_sidecar_config(
    binding: &FrozenAgentBinding,
    repository: std::path::PathBuf,
    diagnostics_dir: std::path::PathBuf,
    permission_mode: PermissionMode,
    session_root: std::path::PathBuf,
    config: &crate::config::Config,
) -> Result<SidecarConfig> {
    Ok(SidecarConfig {
        python_binary: binding
            .executable_path
            .clone()
            .ok_or_else(|| Error::Preflight("DeepSeek Harness Python path is missing".into()))?,
        data_root: config.data_root.clone(),
        repository,
        provider_route: binding
            .provider_route
            .clone()
            .ok_or_else(|| Error::Preflight("DeepSeek Harness provider route is missing".into()))?,
        model: binding
            .model
            .clone()
            .ok_or_else(|| Error::Preflight("DeepSeek Harness model is missing".into()))?,
        session_id: String::new(),
        permission_mode,
        session_root,
        diagnostics_dir,
        deadline: Duration::from_secs(config.node_timeout_seconds),
        max_line_bytes: config.max_line_bytes,
        max_stream_bytes: config.max_stream_bytes,
        max_result_bytes: config.max_result_bytes,
        environment: std::collections::BTreeMap::new(),
    })
}

pub(super) fn qoder_session_config(
    binding: &FrozenAgentBinding,
    repository: std::path::PathBuf,
    diagnostics_dir: std::path::PathBuf,
    sandbox: Sandbox,
    config: &crate::config::Config,
) -> Result<QoderSessionConfig> {
    Ok(QoderSessionConfig {
        binary: binding
            .executable_path
            .clone()
            .ok_or_else(|| Error::Preflight("Qoder executable path is missing".into()))?,
        repository,
        model: binding.model.clone(),
        agent_name: binding.agent_name.clone(),
        provider_config: binding.provider_config.clone(),
        access: match sandbox {
            Sandbox::ReadOnly => crate::provider::acp::AcpAccess::ReadOnly,
            Sandbox::WorkspaceWrite => crate::provider::acp::AcpAccess::WorkspaceWrite,
        },
        diagnostics_dir,
        deadline: Duration::from_secs(config.node_timeout_seconds),
        max_line_bytes: config.max_line_bytes,
        max_stream_bytes: config.max_stream_bytes,
        max_result_bytes: config.max_result_bytes,
        environment: std::collections::BTreeMap::new(),
    })
}

pub(super) fn persistent_provider_config(
    binding: &FrozenAgentBinding,
    repository: std::path::PathBuf,
    diagnostics_dir: std::path::PathBuf,
    sandbox: Sandbox,
    session_root: std::path::PathBuf,
    config: &crate::config::Config,
) -> Result<ProviderSessionConfig> {
    match binding.provider_kind {
        ProviderKind::DeepseekHarness => Ok(dsh_sidecar_config(
            binding,
            repository,
            diagnostics_dir,
            permission_mode(sandbox),
            session_root,
            config,
        )?
        .into()),
        ProviderKind::Qoder => {
            Ok(qoder_session_config(binding, repository, diagnostics_dir, sandbox, config)?.into())
        }
        kind => Err(Error::Preflight(format!(
            "{kind} does not use a persistent provider session"
        ))),
    }
}

pub(super) fn permission_mode(sandbox: Sandbox) -> PermissionMode {
    match sandbox {
        Sandbox::ReadOnly => PermissionMode::ReadOnly,
        Sandbox::WorkspaceWrite => PermissionMode::WorkspaceWrite,
    }
}

pub(super) fn capability(key: &str) -> Result<CapabilityKind> {
    match key {
        "designer" => Ok(CapabilityKind::Designer),
        "coder" => Ok(CapabilityKind::Coder),
        "tester" => Ok(CapabilityKind::Tester),
        "reviewer" => Ok(CapabilityKind::Reviewer),
        _ => Err(Error::Preflight(format!("unknown capability node: {key}"))),
    }
}

fn reviewer_feedback(output: &serde_json::Value) -> Vec<String> {
    let issues = output["issues"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|issue| issue["message"].as_str())
        .filter(|message| !message.trim().is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if issues.is_empty() {
        vec![
            output["summary_markdown"]
                .as_str()
                .filter(|item| !item.trim().is_empty())
                .unwrap_or("Reviewer requested changes; revise the implementation and tests.")
                .to_owned(),
        ]
    } else {
        issues
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
    let completed = |key: &str| {
        run.nodes
            .iter()
            .any(|node| node.key == key && node.state == "succeeded")
    };
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
            if completed("designer") {
                insert_result(&mut inputs, store, &run.id, "designer", "design")?;
            } else {
                insert_workspace(&mut inputs, "repo_view", repository)?;
            }
        }
        CapabilityKind::Tester => {
            if completed("designer") {
                insert_result(&mut inputs, store, &run.id, "designer", "design")?;
            }
            if completed("coder") {
                insert_coder_result(&mut inputs, store, &run.id)?;
            } else {
                insert_workspace(&mut inputs, "repo_view", repository)?;
            }
            insert_workspace(&mut inputs, "workspace_manifest", repository)?;
        }
        CapabilityKind::Reviewer => {
            if completed("designer") {
                insert_result(&mut inputs, store, &run.id, "designer", "design")?;
            }
            if completed("coder") {
                insert_coder_result(&mut inputs, store, &run.id)?;
            }
            if completed("tester") {
                insert_result(&mut inputs, store, &run.id, "tester", "test_result")?;
            }
        }
    }
    let _ = artifacts;
    Ok(inputs)
}

fn insert_workspace(
    inputs: &mut HashMap<String, ContextRef>,
    key: &str,
    repository: &GitRepository,
) -> Result<()> {
    inputs.insert(
        key.into(),
        ContextRef::WorkspaceSnapshot {
            value: serde_json::to_value(snapshot(repository)?)
                .map_err(|error| Error::Preflight(error.to_string()))?,
        },
    );
    Ok(())
}

fn insert_coder_result(
    inputs: &mut HashMap<String, ContextRef>,
    store: &Store,
    run_id: &str,
) -> Result<()> {
    insert_result(inputs, store, run_id, "coder", "implementation")?;
    let coder = store.node_result(run_id, "coder")?;
    inputs.insert(
        "changeset".into(),
        ContextRef::Artifact {
            artifact_id: coder
                .changeset_artifact_id
                .ok_or_else(|| Error::Preflight("coder changeset artifact is missing".into()))?,
        },
    );
    Ok(())
}

fn team_role_instructions(
    kind: CapabilityKind,
    run: &super::RunSnapshot,
    configured: &str,
) -> String {
    if run.team.is_none() {
        return configured.to_owned();
    }
    let note = match kind {
        CapabilityKind::Designer => "Create a design from the goal and repository view.",
        CapabilityKind::Coder => {
            "Implement the goal directly when no design binding is supplied. Do not invent a Designer handoff."
        }
        CapabilityKind::Tester => {
            "Choose and perform appropriate checks from the available implementation or repository evidence. Do not invent a Coder handoff."
        }
        CapabilityKind::Reviewer => {
            "Review only the supplied evidence. If no test_result is supplied, tests have not been run by a Tester; never claim they passed. You may approve based on available evidence if justified."
        }
    };
    format!("{configured}\n{note}")
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
        let has_tester = store.nodes(run_id)?.iter().any(|node| node.key == "tester");
        let accepted = if has_tester {
            store.node_result(run_id, "tester")?.output["status"] == "passed"
        } else {
            true
        };
        Ok(ContractContext {
            tester_accepted: accepted,
        })
    } else {
        Ok(ContractContext::default())
    }
}

pub(super) fn private_dir(path: &Path) -> Result<()> {
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

pub(super) fn error_code(error: &Error) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviewer_without_tester_can_approve_available_evidence() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = Store::open(&directory.path().join("state.db")).unwrap();
        let run = store.create_run("goal", &["coder", "reviewer"]).unwrap();
        assert!(
            contract_context(CapabilityKind::Reviewer, &run, &store)
                .unwrap()
                .tester_accepted
        );
    }
}
