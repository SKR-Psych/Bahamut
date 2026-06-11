use crate::commands::files::with_root_and_conn_optional;
use crate::commands::settings::get_settings_core;
use crate::{catalog, context, database, providers, AppState};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

#[derive(Debug, Serialize, Deserialize)]
pub struct ActiveModelRequest {
    pub model: Option<String>,
}

#[tauri::command]
pub fn get_hardware_profile() -> Result<catalog::HardwareProfile, String> {
    Ok(providers::hardware_profile())
}
#[tauri::command]
pub fn get_model_catalogue() -> Result<Vec<catalog::ModelCatalogueEntry>, String> {
    let c = providers::catalogue();
    catalog::validate_catalogue(&c)?;
    Ok(c)
}
#[tauri::command]
pub fn get_model_recommendations(
    profile: Option<catalog::HardwareProfile>,
) -> Result<Vec<catalog::ModelRecommendation>, String> {
    let p = profile.unwrap_or_else(providers::hardware_profile);
    Ok(providers::recommend(&p))
}
#[tauri::command]
pub async fn get_provider_status(
    state: State<'_, AppState>,
) -> Result<providers::ProviderStatus, String> {
    let settings = with_root_and_conn_optional(&state, |_r, c| get_settings_core(c))?;
    providers::provider_status(&settings.ai).await
}
#[tauri::command]
pub async fn reconnect_provider(
    state: State<'_, AppState>,
) -> Result<providers::ProviderStatus, String> {
    get_provider_status(state).await
}
#[tauri::command]
pub async fn list_installed_models(
    state: State<'_, AppState>,
) -> Result<Vec<providers::InstalledModel>, String> {
    let settings = with_root_and_conn_optional(&state, |_r, c| get_settings_core(c))?;
    providers::installed_models(&settings.ai).await
}
#[tauri::command]
pub async fn get_installed_model_metadata(
    state: State<'_, AppState>,
    model: String,
) -> Result<Option<providers::InstalledModel>, String> {
    Ok(list_installed_models(state)
        .await?
        .into_iter()
        .find(|m| m.id == model))
}
#[tauri::command]
pub async fn delete_model(state: State<'_, AppState>, model: String) -> Result<(), String> {
    let settings = with_root_and_conn_optional(&state, |_r, c| {
        database::log_action_with_conn(
            c,
            "delete_model",
            Some(serde_json::json!({"provider":"ollama","model":model}).to_string()),
            "requested",
            None,
        )?;
        get_settings_core(c)
    })?;
    providers::delete_model(&settings.ai, &model).await
}
#[tauri::command]
pub fn select_active_model(
    state: State<'_, AppState>,
    model: Option<String>,
) -> Result<crate::commands::settings::AppSettings, String> {
    with_root_and_conn_optional(&state, |_r, c| {
        let mut s = get_settings_core(c)?;
        s.ai.active_model = model;
        s.ai.local_ai_enabled = s.ai.active_model.is_some();
        crate::commands::settings::update_settings_core(c, &s)?;
        Ok(s)
    })
}
#[tauri::command]
pub async fn pull_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model: String,
) -> Result<(), String> {
    let settings = with_root_and_conn_optional(&state, |_r, c| get_settings_core(c))?;
    let gen = state.pull_generation.clone();
    providers::pull_model(app, settings.ai, model, gen).await
}
#[tauri::command]
pub fn cancel_model_pull(state: State<'_, AppState>) -> Result<(), String> {
    state.pull_generation.fetch_add(1, Ordering::SeqCst);
    Ok(())
}
#[tauri::command]
pub async fn test_prompt(
    state: State<'_, AppState>,
    app: AppHandle,
    model: String,
) -> Result<String, String> {
    let req = providers::ChatRequest {
        conversation_id: None,
        model,
        messages: vec![providers::ChatMessage {
            role: "user".into(),
            content: "Reply with exactly: Bahamut local AI ready".into(),
        }],
        temperature: Some(0.0),
        max_output_tokens: Some(32),
    };
    start_chat(state, app, req).await
}
#[tauri::command]
pub async fn start_chat(
    state: State<'_, AppState>,
    app: AppHandle,
    request: providers::ChatRequest,
) -> Result<String, String> {
    let settings = with_root_and_conn_optional(&state, |_r, c| get_settings_core(c))?;
    let gen = state.chat_generation.clone();
    providers::chat(app, settings.ai.clone(), request, gen).await
}
#[tauri::command]
pub fn cancel_chat(state: State<'_, AppState>) -> Result<(), String> {
    state.chat_generation.fetch_add(1, Ordering::SeqCst);
    Ok(())
}
#[tauri::command]
pub fn assemble_chat_context(
    state: State<'_, AppState>,
    attachments: Vec<context::AttachmentRequest>,
) -> Result<context::ContextAssembly, String> {
    with_root_and_conn_optional(&state, |root, c| {
        let settings = get_settings_core(c)?;
        context::assemble_context(
            root,
            attachments,
            settings.ai.per_file_attachment_limit,
            settings.ai.context_limit,
        )
    })
}
#[tauri::command]
pub fn approve_secret_context(
    state: State<'_, AppState>,
    categories: Vec<String>,
    attachment_count: usize,
) -> Result<(), String> {
    with_root_and_conn_optional(&state, |_r, c| {
        database::log_action_with_conn(
            c,
            "approve_secret_context",
            Some(
                serde_json::json!({"categories":categories,"attachment_count":attachment_count})
                    .to_string(),
            ),
            "success",
            None,
        )
    })
}

#[tauri::command]
pub fn create_conversation(
    state: State<'_, AppState>,
    title: String,
    model: Option<String>,
) -> Result<database::chat::Conversation, String> {
    with_root_and_conn_optional(&state, |_r, c| {
        let s = get_settings_core(c)?;
        if !s.ai.history_persistence {
            return Ok(database::chat::Conversation {
                id: 0,
                title,
                model,
                created_at: "persistence-disabled".into(),
                updated_at: "persistence-disabled".into(),
            });
        }
        database::chat::create_conversation(c, &title, model.as_deref())
    })
}
#[tauri::command]
pub fn list_conversations(
    state: State<'_, AppState>,
) -> Result<Vec<database::chat::Conversation>, String> {
    with_root_and_conn_optional(&state, |_r, c| {
        if !get_settings_core(c)?.ai.history_persistence {
            return Ok(vec![]);
        }
        database::chat::list_conversations(c)
    })
}
#[tauri::command]
pub fn read_conversation(
    state: State<'_, AppState>,
    id: i64,
) -> Result<database::chat::ConversationDetail, String> {
    with_root_and_conn_optional(&state, |_r, c| database::chat::read_conversation(c, id))
}
#[tauri::command]
pub fn rename_conversation(
    state: State<'_, AppState>,
    id: i64,
    title: String,
) -> Result<database::chat::Conversation, String> {
    with_root_and_conn_optional(&state, |_r, c| {
        database::chat::rename_conversation(c, id, &title)
    })
}
#[tauri::command]
pub fn delete_conversation(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    with_root_and_conn_optional(&state, |_r, c| database::chat::delete_conversation(c, id))
}
#[tauri::command]
pub fn clear_conversation_history(state: State<'_, AppState>) -> Result<(), String> {
    with_root_and_conn_optional(&state, |_r, c| database::chat::clear_history(c))
}
#[tauri::command]
pub fn inspect_stored_chat_data(
    state: State<'_, AppState>,
) -> Result<database::chat::StoredDataSummary, String> {
    with_root_and_conn_optional(&state, |_r, c| {
        let s = get_settings_core(c)?;
        database::chat::inspect_stored_data(c, s.ai.history_persistence)
    })
}
