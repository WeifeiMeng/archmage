use archmage_core::{
    agent::{AgentInstanceInput, AgentService, ConnectionState, ProviderKind},
    capability::{CapabilityConfigurationInput, CapabilityGuidance, CapabilityService},
    config::Config,
    store::Store,
};

#[test]
fn capability_config_is_seeded_and_saved_with_revision_guard() {
    let (_temp, mut store) = fixture_store();
    assert_eq!(store.schema_version().unwrap(), 14);
    let configs = store.capability_configurations().unwrap();
    assert_eq!(configs.len(), 4);
    assert_eq!(configs[0].key, "designer");
    assert_eq!(configs[0].agent_instance_id, "codex-default");

    let input = CapabilityConfigurationInput {
        key: "designer".into(),
        display_name: "Product Designer".into(),
        description: "Plan the change".into(),
        extra_instructions: "  Prefer concise designs\n".into(),
        skills: vec![CapabilityGuidance {
            name: "brainstorming".into(),
            purpose: "Explore alternatives before design".into(),
        }],
        tools: vec![CapabilityGuidance {
            name: "web_search".into(),
            purpose: "Research uncertain facts".into(),
        }],
        agent_instance_id: "codex-default".into(),
        revision: configs[0].revision,
    };
    store.save_capability_configuration(&input).unwrap();
    let saved = &store.capability_configurations().unwrap()[0];
    assert_eq!(saved.display_name, "Product Designer");
    assert_eq!(saved.extra_instructions, "  Prefer concise designs\n");
    assert_eq!(saved.skills, input.skills);
    assert_eq!(saved.tools, input.tools);
    assert_eq!(saved.revision, input.revision + 1);
    assert!(store.save_capability_configuration(&input).is_err());
    assert!(
        store
            .save_capability_configuration(&CapabilityConfigurationInput {
                key: "unknown".into(),
                ..input
            })
            .is_err()
    );
}

#[test]
fn capability_guidance_rejects_duplicate_names_and_blank_purposes() {
    let (_temp, mut store) = fixture_store();
    let current = store.capability_configurations().unwrap().remove(0);
    let mut input = CapabilityConfigurationInput {
        key: current.key,
        display_name: current.display_name,
        description: current.description,
        extra_instructions: String::new(),
        skills: vec![
            CapabilityGuidance {
                name: "brainstorming".into(),
                purpose: "Explore ideas".into(),
            },
            CapabilityGuidance {
                name: "Brainstorming".into(),
                purpose: "Duplicate".into(),
            },
        ],
        tools: vec![],
        agent_instance_id: current.agent_instance_id,
        revision: current.revision,
    };
    assert!(store.save_capability_configuration(&input).is_err());
    input.skills.pop();
    input.skills[0].purpose = "  ".into();
    assert!(store.save_capability_configuration(&input).is_err());
}

#[test]
fn capability_snapshot_exposes_fixed_execution_and_handoff_edges() {
    let temp = tempfile::tempdir().unwrap();
    let config = Config::resolve(Some(&temp.path().join("data")), None).unwrap();
    config.prepare().unwrap();
    let snapshot = CapabilityService::new(config.data_root).snapshot().unwrap();
    assert_eq!(snapshot.configurations.len(), 4);
    assert_eq!(
        snapshot
            .edges
            .iter()
            .filter(|edge| edge.kind == "execution")
            .count(),
        3
    );
    assert_eq!(
        snapshot
            .edges
            .iter()
            .filter(|edge| edge.kind == "context")
            .count(),
        3
    );
    assert!(
        snapshot
            .edges
            .iter()
            .any(|edge| edge.from == "designer" && edge.to == "reviewer" && edge.kind == "context")
    );
}

#[test]
fn legacy_binding_update_invalidates_stale_capability_editor() {
    let (_temp, mut store) = fixture_store();
    let before = store.capability_configurations().unwrap()[0].revision;
    store.bind_capability("designer", "codex-default").unwrap();
    assert_eq!(
        store.capability_configurations().unwrap()[0].revision,
        before + 1
    );
}

#[test]
fn schema_eight_upgrade_keeps_existing_agent_bindings() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("archmage.db");
    drop(Store::open(&path).unwrap());
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch(
            "DROP TABLE flow_deliveries; DROP TABLE consultation_processes; DROP TABLE revision_feedback; DROP TABLE consultations; DROP TABLE teams; DROP TABLE capability_configurations; DELETE FROM schema_migrations WHERE version IN (9,10,11,12,13,14);",
        )
        .unwrap();
    let upgraded = Store::open(&path).unwrap();
    assert_eq!(upgraded.schema_version().unwrap(), 14);
    assert_eq!(upgraded.capability_configurations().unwrap().len(), 4);
    assert_eq!(
        upgraded.capability_bindings().unwrap()[0].agent_instance_id,
        "codex-default"
    );
}

fn fixture_store() -> (tempfile::TempDir, Store) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(&temp.path().join("archmage.db")).unwrap();
    (temp, store)
}

#[test]
fn seeds_one_codex_instance_and_binds_all_builtin_capabilities() {
    let (_temp, store) = fixture_store();
    let snapshot = store.agent_management_snapshot().unwrap();

    assert_eq!(snapshot.instances.len(), 1);
    assert_eq!(snapshot.instances[0].id, "codex-default");
    assert_eq!(snapshot.instances[0].provider_kind, ProviderKind::Codex);
    assert_eq!(snapshot.bindings.len(), 4);
    assert!(
        snapshot
            .bindings
            .iter()
            .all(|binding| binding.agent_instance_id == "codex-default")
    );
}

#[test]
fn stores_multiple_codex_instances_with_stable_ids() {
    let (_temp, mut store) = fixture_store();
    store
        .save_agent_instance(&AgentInstanceInput {
            id: "codex-work".into(),
            provider_kind: ProviderKind::Codex,
            display_name: "Codex Work".into(),
            executable_path: Some("/opt/codex/bin/codex".into()),
            model: Some("gpt-5.4".into()),
            enabled: true,
        })
        .unwrap();

    let instances = store.agent_instances().unwrap();
    assert_eq!(instances.len(), 2);
    assert!(instances.iter().any(|instance| instance.id == "codex-work"));
}

#[test]
fn changing_execution_config_invalidates_verification() {
    let (_temp, mut store) = fixture_store();
    store
        .record_agent_ready("codex-default", "codex 1.2.3", "thread-1")
        .unwrap();
    let verified = store.agent_instance("codex-default").unwrap();
    assert_eq!(verified.connection_state, ConnectionState::Ready);

    store
        .save_agent_instance(&AgentInstanceInput {
            id: "codex-default".into(),
            provider_kind: ProviderKind::Codex,
            display_name: "Codex".into(),
            executable_path: Some("/opt/codex/bin/codex".into()),
            model: None,
            enabled: true,
        })
        .unwrap();

    let changed = store.agent_instance("codex-default").unwrap();
    assert_eq!(changed.connection_state, ConnectionState::Unverified);
    assert_eq!(changed.discovered_version, None);
    assert_eq!(changed.verification_thread_id, None);
}

#[test]
fn bound_instance_cannot_be_disabled_or_deleted() {
    let (_temp, mut store) = fixture_store();
    let disabled = AgentInstanceInput {
        id: "codex-default".into(),
        provider_kind: ProviderKind::Codex,
        display_name: "Codex".into(),
        executable_path: None,
        model: None,
        enabled: false,
    };
    assert!(store.save_agent_instance(&disabled).is_err());
    assert!(store.delete_agent_instance("codex-default").is_err());
    store
        .save_agent_instance(&AgentInstanceInput {
            id: "codex-extra".into(),
            display_name: "Extra".into(),
            enabled: true,
            ..disabled
        })
        .unwrap();
    store.delete_agent_instance("codex-extra").unwrap();
    assert!(store.agent_instance("codex-extra").is_err());
}

#[test]
fn rediscovering_same_version_preserves_successful_verification() {
    let (_temp, mut store) = fixture_store();
    store
        .record_agent_ready("codex-default", "codex 1.2.3", "thread-1")
        .unwrap();
    store
        .record_agent_discovered("codex-default", "codex 1.2.3")
        .unwrap();
    let instance = store.agent_instance("codex-default").unwrap();
    assert_eq!(instance.connection_state, ConnectionState::Ready);
    assert_eq!(instance.verification_thread_id.as_deref(), Some("thread-1"));
    store
        .record_agent_discovered("codex-default", "codex 2.0.0")
        .unwrap();
    assert_eq!(
        store
            .agent_instance("codex-default")
            .unwrap()
            .connection_state,
        ConnectionState::Unverified
    );
}

#[test]
fn failed_cli_discovery_does_not_record_an_online_verification_time() {
    let (_temp, mut store) = fixture_store();
    store
        .record_agent_discovery_failed("codex-default")
        .unwrap();
    let instance = store.agent_instance("codex-default").unwrap();
    assert_eq!(instance.connection_state, ConnectionState::Failed);
    assert!(instance.last_verified_at.is_none());
    assert!(instance.discovered_version.is_none());
}

#[test]
fn renaming_a_failed_instance_preserves_its_failure_message() {
    let (_temp, mut store) = fixture_store();
    store
        .record_agent_failed("codex-default", "Codex connection verification failed")
        .unwrap();
    store
        .save_agent_instance(&AgentInstanceInput {
            id: "codex-default".into(),
            provider_kind: ProviderKind::Codex,
            display_name: "Renamed".into(),
            executable_path: None,
            model: None,
            enabled: true,
        })
        .unwrap();
    let instance = store.agent_instance("codex-default").unwrap();
    assert_eq!(instance.connection_state, ConnectionState::Failed);
    assert_eq!(
        instance.last_error.as_deref(),
        Some("Codex connection verification failed")
    );
}

#[cfg(unix)]
#[tokio::test]
async fn stale_verification_cannot_mark_a_changed_model_ready() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let config = Config::resolve(Some(&temp.path().join("data")), None).unwrap();
    config.prepare().unwrap();
    let service = AgentService::new(config.data_root.clone());
    let binary = temp.path().join("codex-test");
    let marker = temp.path().join("probe-started");
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'codex 1.2.3'; exit 0; fi\nif [ \"$1\" = \"exec\" ] && [ \"$2\" = \"--help\" ]; then echo '--cd --sandbox --json --output-schema --output-last-message'; exit 0; fi\noutput=''\nwhile [ $# -gt 0 ]; do\n  case \"$1\" in --output-last-message) output=\"$2\"; shift 2;; *) shift;; esac\ndone\ntouch '{}'\nsleep 1\nprintf '{{\"ok\":true}}' > \"$output\"\nprintf '%s\\n' '{{\"type\":\"thread.started\",\"thread_id\":\"thread-test\"}}' '{{\"type\":\"turn.completed\"}}'\n",
        marker.display()
    );
    std::fs::write(&binary, script).unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    service
        .save(&AgentInstanceInput {
            id: "codex-race".into(),
            provider_kind: ProviderKind::Codex,
            display_name: "Race".into(),
            executable_path: Some(binary.clone()),
            model: None,
            enabled: true,
        })
        .unwrap();
    let pending = tokio::spawn({
        let service = service.clone();
        async move { service.verify("codex-race").await.unwrap() }
    });
    for _ in 0..200 {
        if marker.exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(marker.exists(), "verification did not begin");
    service
        .save(&AgentInstanceInput {
            id: "codex-race".into(),
            provider_kind: ProviderKind::Codex,
            display_name: "Race".into(),
            executable_path: Some(binary),
            model: Some("new-model".into()),
            enabled: true,
        })
        .unwrap();
    pending.await.unwrap();
    let instance = service
        .snapshot()
        .unwrap()
        .instances
        .into_iter()
        .find(|item| item.id == "codex-race")
        .unwrap();
    assert_eq!(instance.connection_state, ConnectionState::Unverified);
    assert_eq!(instance.model.as_deref(), Some("new-model"));
}

#[tokio::test]
async fn verification_failure_is_persisted_as_safe_state() {
    let temp = tempfile::tempdir().unwrap();
    let data_root = temp.path().join("data");
    let config = Config::resolve(Some(&data_root), None).unwrap();
    config.prepare().unwrap();
    let service = AgentService::new(config.data_root.clone());
    service
        .save(&AgentInstanceInput {
            id: "codex-broken".into(),
            provider_kind: ProviderKind::Codex,
            display_name: "Broken Codex".into(),
            executable_path: Some(temp.path().join("missing-codex")),
            model: None,
            enabled: true,
        })
        .unwrap();

    let snapshot = service.verify("codex-broken").await.unwrap();
    let instance = snapshot
        .instances
        .iter()
        .find(|instance| instance.id == "codex-broken")
        .unwrap();
    assert_eq!(instance.connection_state, ConnectionState::Failed);
    assert_eq!(
        instance.last_error.as_deref(),
        Some("Codex executable inspection failed")
    );
}

#[cfg(unix)]
#[tokio::test]
async fn discovery_does_not_claim_online_verification() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let config = Config::resolve(Some(&temp.path().join("data")), None).unwrap();
    config.prepare().unwrap();
    let service = AgentService::new(config.data_root.clone());
    let binary = temp.path().join("codex-test");
    std::fs::write(&binary, "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'codex 1.2.3'; else echo '--cd --sandbox --json --output-schema --output-last-message'; fi\n").unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    service
        .save(&AgentInstanceInput {
            id: "codex-discovery".into(),
            provider_kind: ProviderKind::Codex,
            display_name: "Discovery".into(),
            executable_path: Some(binary),
            model: None,
            enabled: true,
        })
        .unwrap();

    let snapshot = service.discover("codex-discovery").await.unwrap();
    let instance = snapshot
        .instances
        .iter()
        .find(|item| item.id == "codex-discovery")
        .unwrap();
    assert_eq!(instance.discovered_version.as_deref(), Some("codex 1.2.3"));
    assert_eq!(instance.connection_state, ConnectionState::Unverified);
    assert!(instance.last_verified_at.is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn configured_model_is_used_for_real_agent_verification() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let config = Config::resolve(Some(&temp.path().join("data")), None).unwrap();
    config.prepare().unwrap();
    let service = AgentService::new(config.data_root.clone());
    let binary = temp.path().join("codex-test");
    std::fs::write(&binary, "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'codex 1.2.3'; exit 0; fi\nif [ \"$1\" = \"exec\" ] && [ \"$2\" = \"--help\" ]; then echo '--cd --sandbox --json --output-schema --output-last-message'; exit 0; fi\nmodel=''\noutput=''\nwhile [ $# -gt 0 ]; do\n  case \"$1\" in\n    --model) model=\"$2\"; shift 2;;\n    --output-last-message) output=\"$2\"; shift 2;;\n    *) shift;;\n  esac\ndone\n[ \"$model\" = 'test-model' ] || exit 7\nprintf '{\"ok\":true}' > \"$output\"\nprintf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"thread-test\"}' '{\"type\":\"turn.completed\"}'\n").unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    service
        .save(&AgentInstanceInput {
            id: "codex-model".into(),
            provider_kind: ProviderKind::Codex,
            display_name: "Model".into(),
            executable_path: Some(binary),
            model: Some("test-model".into()),
            enabled: true,
        })
        .unwrap();

    let snapshot = service.verify("codex-model").await.unwrap();
    let instance = snapshot
        .instances
        .iter()
        .find(|item| item.id == "codex-model")
        .unwrap();
    assert_eq!(instance.connection_state, ConnectionState::Ready);
    assert_eq!(
        instance.verification_thread_id.as_deref(),
        Some("thread-test")
    );
}
