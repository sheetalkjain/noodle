use noodle_core::error::Result;
use serde_json;
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use std::path::Path;
use tracing::info;

pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_str().ok_or_else(|| {
            noodle_core::error::NoodleError::Storage("Invalid database path".to_string())
        })?;

        let connection_str = format!("sqlite://{}", path_str);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&connection_str)
            .await
            .map_err(|e| noodle_core::error::NoodleError::Storage(e.to_string()))?;

        info!("Connected to SQLite at {}", path_str);

        let storage = Self { pool };
        storage.migrate().await?;

        Ok(storage)
    }

    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| noodle_core::error::NoodleError::Storage(e.to_string()))?;

        info!("SQLite migrations completed");
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn save_email(&self, email: &noodle_core::types::Email) -> Result<i64> {
        let importance = email.importance as i64;
        let flags = email.flags.map(|f| f as i64);

        let row = sqlx::query(
            r#"
            INSERT INTO emails (
                store_id, entry_id, conversation_id, folder, subject, sender, "to", cc, bcc, 
                sent_at, received_at, body_text, body_html, importance, categories, flags, 
                internet_message_id, last_indexed_at, hash
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(store_id, entry_id) DO UPDATE SET
                folder = excluded.folder,
                subject = excluded.subject,
                received_at = excluded.received_at,
                body_text = excluded.body_text,
                last_indexed_at = excluded.last_indexed_at,
                hash = excluded.hash
            RETURNING id
            "#,
        )
        .bind(&email.store_id)
        .bind(&email.entry_id)
        .bind(email.conversation_id.as_ref())
        .bind(&email.folder)
        .bind(&email.subject)
        .bind(&email.sender)
        .bind(&email.to)
        .bind(email.cc.as_ref())
        .bind(email.bcc.as_ref())
        .bind(email.sent_at)
        .bind(email.received_at)
        .bind(&email.body_text)
        .bind(email.body_html.as_ref())
        .bind(importance)
        .bind(email.categories.as_ref())
        .bind(flags)
        .bind(email.internet_message_id.as_ref())
        .bind(email.last_indexed_at)
        .bind(&email.hash)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| noodle_core::error::NoodleError::Storage(e.to_string()))?;

        Ok(row.get("id"))
    }

    pub async fn save_facts(&self, facts: &noodle_core::types::EmailFact) -> Result<()> {
        let email_type = facts.email_type.to_string();
        let sentiment = facts.sentiment.to_string();
        let urgency = facts.urgency.to_string();

        let project_json = serde_json::to_string(&facts.project).unwrap();
        let key_points = serde_json::to_string(&facts.key_points).unwrap();
        let action_items = serde_json::to_string(&facts.action_items).unwrap();
        let decisions = serde_json::to_string(&facts.decisions).unwrap();
        let risks = serde_json::to_string(&facts.risks).unwrap();
        let deadlines = serde_json::to_string(&facts.deadlines).unwrap();
        let labels = serde_json::to_string(&facts.suggested_labels).unwrap();
        let provenance = serde_json::to_string(&facts.provenance).unwrap();

        sqlx::query(
            r#"
            INSERT INTO extracted_email_facts (
                email_id, email_type, project, sentiment, urgency, summary,
                key_points_json, action_items_json, decisions_json, risks_json,
                deadlines_json, needs_response, suggested_labels_json,
                confidence, provenance_json, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(email_id) DO UPDATE SET
                sentiment = excluded.sentiment,
                urgency = excluded.urgency,
                summary = excluded.summary,
                confidence = excluded.confidence
            "#,
        )
        .bind(facts.email_id)
        .bind(email_type)
        .bind(project_json)
        .bind(sentiment)
        .bind(urgency)
        .bind(&facts.summary)
        .bind(key_points)
        .bind(action_items)
        .bind(decisions)
        .bind(risks)
        .bind(deadlines)
        .bind(facts.needs_response)
        .bind(labels)
        .bind(facts.confidence)
        .bind(provenance)
        .bind(facts.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| noodle_core::error::NoodleError::Storage(e.to_string()))?;

        Ok(())
    }

    pub async fn get_dashboard_stats(&self) -> Result<serde_json::Value> {
        let total_emails = sqlx::query("SELECT COUNT(*) as count FROM emails")
            .fetch_one(&self.pool)
            .await
            .map(|r| r.get::<i64, _>("count"))
            .unwrap_or(0);

        let sentiment_data = sqlx::query(
            "SELECT sentiment, COUNT(*) as count FROM extracted_email_facts GROUP BY sentiment",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_else(|_| vec![]);

        let sentiments = sentiment_data
            .into_iter()
            .map(|r| serde_json::json!({ "sentiment": r.get::<String, _>("sentiment"), "count": r.get::<i64, _>("count") }))
            .collect::<Vec<_>>();

        Ok(serde_json::json!({
            "total_emails": total_emails,
            "sentiments": sentiments
        }))
    }
    pub async fn get_emails_by_ids(&self, ids: Vec<i64>) -> Result<Vec<serde_json::Value>> {
        let mut results = Vec::new();
        for id in ids {
            let email = sqlx::query(
                "SELECT id, subject, sender, received_at, body_text FROM emails WHERE id = ?",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| noodle_core::error::NoodleError::Storage(e.to_string()))?;

            if let Some(row) = email {
                results.push(serde_json::json!({
                    "id": row.get::<i64, _>("id"),
                    "subject": row.get::<String, _>("subject"),
                    "sender": row.get::<String, _>("sender"),
                    "received_at": row.get::<chrono::DateTime<chrono::Utc>, _>("received_at"),
                    "body_text": row.get::<String, _>("body_text")
                }));
            }
        }
        Ok(results)
    }

    pub async fn get_entities(&self) -> Result<serde_json::Value> {
        let nodes_rows = sqlx::query(
            "SELECT id, canonical_name as name, entity_type as kind FROM entities LIMIT 100",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_else(|_| vec![]);

        let links_rows = sqlx::query("SELECT src_entity_id as source, dst_entity_id as target, edge_type as kind FROM edges LIMIT 200")
            .fetch_all(&self.pool)
            .await
            .unwrap_or_else(|_| vec![]);

        Ok(serde_json::json!({
            "nodes": nodes_rows.into_iter().map(|n| serde_json::json!({ "id": n.get::<i64, _>("id").to_string(), "name": n.get::<String, _>("name"), "type": n.get::<String, _>("kind") })).collect::<Vec<_>>(),
            "links": links_rows.into_iter().map(|l| serde_json::json!({ "source": l.get::<i64, _>("source").to_string(), "target": l.get::<i64, _>("target").to_string(), "type": l.get::<String, _>("kind") })).collect::<Vec<_>>()
        }))
    }
}
