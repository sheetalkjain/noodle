use core::error::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;
use tracing::info;

pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_str().ok_or_else(|| {
            core::error::NoodleError::Storage("Invalid database path".to_string())
        })?;
        
        let connection_str = format!("sqlite://{}", path_str);
        
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&connection_str)
            .await
            .map_err(|e| core::error::NoodleError::Storage(e.to_string()))?;
            
        info!("Connected to SQLite at {}", path_str);
        
        let storage = Self { pool };
        storage.migrate().await?;
        
        Ok(storage)
    }

    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| core::error::NoodleError::Storage(e.to_string()))?;
            
        info!("SQLite migrations completed");
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
