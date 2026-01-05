//! Email synchronization engine.
//!
//! This module provides the `SyncManager` which orchestrates the email sync process:
//! 1. Fetches emails from Outlook (Windows COM or macOS AppleScript)
//! 2. Passes each email through the extraction pipeline
//! 3. Handles periodic delta syncs for new emails
//! 4. Supports cancellation via `CancellationToken`
//!
//! # Sync Flow
//! 1. **Initial Scan**: Fetches emails from the last N days (configurable)
//! 2. **Delta Scan**: Periodically checks for new emails (every N minutes)
//! 3. **Cancellation**: Can be stopped at any time via the cancel token

use crate::pipeline::ExtractionPipeline;
use noodle_core::error::Result;
use outlook::client::OutlookClient;
use std::sync::Arc;
use storage::sqlite::SqliteStorage;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

/// Manages email synchronization from Outlook to local storage.
///
/// The `SyncManager` coordinates between the Outlook client, extraction pipeline,
/// and storage layer to keep emails synchronized and processed.
///
/// # Features
/// - Configurable history window (how many days to sync)
/// - Configurable sync interval for delta updates
/// - Cancellable sync operations
/// - Progress logging to UI via Tauri events
///
/// # Example
/// ```ignore
/// let manager = SyncManager::new(pipeline, outlook, sqlite, app_handle, 30, 5);
/// let manager = Arc::new(manager);
/// manager.start_background_sync().await;
/// ```
pub struct SyncManager {
    /// Pipeline for processing emails (entity extraction, AI facts, embeddings)
    pipeline: Arc<ExtractionPipeline>,
    /// Outlook client for fetching emails
    outlook: Arc<OutlookClient>,
    /// SQLite storage for persisting emails and logs
    #[allow(dead_code)]
    sqlite: Arc<SqliteStorage>,
    /// Tauri app handle for emitting events to the UI
    app_handle: tauri::AppHandle,
    /// Number of days of email history to sync
    history_days: i64,
    /// Minutes between delta syncs
    sync_interval_mins: i64,
    /// Token to cancel sync operations
    cancel_token: CancellationToken,
}

impl SyncManager {
    pub fn new(
        pipeline: Arc<ExtractionPipeline>,
        outlook: Arc<OutlookClient>,
        sqlite: Arc<SqliteStorage>,
        app_handle: tauri::AppHandle,
        history_days: i64,
        sync_interval_mins: i64,
    ) -> Self {
        Self {
            pipeline,
            outlook,
            sqlite,
            app_handle,
            history_days,
            sync_interval_mins,
            cancel_token: CancellationToken::new(),
        }
    }

    /// Get a clone of the cancellation token for external use
    pub fn get_cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    /// Cancel the sync operation
    pub fn cancel(&self) {
        info!("Sync cancellation requested");
        self.log_to_ui("Sync cancelled by user", "warn");
        self.cancel_token.cancel();
    }

    fn log_to_ui(&self, message: &str, level: &str) {
        use tauri::Emitter;
        let _ = self.app_handle.emit(
            "noodle://log",
            serde_json::json!({
                "message": message,
                "level": level
            }),
        );

        // Also persist to DB
        let sqlite = self.sqlite.clone();
        let msg = message.to_string();
        let lvl = level.to_string();
        tokio::spawn(async move {
            let _ = sqlite.save_log(&lvl, "BACKEND", &msg, None).await;
        });
    }

    fn emit_sync_status(&self, status: &str) {
        use tauri::Emitter;
        let _ = self.app_handle.emit(
            "noodle://sync_status",
            serde_json::json!({ "status": status }),
        );
    }

    pub async fn start_background_sync(self: Arc<Self>) {
        info!("Starting background sync manager");
        self.log_to_ui("Sync manager started", "info");
        self.emit_sync_status("running");

        // 1. Initial Scan (Last N days)
        if let Err(e) = self.run_initial_scan().await {
            if self.cancel_token.is_cancelled() {
                info!("Initial scan cancelled");
                self.emit_sync_status("cancelled");
                return;
            }
            error!("Initial scan failed: {}", e);
        }

        if self.cancel_token.is_cancelled() {
            info!("Sync cancelled after initial scan");
            self.emit_sync_status("cancelled");
            return;
        }

        // 2. Periodic Delta Scan
        let mut interval = interval(Duration::from_secs(self.sync_interval_mins as u64 * 60));
        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    info!("Periodic sync cancelled");
                    self.log_to_ui("Sync stopped", "info");
                    self.emit_sync_status("cancelled");
                    break;
                }
                _ = interval.tick() => {
                    info!("Running periodic delta scan...");
                    if let Err(e) = self.run_delta_scan().await {
                        if self.cancel_token.is_cancelled() {
                            break;
                        }
                        error!("Delta scan failed: {}", e);
                    }
                }
            }
        }
    }

    async fn run_initial_scan(&self) -> Result<()> {
        info!(
            "Running initial {}-day sync for all folders...",
            self.history_days
        );
        self.log_to_ui(
            &format!("Starting {} day email sync...", self.history_days),
            "info",
        );

        let folders = [(6, "Inbox"), (5, "Sent Items")];

        for (folder_id, folder_name) in folders {
            if self.cancel_token.is_cancelled() {
                warn!("Sync cancelled during folder iteration");
                return Ok(());
            }

            info!("Processing folder: {}", folder_name);
            self.log_to_ui(&format!("Fetching emails from {}...", folder_name), "info");
            let emails = match self
                .outlook
                .get_emails_last_n_days(self.history_days, folder_id, folder_name)
                .await
            {
                Ok(e) => e,
                Err(e) => {
                    error!("Failed to fetch emails from {}: {}", folder_name, e);
                    self.log_to_ui(&format!("Error fetching {}: {}", folder_name, e), "error");
                    continue;
                }
            };

            let total = emails.len();
            info!("Found {} emails in {}", total, folder_name);
            self.log_to_ui(
                &format!("Found {} emails in {}. Processing...", total, folder_name),
                "info",
            );

            for (idx, email) in emails.into_iter().enumerate() {
                if self.cancel_token.is_cancelled() {
                    warn!("Sync cancelled during email processing");
                    self.log_to_ui("Sync cancelled", "warn");
                    return Ok(());
                }

                let subject = email.subject.clone();
                if let Err(e) = self.pipeline.process_email(email).await {
                    error!(
                        "Failed to process email '{}' from {}: {}",
                        subject, folder_name, e
                    );
                    self.log_to_ui(&format!("Skipped '{}': {}", subject, e), "warn");
                }

                // Log progress every 10 emails
                if (idx + 1) % 10 == 0 || idx + 1 == total {
                    self.log_to_ui(
                        &format!(
                            "Processed {}/{} emails from {}",
                            idx + 1,
                            total,
                            folder_name
                        ),
                        "info",
                    );
                }
            }
        }

        info!("Initial sync completed");
        self.log_to_ui("Initial sync cycle completed", "info");
        Ok(())
    }

    async fn run_delta_scan(&self) -> Result<()> {
        info!("Running periodic delta scan for all folders...");
        let folders = [(6, "Inbox"), (5, "Sent Items")];

        for (folder_id, folder_name) in folders {
            if self.cancel_token.is_cancelled() {
                return Ok(());
            }

            let emails = match self
                .outlook
                .get_emails_last_n_days(1, folder_id, folder_name)
                .await
            {
                Ok(e) => e,
                Err(e) => {
                    error!("Failed to fetch delta emails from {}: {}", folder_name, e);
                    continue;
                }
            };

            for email in emails {
                if self.cancel_token.is_cancelled() {
                    return Ok(());
                }

                let subject = email.subject.clone();
                if let Err(e) = self.pipeline.process_email(email).await {
                    error!(
                        "Failed to process email in delta scan '{}' from {}: {}",
                        subject, folder_name, e
                    );
                }
            }
        }
        Ok(())
    }
}
