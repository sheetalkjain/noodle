use crate::pipeline::ExtractionPipeline;
use noodle_core::error::Result;
use outlook::client::OutlookClient;
use std::sync::Arc;
use storage::sqlite::SqliteStorage;
use tokio::time::{interval, Duration};
use tracing::{error, info};

pub struct SyncManager {
    pipeline: Arc<ExtractionPipeline>,
    outlook: Arc<OutlookClient>,
    sqlite: Arc<SqliteStorage>,
    app_handle: tauri::AppHandle,
}

impl SyncManager {
    pub fn new(
        pipeline: Arc<ExtractionPipeline>,
        outlook: Arc<OutlookClient>,
        sqlite: Arc<SqliteStorage>,
        app_handle: tauri::AppHandle,
    ) -> Self {
        Self {
            pipeline,
            outlook,
            sqlite,
            app_handle,
        }
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
    }

    pub async fn start_background_sync(self: Arc<Self>) {
        info!("Starting background sync manager");
        self.log_to_ui("Sync manager started", "info");

        // 1. Initial Scan (Last 30 days)
        if let Err(e) = self.run_initial_scan().await {
            error!("Initial scan failed: {}", e);
        }

        // 2. Periodic Delta Scan (Every 2 minutes)
        let mut interval = interval(Duration::from_secs(120));
        loop {
            interval.tick().await;
            info!("Running periodic delta scan...");
            if let Err(e) = self.run_delta_scan().await {
                error!("Delta scan failed: {}", e);
            }
        }
    }

    async fn run_initial_scan(&self) -> Result<()> {
        info!("Running initial 90-day sync for all folders...");
        let folders = [(6, "Inbox"), (5, "Sent Items")];

        for (folder_id, folder_name) in folders {
            info!("Processing folder: {}", folder_name);
            self.log_to_ui(&format!("Fetching emails from {}...", folder_name), "info");
            let emails = match self
                .outlook
                .get_emails_last_n_days(90, folder_id, folder_name)
            {
                Ok(e) => e,
                Err(e) => {
                    error!("Failed to fetch emails from {}: {}", folder_name, e);
                    self.log_to_ui(&format!("Error fetching {}: {}", folder_name, e), "error");
                    continue;
                }
            };

            info!("Found {} emails in {}", emails.len(), folder_name);
            self.log_to_ui(
                &format!(
                    "Found {} emails in {}. Processing...",
                    emails.len(),
                    folder_name
                ),
                "info",
            );
            for email in emails {
                let subject = email.subject.clone();
                if let Err(e) = self.pipeline.process_email(email).await {
                    error!(
                        "Failed to process email '{}' from {}: {}",
                        subject, folder_name, e
                    );
                    self.log_to_ui(&format!("Skipped '{}': {}", subject, e), "warn");
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
            let emails = match self
                .outlook
                .get_emails_last_n_days(1, folder_id, folder_name)
            {
                Ok(e) => e,
                Err(e) => {
                    error!("Failed to fetch delta emails from {}: {}", folder_name, e);
                    continue;
                }
            };

            for email in emails {
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
