use chrono::{DateTime, Utc};
use noodle_core::error::Result;
use serde_json;
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use std::path::Path;
use tracing::info;

#[derive(sqlx::FromRow)]
pub struct EmailRow {
    pub id: i64,
    pub subject: String,
    pub sender: String,
    pub received_at: DateTime<Utc>,
    pub body_text: String,
}

pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| {
                noodle_core::error::NoodleError::Storage("Invalid database path".to_string())
            })?
            .to_string();

        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
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
        let primary_type = facts.primary_type.to_string();
        let intent = facts.intent.to_string();
        let sentiment = facts.sentiment.to_string();
        let urgency = facts.urgency.to_string();
        let waiting_on = facts.waiting_on.to_string();

        let client_project = serde_json::to_string(&facts.client_or_project).unwrap();
        let key_points = serde_json::to_string(&facts.key_points).unwrap();
        let risks = serde_json::to_string(&facts.risks).unwrap();
        let issues = serde_json::to_string(&facts.issues).unwrap();
        let blockers = serde_json::to_string(&facts.blockers).unwrap();
        let open_questions = serde_json::to_string(&facts.open_questions).unwrap();
        let answered_questions = serde_json::to_string(&facts.answered_questions).unwrap();

        let provenance = serde_json::to_string(&facts.provenance).unwrap();

        // Previous schema had 'deadlines_json', 'action_items_json', 'decisions_json', 'suggested_labels_json'.
        // These are removed or re-mapped. We do NOT insert them.

        sqlx::query(
            r#"
            INSERT INTO extracted_email_facts (
                email_id, primary_type, intent, urgency, sentiment, client_or_project_json,
                due_by, needs_response, waiting_on, summary, key_points_json,
                risks_json, issues_json, blockers_json, open_questions_json, answered_questions_json,
                confidence, provenance_json, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(email_id) DO UPDATE SET
                primary_type = excluded.primary_type,
                intent = excluded.intent,
                urgency = excluded.urgency,
                sentiment = excluded.sentiment,
                client_or_project_json = excluded.client_or_project_json,
                due_by = excluded.due_by,
                needs_response = excluded.needs_response,
                waiting_on = excluded.waiting_on,
                summary = excluded.summary,
                key_points_json = excluded.key_points_json,
                risks_json = excluded.risks_json,
                issues_json = excluded.issues_json,
                blockers_json = excluded.blockers_json,
                open_questions_json = excluded.open_questions_json,
                answered_questions_json = excluded.answered_questions_json,
                confidence = excluded.confidence,
                provenance_json = excluded.provenance_json
            "#,
        )
        .bind(facts.email_id)
        .bind(primary_type)
        .bind(intent)
        .bind(urgency)
        .bind(sentiment)
        .bind(client_project)
        .bind(facts.due_by)
        .bind(facts.needs_response)
        .bind(waiting_on)
        .bind(&facts.summary)
        .bind(key_points)
        .bind(risks)
        .bind(issues)
        .bind(blockers)
        .bind(open_questions)
        .bind(answered_questions)
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
                r#"
                SELECT 
                    e.id, e.subject, e.sender, e.received_at, e.body_text,
                    f.primary_type, f.intent, f.urgency, f.sentiment, f.client_or_project_json,
                    f.needs_response, f.waiting_on, f.due_by, f.risks_json, f.issues_json, f.blockers_json,
                    f.summary
                FROM emails e
                LEFT JOIN extracted_email_facts f ON e.id = f.email_id
                WHERE e.id = ?
                "#,
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| noodle_core::error::NoodleError::Storage(e.to_string()))?;

            if let Some(row) = email {
                let client_project: Option<serde_json::Value> = row
                    .get::<Option<String>, _>("client_or_project_json")
                    .and_then(|s| serde_json::from_str(&s).ok());

                let risks: Option<serde_json::Value> = row
                    .get::<Option<String>, _>("risks_json")
                    .and_then(|s| serde_json::from_str(&s).ok());

                results.push(serde_json::json!({
                    "id": row.get::<i64, _>("id"),
                    "subject": row.get::<String, _>("subject"),
                    "sender": row.get::<String, _>("sender"),
                    "received_at": row.get::<chrono::DateTime<chrono::Utc>, _>("received_at"),
                    "body_text": row.get::<String, _>("body_text"),
                    "primary_type": row.get::<Option<String>, _>("primary_type"),
                    "intent": row.get::<Option<String>, _>("intent"),
                    "urgency": row.get::<Option<String>, _>("urgency"),
                    "sentiment": row.get::<Option<String>, _>("sentiment"),
                    "needs_response": row.get::<Option<bool>, _>("needs_response"),
                    "waiting_on": row.get::<Option<String>, _>("waiting_on"),
                    "due_by": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("due_by"),
                    "summary": row.get::<Option<String>, _>("summary"),
                    "client_or_project": client_project,
                    "risks": risks
                }));
            }
        }
        Ok(results)
    }

    pub async fn get_recent_emails(&self, limit: i64) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                e.id, e.subject, e.sender, e.received_at, e.body_text,
                f.primary_type, f.intent, f.urgency, f.sentiment, f.client_or_project_json,
                f.needs_response, f.waiting_on, f.due_by, f.risks_json, f.issues_json, f.blockers_json,
                f.summary
            FROM emails e
            LEFT JOIN extracted_email_facts f ON e.id = f.email_id
            ORDER BY e.received_at DESC 
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| noodle_core::error::NoodleError::Storage(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let client_project: Option<serde_json::Value> = row
                    .get::<Option<String>, _>("client_or_project_json")
                    .and_then(|s| serde_json::from_str(&s).ok());

                let risks: Option<serde_json::Value> = row
                    .get::<Option<String>, _>("risks_json")
                    .and_then(|s| serde_json::from_str(&s).ok());

                serde_json::json!({
                    "id": row.get::<i64, _>("id"),
                    "subject": row.get::<String, _>("subject"),
                    "sender": row.get::<String, _>("sender"),
                    "received_at": row.get::<chrono::DateTime<chrono::Utc>, _>("received_at"),
                    "body_text": row.get::<String, _>("body_text"),
                    "primary_type": row.get::<Option<String>, _>("primary_type"),
                    "intent": row.get::<Option<String>, _>("intent"),
                    "urgency": row.get::<Option<String>, _>("urgency"),
                    "sentiment": row.get::<Option<String>, _>("sentiment"),
                    "needs_response": row.get::<Option<bool>, _>("needs_response"),
                    "waiting_on": row.get::<Option<String>, _>("waiting_on"),
                    "due_by": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("due_by"),
                    "summary": row.get::<Option<String>, _>("summary"),
                    "client_or_project": client_project,
                    "risks": risks
                })
            })
            .collect())
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

    pub async fn save_log(
        &self,
        level: &str,
        source: &str,
        message: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<()> {
        let metadata_str = metadata.map(|m| serde_json::to_string(&m).unwrap());
        sqlx::query("INSERT INTO logs (timestamp, level, source, message, metadata_json) VALUES (?, ?, ?, ?, ?)")
            .bind(Utc::now())
            .bind(level)
            .bind(source)
            .bind(message)
            .bind(metadata_str)
            .execute(&self.pool)
            .await
            .map_err(|e| noodle_core::error::NoodleError::Storage(e.to_string()))?;
        Ok(())
    }

    pub async fn get_logs(&self, limit: i64) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query("SELECT * FROM logs ORDER BY timestamp DESC LIMIT ?")
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| noodle_core::error::NoodleError::Storage(e.to_string()))?;

        Ok(rows.into_iter().map(|r| serde_json::json!({
            "id": r.get::<i64, _>("id"),
            "timestamp": r.get::<DateTime<Utc>, _>("timestamp"),
            "level": r.get::<String, _>("level"),
            "source": r.get::<String, _>("source"),
            "message": r.get::<String, _>("message"),
            "metadata": r.get::<Option<String>, _>("metadata_json").and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        })).collect())
    }

    pub async fn set_config(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query("INSERT INTO app_config (key, value, updated_at) VALUES (?, ?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at")
            .bind(key)
            .bind(value)
            .bind(Utc::now())
            .execute(&self.pool)
            .await
            .map_err(|e| noodle_core::error::NoodleError::Storage(e.to_string()))?;
        Ok(())
    }

    pub async fn get_config(&self, key: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT value FROM app_config WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| noodle_core::error::NoodleError::Storage(e.to_string()))?;

        Ok(row.map(|r| r.get("value")))
    }
}
