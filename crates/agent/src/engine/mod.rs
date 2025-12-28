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
}

impl SyncManager {
    pub fn new(
        pipeline: Arc<ExtractionPipeline>,
        outlook: Arc<OutlookClient>,
        sqlite: Arc<SqliteStorage>,
    ) -> Self {
        Self {
            pipeline,
            outlook,
            sqlite,
        }
    }

    pub async fn start_background_sync(self: Arc<Self>) {
        info!("Starting background sync manager");

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
        info!("Running initial 30-day sync...");
        let emails = self.outlook.get_emails_last_n_days(30)?;
        info!("Found {} emails in Outlook", emails.len());
        for email in emails {
            let subject = email.subject.clone();
            if let Err(e) = self.pipeline.process_email(email).await {
                error!("Failed to process email '{}': {}", subject, e);
                // Continue to next email
            }
        }
        Ok(())
    }

    async fn run_delta_scan(&self) -> Result<()> {
        // Scans last 1 day for any missed items
        let emails = self.outlook.get_emails_last_n_days(1)?;
        for email in emails {
            let subject = email.subject.clone();
            // Pipeline should handle deduplication based on StoreID + EntryID
            if let Err(e) = self.pipeline.process_email(email).await {
                error!("Failed to process email in delta scan '{}': {}", subject, e);
            }
        }
        Ok(())
    }
}
