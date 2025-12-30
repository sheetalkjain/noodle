use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    pub id: i64,
    pub store_id: String,
    pub entry_id: String,
    pub conversation_id: Option<String>,
    pub folder: String,
    pub subject: String,
    pub sender: String,
    pub to: String,
    pub cc: Option<String>,
    pub bcc: Option<String>,
    pub sent_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub body_text: String,
    pub body_html: Option<String>,
    pub importance: i32,
    pub categories: Option<String>,
    pub flags: Option<i32>,
    pub internet_message_id: Option<String>,
    pub last_indexed_at: DateTime<Utc>,
    pub hash: String,
    pub excluded_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: i64,
    pub email_id: i64,
    pub filename: String,
    pub mime: String,
    pub size_bytes: i64,
    pub extracted_text: Option<String>,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailFact {
    pub email_id: i64,
    pub primary_type: PrimaryType,
    pub intent: Intent,
    pub client_or_project: ProjectInfo,
    pub sentiment: Sentiment,
    pub urgency: Urgency,
    pub due_by: Option<DateTime<Utc>>,
    pub needs_response: bool,
    pub waiting_on: WaitingOn,
    pub summary: String,
    pub key_points: Vec<String>,
    pub risks: Vec<Risk>,
    pub issues: Vec<Issue>,
    pub blockers: Vec<Blocker>,
    pub open_questions: Vec<OpenQuestion>,
    pub answered_questions: Vec<AnsweredQuestion>,
    pub confidence: f32,
    pub provenance: Provenance,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, strum_macros::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PrimaryType {
    Update,
    Request,
    Decision,
    Fyi,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, strum_macros::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Intent {
    Inform,
    Ask,
    Escalate,
    Commit,
    Clarify,
    Resolve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, strum_macros::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Sentiment {
    Neutral,
    Positive,
    Concerned,
    Hostile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, strum_macros::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Urgency {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, strum_macros::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum WaitingOn {
    Me,
    Them,
    ThirdParty,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, strum_macros::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risk {
    pub title: String,
    pub details: String,
    pub owner: Option<String>,
    pub severity: Severity,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub title: String,
    pub details: String,
    pub owner: Option<String>,
    pub severity: Severity,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blocker {
    pub title: String,
    pub details: String,
    pub owner: Option<String>,
    pub severity: Severity,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenQuestion {
    pub question: String,
    pub asked_by: Option<String>,
    pub owner: Option<String>,
    pub due_by: Option<DateTime<Utc>>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnsweredQuestion {
    pub question: String,
    pub answer_summary: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub model: String,
    pub provider: String,
    pub prompt_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: i64,
    pub entity_type: String,
    pub canonical_name: String,
    pub normalized_key: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMention {
    pub id: i64,
    pub email_id: i64,
    pub entity_id: i64,
    pub role: EntityRole,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EntityRole {
    Sender,
    Recipient,
    Cc,
    Internal,
    External,
    Client,
    Vendor,
    Opposing,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: i64,
    pub src_entity_id: i64,
    pub dst_entity_id: i64,
    pub edge_type: String,
    pub email_id: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub id: Uuid,
    pub name: String,
    pub kind: PromptKind,
    pub enabled: bool,
    pub schedule_cron: Option<String>,
    pub scope: PromptScope,
    pub model_pref: ModelPreference,
    pub prompt_template: String,
    pub json_schema: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PromptKind {
    Extraction,
    Periodic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptScope {
    pub folders: Option<Vec<String>>,
    pub date_range: Option<DateRange>,
    pub project: Option<String>,
    pub needs_response: Option<bool>,
    pub sentiment: Option<Vec<Sentiment>>,
    pub participants: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreference {
    pub provider: String,
    pub model: String,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodicRun {
    pub id: i64,
    pub prompt_id: Uuid,
    pub run_at: DateTime<Utc>,
    pub status: String,
    pub output_json: Option<serde_json::Value>,
    pub output_text: Option<String>,
    pub error_text: Option<String>,
}
