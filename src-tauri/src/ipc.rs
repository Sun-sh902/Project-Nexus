//! Typed Tauri commands. Each is an explicit query or intent with a bounded, `deny_unknown_fields`
//! input; there is no generic execute/write/fetch command. Errors are `NexusError` (stable code +
//! safe params) which the frontend localizes. Only the bundled `main` window may call them.

use crate::actions::{ActionRequest, OperationResult, PlanView};
use crate::app::{AgentHistory, App, ModelsView, Overview, ResolvedAssignment};
use crate::core::{NexusError, NexusResult};
use crate::domain::AgentId;
use crate::models::{AssignmentMode, AuthMode, ModelDefinition, ModelProfile, ProviderKind, ProviderProfile};
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

pub type AppState = Arc<App>;

fn main_window(window: &tauri::WebviewWindow) -> NexusResult<()> {
    if window.label() != "main" {
        return Err(NexusError::new("ipc.window_denied"));
    }
    Ok(())
}

fn bounded(s: &str, max: usize) -> NexusResult<()> {
    if s.len() > max {
        return Err(NexusError::new("ipc.invalid_input"));
    }
    Ok(())
}

#[tauri::command]
pub async fn nexus_overview(window: tauri::WebviewWindow, state: State<'_, AppState>, refresh: bool) -> NexusResult<Overview> {
    main_window(&window)?;
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || app.overview(refresh)).await.map_err(|_| NexusError::new("app.task_failed"))?
}

#[tauri::command]
pub fn nexus_agent_history(window: tauri::WebviewWindow, state: State<'_, AppState>, agent_id: AgentId) -> NexusResult<AgentHistory> {
    main_window(&window)?;
    state.agent_history(agent_id)
}

#[tauri::command]
pub async fn nexus_models(window: tauri::WebviewWindow, state: State<'_, AppState>) -> NexusResult<ModelsView> {
    main_window(&window)?;
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || app.models_view()).await.map_err(|_| NexusError::new("app.task_failed"))?
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderInput {
    name: String,
    kind: ProviderKind,
    base_url: Option<String>,
    auth_mode: AuthMode,
    env_var: Option<String>,
}

#[tauri::command]
pub fn nexus_create_provider(window: tauri::WebviewWindow, state: State<'_, AppState>, input: ProviderInput) -> NexusResult<ProviderProfile> {
    main_window(&window)?;
    bounded(&input.name, 200)?;
    state.create_provider(&input.name, input.kind, input.base_url.as_deref(), input.auth_mode, input.env_var.as_deref())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretInput {
    provider_id: String,
    secret: String,
}

/// Write-only: the secret goes straight to the OS credential store. There is no read command.
#[tauri::command]
pub fn nexus_set_provider_secret(window: tauri::WebviewWindow, state: State<'_, AppState>, input: SecretInput) -> NexusResult<()> {
    main_window(&window)?;
    bounded(&input.provider_id, 64)?;
    state.set_provider_secret(&input.provider_id, input.secret)
}

#[tauri::command]
pub fn nexus_delete_provider(window: tauri::WebviewWindow, state: State<'_, AppState>, provider_id: String) -> NexusResult<()> {
    main_window(&window)?;
    bounded(&provider_id, 64)?;
    state.delete_provider(&provider_id)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelInput {
    provider_id: String,
    identifier: String,
    display_name: Option<String>,
}

#[tauri::command]
pub fn nexus_create_model(window: tauri::WebviewWindow, state: State<'_, AppState>, input: ModelInput) -> NexusResult<ModelDefinition> {
    main_window(&window)?;
    bounded(&input.provider_id, 64)?;
    state.create_model(&input.provider_id, &input.identifier, input.display_name.as_deref())
}

#[tauri::command]
pub fn nexus_delete_model(window: tauri::WebviewWindow, state: State<'_, AppState>, model_id: String) -> NexusResult<()> {
    main_window(&window)?;
    bounded(&model_id, 300)?;
    state.delete_model(&model_id)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileInput {
    id: Option<String>,
    name: String,
    model_id: String,
    reasoning: Option<String>,
}

#[tauri::command]
pub async fn nexus_save_profile(window: tauri::WebviewWindow, state: State<'_, AppState>, input: ProfileInput) -> NexusResult<ModelProfile> {
    main_window(&window)?;
    bounded(&input.model_id, 300)?;
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || app.save_profile(input.id.as_deref(), &input.name, &input.model_id, input.reasoning.as_deref())).await.map_err(|_| NexusError::new("app.task_failed"))?
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentInput {
    agent_id: AgentId,
    mode: AssignmentMode,
    model_id: Option<String>,
    profile_id: Option<String>,
}

#[tauri::command]
pub async fn nexus_set_assignment(window: tauri::WebviewWindow, state: State<'_, AppState>, input: AssignmentInput) -> NexusResult<Option<ResolvedAssignment>> {
    main_window(&window)?;
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || app.set_assignment(input.agent_id, input.mode, input.model_id.as_deref(), input.profile_id.as_deref())).await.map_err(|_| NexusError::new("app.task_failed"))?
}

#[tauri::command]
pub async fn nexus_prepare_action(window: tauri::WebviewWindow, state: State<'_, AppState>, request: ActionRequest) -> NexusResult<PlanView> {
    main_window(&window)?;
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || app.prepare(request)).await.map_err(|_| NexusError::new("app.task_failed"))?
}

/// The renderer can only *request* execution of a plan Rust prepared; Rust shows the native
/// confirmation dialog and the plan is consumed whether or not the user approves.
#[tauri::command]
pub async fn nexus_confirm_plan(window: tauri::WebviewWindow, state: State<'_, AppState>, plan_id: String, digest: String) -> NexusResult<OperationResult> {
    main_window(&window)?;
    bounded(&plan_id, 64)?;
    bounded(&digest, 64)?;
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || app.confirm(&plan_id, &digest)).await.map_err(|_| NexusError::new("app.task_failed"))?
}

#[tauri::command]
pub fn nexus_discard_plan(window: tauri::WebviewWindow, state: State<'_, AppState>, plan_id: String) -> NexusResult<()> {
    main_window(&window)?;
    bounded(&plan_id, 64)?;
    state.discard_plan(&plan_id)
}

#[tauri::command]
pub async fn nexus_launch_agent(window: tauri::WebviewWindow, state: State<'_, AppState>, agent_id: AgentId, installation_id: String) -> NexusResult<String> {
    main_window(&window)?;
    let app = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || app.launch(agent_id, &installation_id)).await.map_err(|_| NexusError::new("app.task_failed"))?
}

#[tauri::command]
pub fn nexus_set_locale(window: tauri::WebviewWindow, state: State<'_, AppState>, locale: String) -> NexusResult<()> {
    main_window(&window)?;
    bounded(&locale, 8)?;
    state.set_locale(&locale);
    Ok(())
}

#[cfg(test)]
pub const COMMANDS: &[&str] = &[
    "nexus_overview",
    "nexus_agent_history",
    "nexus_models",
    "nexus_create_provider",
    "nexus_set_provider_secret",
    "nexus_delete_provider",
    "nexus_create_model",
    "nexus_delete_model",
    "nexus_save_profile",
    "nexus_set_assignment",
    "nexus_prepare_action",
    "nexus_confirm_plan",
    "nexus_discard_plan",
    "nexus_launch_agent",
    "nexus_set_locale",
];

#[cfg(test)]
mod tests {
    #[test]
    fn every_command_is_registered_and_allowlisted() {
        let build = include_str!("../build.rs");
        let caps = include_str!("../capabilities/main.json");
        let lib = include_str!("lib.rs");
        for c in super::COMMANDS {
            assert!(build.contains(&format!("\"{c}\"")), "{c} missing from build.rs");
            assert!(caps.contains(&format!("allow-{}", c.replace('_', "-"))), "{c} missing from capability");
            assert!(lib.contains(&format!("ipc::{c}")), "{c} missing from handler");
        }
        // No broad plugin permissions are granted to the renderer.
        for forbidden in ["shell:", "fs:", "dialog:", "http:", "process:"] {
            assert!(!caps.contains(forbidden), "{forbidden}");
        }
    }
}
