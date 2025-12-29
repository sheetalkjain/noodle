use agent::engine::SyncManager;
use agent::pipeline::ExtractionPipeline;
use ai::provider::{AiProvider, LocalProvider};
use outlook::client::OutlookClient;
use std::sync::Arc;
use storage::qdrant::QdrantStorage;
use storage::sqlite::SqliteStorage;
use tauri::{command, Emitter, Manager, State};
use tracing::{error, info};

struct AppState {
    sqlite: Arc<SqliteStorage>,
    qdrant: Arc<QdrantStorage>,
    ai: Arc<dyn AiProvider>,
    pipeline: Arc<ExtractionPipeline>,
    outlook: Arc<OutlookClient>,
    app_handle: tauri::AppHandle,
}

#[command]
async fn search_emails(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<serde_json::Value>, String> {
    // 1. Generate embedding for query
    let embedding = state
        .ai
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
        .map_err(|e| e.to_string())
}

#[command]
async fn get_config(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    state
        .sqlite
        .get_config(&key)
        .await
        .map_err(|e| e.to_string())
}

#[command]
async fn save_config(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    state
        .sqlite
        .set_config(&key, &value)
        .await
        .map_err(|e| e.to_string())
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
        .map_err(|e| e.to_string())
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
        };
        let response = state
            .ai
            .chat_completion(request)
            .await
            .map_err(|e| e.to_string())?;
        Ok(response.content)
    } else {
        Err("Email not found".into())
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();

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
                        // We still need to manage state even if broken, or app will crash on invoke
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

                let ai: Arc<dyn AiProvider> =
                    Arc::new(LocalProvider::new("http://localhost:1234/v1".into(), None));

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
            save_log_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
