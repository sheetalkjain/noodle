#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use agent::engine::SyncManager;
use agent::pipeline::ExtractionPipeline;
use ai::provider::{AiProvider, OllamaProvider, OpenAICompatibleProvider};
use outlook::client::OutlookClient;
use std::sync::Arc;
use storage::qdrant::QdrantStorage;
use storage::sqlite::SqliteStorage;
use tauri::{command, Emitter, Manager, State};
use tokio::sync::RwLock;
use tracing::{error, info};

struct AppState {
    sqlite: Arc<SqliteStorage>,
    qdrant: Arc<QdrantStorage>,
    ai: Arc<RwLock<Arc<dyn AiProvider>>>, // Wrap in RwLock for runtime updates
    pipeline: Arc<ExtractionPipeline>,
    outlook: Arc<OutlookClient>,
    app_handle: tauri::AppHandle,
}

#[command]
async fn search_emails(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<serde_json::Value>, String> {
    // If query is empty, return recent 50 emails
    if query.trim().is_empty() {
        return state
            .sqlite
            .get_recent_emails(50)
            .await
            .map_err(|e| e.to_string());
    }

    // 1. Generate embedding for query
    // 1. Generate embedding for query
    let ai = state.ai.read().await;
    let embedding = ai
        .generate_embedding(&query)
        .await
        .map_err(|e| e.to_string())?;

    // 2. Vector Search in Qdrant
    let results = state
        .qdrant
        .search_emails(embedding, None, 20)
        .await
        .map_err(|e| e.to_string())?;

    // 3. Fetch full email data from SQLite using internal IDs
    let ids: Vec<i64> = results
        .into_iter()
        .filter_map(|r| {
            r.id.and_then(|id| id.point_id_options)
                .and_then(|id| match id {
                    qdrant_client::qdrant::point_id::PointIdOptions::Num(num) => Some(num as i64),
                    _ => None,
                })
        })
        .collect();

    state
        .sqlite
        .get_emails_by_ids(ids)
        .await
        .map_err(|e| e.to_string())
}

#[command]
async fn get_graph(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    state.sqlite.get_entities().await.map_err(|e| e.to_string())
}

#[command]
async fn get_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    state
        .sqlite
        .get_dashboard_stats()
        .await
        .map_err(|e| e.to_string())
}

#[command]
async fn start_sync(state: State<'_, AppState>) -> Result<(), String> {
    info!("Manual sync requested");
    let app_handle = state.app_handle.clone();
    let _ = app_handle.emit(
        "noodle://log",
        serde_json::json!({
            "message": "Manual sync started",
            "level": "info"
        }),
    );

    let history_days = state
        .sqlite
        .get_config("history_days")
        .await
        .unwrap_or(None)
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(90);

    let sync_interval = state
        .sqlite
        .get_config("sync_interval")
        .await
        .unwrap_or(None)
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(2);

    let sync_manager = Arc::new(SyncManager::new(
        state.pipeline.clone(),
        state.outlook.clone(),
        state.sqlite.clone(),
        state.app_handle.clone(),
        history_days,
        sync_interval,
    ));

    tokio::spawn(async move {
        sync_manager.start_background_sync().await;
    });

    Ok(())
}

#[command]
async fn get_logs(
    state: State<'_, AppState>,
    limit: i64,
) -> Result<Vec<serde_json::Value>, String> {
    state
        .sqlite
        .get_logs(limit)
        .await
        .map_err(|e: noodle_core::error::NoodleError| e.to_string())
}

#[command]
async fn get_config(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    state
        .sqlite
        .get_config(&key)
        .await
        .map_err(|e: noodle_core::error::NoodleError| e.to_string())
}

#[command]
async fn save_config(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    state
        .sqlite
        .set_config(&key, &value)
        .await
        .map_err(|e: noodle_core::error::NoodleError| e.to_string())?;

    // If AI settings changed, re-initialize provider
    if key == "ollama_url" || key == "model_name" || key == "provider_type" || key == "api_key" {
        let provider_type = state
            .sqlite
            .get_config("provider_type")
            .await
            .unwrap_or(Some("ollama".to_string()))
            .unwrap_or("ollama".to_string());

        let url = match provider_type.as_str() {
            "lemonade" => state
                .sqlite
                .get_config("lemonade_url")
                .await
                .unwrap_or(Some("http://localhost:8000/v1".to_string()))
                .unwrap_or("http://localhost:8000/v1".to_string()),
            "foundry" => state
                .sqlite
                .get_config("foundry_url")
                .await
                .unwrap_or(Some("http://localhost:5000/v1".to_string()))
                .unwrap_or("http://localhost:5000/v1".to_string()),
            "openai" | _ => state
                .sqlite
                .get_config("ollama_url")
                .await
                .unwrap_or(Some("http://localhost:11434".to_string()))
                .unwrap_or("http://localhost:11434".to_string()),
        };

        let model = state.sqlite.get_config("model_name").await.unwrap_or(None);
        let api_key = state.sqlite.get_config("api_key").await.unwrap_or(None);

        let new_provider: Arc<dyn AiProvider> = if provider_type == "ollama" {
            Arc::new(OllamaProvider::new(url, model))
        } else {
            // Lemonade, Foundry, and OpenAI all use OpenAI-compatible API
            Arc::new(OpenAICompatibleProvider::new(url, api_key, model))
        };

        let mut ai_lock = state.ai.write().await;
        *ai_lock = new_provider;
        info!("Re-initialized AI provider: {}", provider_type);
    }
    Ok(())
}

#[command]
async fn get_models(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let ai = state.ai.read().await;
    ai.list_models().await.map_err(|e| e.to_string())
}

#[command]
async fn save_log_cmd(
    state: State<'_, AppState>,
    level: String,
    source: String,
    message: String,
) -> Result<(), String> {
    state
        .sqlite
        .save_log(&level, &source, &message, None)
        .await
        .map_err(|e: noodle_core::error::NoodleError| e.to_string())
}

#[command]
async fn get_email(state: State<'_, AppState>, id: i64) -> Result<serde_json::Value, String> {
    use sqlx::Row;
    let email = sqlx::query("SELECT * FROM emails WHERE id = ?")
        .bind(id)
        .fetch_optional(state.sqlite.pool())
        .await
        .map_err(|e| e.to_string())?;

    match email {
        Some(row) => Ok(serde_json::json!({
            "id": row.get::<i64, _>("id"),
            "subject": row.get::<String, _>("subject"),
            "sender": row.get::<String, _>("sender"),
            "received_at": row.get::<chrono::DateTime<chrono::Utc>, _>("received_at"),
            "body_text": row.get::<String, _>("body_text")
        })),
        None => Err("Email not found".into()),
    }
}

#[command]
async fn list_prompts(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    use sqlx::Row;
    // Return empty list if table doesn't exist yet, but use real query
    let results = sqlx::query("SELECT id, name, prompt_template FROM prompts")
        .fetch_all(state.sqlite.pool())
        .await
        .unwrap_or_else(|_| vec![]);

    Ok(results
        .into_iter()
        .map(|r: sqlx::sqlite::SqliteRow| {
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "name": r.get::<String, _>("name"),
                "content": r.get::<String, _>("prompt_template")
            })
        })
        .collect::<Vec<_>>())
}

#[command]
async fn save_prompt(state: State<'_, AppState>, prompt: serde_json::Value) -> Result<(), String> {
    use uuid::Uuid;
    let now = chrono::Utc::now();
    sqlx::query("INSERT INTO prompts (id, name, kind, scope_json, model_pref_json, prompt_template, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(Uuid::new_v4().to_string())
        .bind(prompt["name"].as_str().unwrap_or("Untitled"))
        .bind("custom")
        .bind("{}")
        .bind("{}")
        .bind(prompt["content"].as_str().unwrap_or(""))
        .bind(now)
        .bind(now)
        .execute(state.sqlite.pool())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
async fn draft_reply(state: State<'_, AppState>, email_id: i64) -> Result<String, String> {
    use sqlx::Row;
    let email = sqlx::query("SELECT body_text FROM emails WHERE id = ?")
        .bind(email_id)
        .fetch_optional(state.sqlite.pool())
        .await
        .map_err(|e| e.to_string())?;

    if let Some(row) = email {
        let body: String = row.get("body_text");
        let prompt = format!("Draft a professional reply to this email: {}", body);
        let request = ai::provider::ChatRequest {
            messages: vec![ai::provider::Message {
                role: "user".into(),
                content: prompt,
            }],
            temperature: 0.7,
            response_format: None,
            model: None,
        };
        let ai = state.ai.read().await;
        let response = ai
            .chat_completion(request)
            .await
            .map_err(|e| e.to_string())?;
        Ok(response.content)
    } else {
        Err("Email not found".into())
    }
}

#[command]
async fn force_exit(app_handle: tauri::AppHandle) {
    app_handle.exit(0);
}

#[command]
async fn request_exit(state: State<'_, AppState>) -> Result<(), String> {
    let confirm = state
        .sqlite
        .get_config("confirm_exit")
        .await
        .unwrap_or(Some("true".to_string()))
        .unwrap_or("true".to_string())
        != "false";

    if confirm {
        let window = state
            .app_handle
            .get_webview_window("main")
            .ok_or("No main window")?;
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        state
            .app_handle
            .emit("noodle://show-exit-confirm", ())
            .map_err(|e| e.to_string())?;
    } else {
        state.app_handle.exit(0);
    }
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Initialize Tray
            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::with_id("tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "quit" => {
                        let app_clone = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app_clone.state::<AppState>();
                            let confirm = state
                                .sqlite
                                .get_config("confirm_exit")
                                .await
                                .unwrap_or(Some("true".to_string()))
                                .unwrap_or("true".to_string())
                                != "false";

                            if confirm {
                                if let Some(window) = app_clone.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                    let _ = app_clone.emit("noodle://show-exit-confirm", ());
                                }
                            } else {
                                app_clone.exit(0);
                            }
                        });
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            tauri::async_runtime::block_on(async move {
                let app_dir = match app_handle.path().app_data_dir() {
                    Ok(path) => path,
                    Err(e) => {
                        error!("Failed to get app data dir: {}", e);
                        return;
                    }
                };

                if let Err(e) = std::fs::create_dir_all(&app_dir) {
                    error!("Failed to create app data dir: {}", e);
                }

                let db_path = app_dir.join("noodle.db");
                let sqlite = match SqliteStorage::new(db_path).await {
                    Ok(s) => Arc::new(s),
                    Err(e) => {
                        error!("Failed to initialize SQLite: {}", e);
                        return;
                    }
                };

                let qdrant = match QdrantStorage::new("http://localhost:6334").await {
                    Ok(q) => Arc::new(q),
                    Err(e) => {
                        error!("Failed to initialize Qdrant: {}", e);
                        return;
                    }
                };

                let provider_type = sqlite
                    .get_config("provider_type")
                    .await
                    .unwrap_or(Some("ollama".into()))
                    .unwrap_or("ollama".into());

                let url = match provider_type.as_str() {
                    "lemonade" => sqlite
                        .get_config("lemonade_url")
                        .await
                        .unwrap_or(Some("http://localhost:8000/v1".to_string()))
                        .unwrap_or("http://localhost:8000/v1".to_string()),
                    "foundry" => sqlite
                        .get_config("foundry_url")
                        .await
                        .unwrap_or(Some("http://localhost:5000/v1".to_string()))
                        .unwrap_or("http://localhost:5000/v1".to_string()),
                    "openai" | _ => sqlite
                        .get_config("ollama_url")
                        .await
                        .unwrap_or(Some("http://localhost:11434".to_string()))
                        .unwrap_or("http://localhost:11434".to_string()),
                };

                let model = sqlite.get_config("model_name").await.unwrap_or(None);
                let api_key = sqlite.get_config("api_key").await.unwrap_or(None);

                let ai_provider: Arc<dyn AiProvider> = if provider_type == "ollama" {
                    Arc::new(OllamaProvider::new(url, model))
                } else {
                    Arc::new(OpenAICompatibleProvider::new(url, api_key, model))
                };

                let ai = Arc::new(RwLock::new(ai_provider));

                let pipeline = Arc::new(ExtractionPipeline::new(
                    sqlite.clone(),
                    qdrant.clone(),
                    ai.clone(),
                ));

                let outlook = match OutlookClient::new() {
                    Ok(o) => Arc::new(o),
                    Err(e) => {
                        error!("Failed to initialize Outlook client: {}", e);
                        return;
                    }
                };

                app_handle.manage(AppState {
                    sqlite,
                    qdrant,
                    ai,
                    pipeline,
                    outlook,
                    app_handle: app_handle.clone(),
                });
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            search_emails,
            get_stats,
            get_graph,
            start_sync,
            get_email,
            list_prompts,
            save_prompt,
            draft_reply,
            get_logs,
            get_config,
            save_config,
            save_log_cmd,
            get_models,
            force_exit,
            request_exit
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
