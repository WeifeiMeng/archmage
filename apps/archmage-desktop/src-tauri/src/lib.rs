use archmage_core::{
    agent::{AgentInstanceInput, AgentManagementSnapshot},
    capability::{CapabilityConfigurationInput, CapabilityManagementSnapshot},
    config::Config,
    event::Event,
    provider::codex,
    runtime::{RunRequest, RunSnapshot, RunSummary, Runtime},
    team::{Team, TeamGraph},
};
use serde::Serialize;
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{Emitter, Manager, State};

struct AppState {
    runtime: Runtime,
    active_runs: Arc<Mutex<HashSet<String>>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunEventSignal {
    run_id: String,
    event_sequence: i64,
}

#[tauri::command]
async fn doctor(online: bool) -> Result<serde_json::Value, String> {
    let tools = codex::inspect().await.map_err(|error| error.to_string())?;
    let probe = if online {
        Some(
            codex::readonly_probe(&tools)
                .await
                .map_err(|error| error.to_string())?,
        )
    } else {
        None
    };
    Ok(serde_json::json!({
        "tools": tools,
        "authentication": if online { "verified" } else { "not_verified_offline" },
        "probe": probe,
    }))
}

#[tauri::command]
fn get_agent_management(state: State<'_, AppState>) -> Result<AgentManagementSnapshot, String> {
    state
        .runtime
        .agent_management()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_agent_instance(
    state: State<'_, AppState>,
    input: AgentInstanceInput,
) -> Result<AgentManagementSnapshot, String> {
    state
        .runtime
        .agent_service()
        .save(&input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn verify_agent_instance(
    state: State<'_, AppState>,
    agent_instance_id: String,
) -> Result<AgentManagementSnapshot, String> {
    let service = state.runtime.agent_service();
    service
        .verify(&agent_instance_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn discover_agent_instance(
    state: State<'_, AppState>,
    agent_instance_id: String,
) -> Result<AgentManagementSnapshot, String> {
    state
        .runtime
        .agent_service()
        .discover(&agent_instance_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_agent_instance(
    state: State<'_, AppState>,
    agent_instance_id: String,
) -> Result<AgentManagementSnapshot, String> {
    state
        .runtime
        .agent_service()
        .delete(&agent_instance_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn bind_capability(
    state: State<'_, AppState>,
    capability: String,
    agent_instance_id: String,
) -> Result<AgentManagementSnapshot, String> {
    state
        .runtime
        .agent_service()
        .bind(&capability, &agent_instance_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_capability_management(
    state: State<'_, AppState>,
) -> Result<CapabilityManagementSnapshot, String> {
    state
        .runtime
        .capability_service()
        .snapshot()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_capability_configuration(
    state: State<'_, AppState>,
    input: CapabilityConfigurationInput,
) -> Result<CapabilityManagementSnapshot, String> {
    state
        .runtime
        .capability_service()
        .save(&input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn list_teams(state: State<'_, AppState>) -> Result<Vec<Team>, String> {
    state.runtime.team_service().list().map_err(|error| error.to_string())
}

#[tauri::command]
fn create_team(state: State<'_, AppState>, name: String, graph: TeamGraph) -> Result<Team, String> {
    state.runtime.team_service().create(&name, &graph).map_err(|error| error.to_string())
}

#[tauri::command]
fn save_team(state: State<'_, AppState>, id: String, revision: i64, name: String, graph: TeamGraph) -> Result<Team, String> {
    state.runtime.team_service().save(&id, revision, &name, &graph).map_err(|error| error.to_string())
}

#[tauri::command]
async fn start_run(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: RunRequest,
) -> Result<String, String> {
    let run_id = state
        .runtime
        .start_run(&request)
        .await
        .map_err(|error| error.to_string())?;
    state.active_runs.lock().unwrap().insert(run_id.clone());
    let runtime = state.runtime.clone();
    let active_runs = state.active_runs.clone();
    let task_run_id = run_id.clone();
    app.emit(
        "run-event",
        RunEventSignal {
            run_id: run_id.clone(),
            event_sequence: 0,
        },
    )
    .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn(async move {
        let _ = runtime.run_to_terminal(&task_run_id).await;
        active_runs.lock().unwrap().remove(&task_run_id);
        let sequence = runtime
            .snapshot(&task_run_id)
            .map(|snapshot| snapshot.last_event_sequence)
            .unwrap_or_default();
        let _ = app.emit(
            "run-event",
            RunEventSignal {
                run_id: task_run_id,
                event_sequence: sequence,
            },
        );
    });
    Ok(run_id)
}

#[tauri::command]
fn list_runs(state: State<'_, AppState>) -> Result<Vec<RunSummary>, String> {
    state.runtime.list_runs().map_err(|error| error.to_string())
}

#[tauri::command]
fn active_run_ids(state: State<'_, AppState>) -> Vec<String> {
    state.active_runs.lock().unwrap().iter().cloned().collect()
}

#[tauri::command]
fn get_run_snapshot(state: State<'_, AppState>, run_id: String) -> Result<RunSnapshot, String> {
    state
        .runtime
        .snapshot(&run_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn list_events(
    state: State<'_, AppState>,
    run_id: String,
    after_sequence: i64,
) -> Result<Vec<Event>, String> {
    state
        .runtime
        .events(&run_id, after_sequence)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn cancel_run(state: State<'_, AppState>, run_id: String) -> Result<(), String> {
    state
        .runtime
        .cancel(&run_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn reveal_workspace(state: State<'_, AppState>, run_id: String) -> Result<(), String> {
    let workspace = state
        .runtime
        .snapshot(&run_id)
        .map_err(|error| error.to_string())?
        .workspace
        .ok_or_else(|| "Run workspace is not ready".to_owned())?;
    reveal(&workspace)
}

#[cfg(target_os = "macos")]
fn reveal(path: &std::path::Path) -> Result<(), String> {
    let status = std::process::Command::new("/usr/bin/open")
        .arg("-R")
        .arg(path)
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("Finder could not reveal the workspace".into())
    }
}

#[cfg(not(target_os = "macos"))]
fn reveal(_path: &std::path::Path) -> Result<(), String> {
    Err("Reveal workspace is currently implemented for macOS".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let root = std::env::var_os("ARCHMAGE_HOME").map(PathBuf::from);
            let runtime = Runtime::open(Config::resolve(root.as_deref(), None)?)?;
            runtime.reconcile_interrupted()?;
            app.manage(AppState {
                runtime,
                active_runs: Arc::new(Mutex::new(HashSet::new())),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            doctor,
            get_agent_management,
            save_agent_instance,
            discover_agent_instance,
            verify_agent_instance,
            delete_agent_instance,
            bind_capability,
            get_capability_management,
            save_capability_configuration,
            list_teams,
            create_team,
            save_team,
            start_run,
            list_runs,
            active_run_ids,
            get_run_snapshot,
            list_events,
            cancel_run,
            reveal_workspace,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Archmage desktop");
}
