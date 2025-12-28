-- Initial migration for Noodle

-- Emails table
CREATE TABLE IF NOT EXISTS emails (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    store_id TEXT NOT NULL,
    entry_id TEXT NOT NULL,
    conversation_id TEXT,
    folder TEXT NOT NULL,
    subject TEXT NOT NULL,
    sender TEXT NOT NULL,
    "to" TEXT NOT NULL,
    cc TEXT,
    bcc TEXT,
    sent_at DATETIME NOT NULL,
    received_at DATETIME NOT NULL,
    body_text TEXT NOT NULL,
    body_html TEXT,
    importance INTEGER NOT NULL DEFAULT 1,
    categories TEXT,
    flags INTEGER,
    internet_message_id TEXT,
    last_indexed_at DATETIME NOT NULL,
    hash TEXT NOT NULL,
    excluded_reason TEXT,
    UNIQUE(store_id, entry_id)
);

-- Attachments table
CREATE TABLE IF NOT EXISTS attachments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email_id INTEGER NOT NULL,
    filename TEXT NOT NULL,
    mime TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    extracted_text TEXT,
    hash TEXT NOT NULL,
    FOREIGN KEY(email_id) REFERENCES emails(id) ON DELETE CASCADE
);

-- Extracted Email Facts table
CREATE TABLE IF NOT EXISTS extracted_email_facts (
    email_id INTEGER PRIMARY KEY,
    email_type TEXT NOT NULL,
    project TEXT NOT NULL, -- JSON {name, confidence}
    sentiment TEXT NOT NULL,
    urgency TEXT NOT NULL,
    summary TEXT NOT NULL,
    key_points_json TEXT NOT NULL,
    action_items_json TEXT NOT NULL,
    decisions_json TEXT NOT NULL,
    risks_json TEXT NOT NULL,
    deadlines_json TEXT NOT NULL,
    needs_response BOOLEAN NOT NULL,
    suggested_labels_json TEXT NOT NULL,
    confidence REAL NOT NULL,
    provenance_json TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    FOREIGN KEY(email_id) REFERENCES emails(id) ON DELETE CASCADE
);

-- Entities table
CREATE TABLE IF NOT EXISTS entities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,
    canonical_name TEXT NOT NULL,
    normalized_key TEXT NOT NULL UNIQUE,
    created_at DATETIME NOT NULL
);

-- Entity Mentions table
CREATE TABLE IF NOT EXISTS entity_mentions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email_id INTEGER NOT NULL,
    entity_id INTEGER NOT NULL,
    role TEXT NOT NULL,
    confidence REAL NOT NULL,
    FOREIGN KEY(email_id) REFERENCES emails(id) ON DELETE CASCADE,
    FOREIGN KEY(entity_id) REFERENCES entities(id) ON DELETE CASCADE
);

-- Edges table
CREATE TABLE IF NOT EXISTS edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    src_entity_id INTEGER NOT NULL,
    dst_entity_id INTEGER NOT NULL,
    edge_type TEXT NOT NULL,
    email_id INTEGER NOT NULL,
    created_at DATETIME NOT NULL,
    FOREIGN KEY(src_entity_id) REFERENCES entities(id) ON DELETE CASCADE,
    FOREIGN KEY(dst_entity_id) REFERENCES entities(id) ON DELETE CASCADE,
    FOREIGN KEY(email_id) REFERENCES emails(id) ON DELETE CASCADE
);

-- Prompts table
CREATE TABLE IF NOT EXISTS prompts (
    id TEXT PRIMARY KEY, -- UUID
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    schedule_cron TEXT,
    scope_json TEXT NOT NULL,
    model_pref_json TEXT NOT NULL,
    prompt_template TEXT NOT NULL,
    json_schema TEXT,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

-- Periodic Runs table
CREATE TABLE IF NOT EXISTS periodic_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    prompt_id TEXT NOT NULL,
    run_at DATETIME NOT NULL,
    status TEXT NOT NULL,
    output_json TEXT,
    output_text TEXT,
    error_text TEXT,
    FOREIGN KEY(prompt_id) REFERENCES prompts(id) ON DELETE CASCADE
);

-- FTS5 Virtual Table for Search
CREATE VIRTUAL TABLE IF NOT EXISTS emails_fts USING fts5(
    subject,
    body_text,
    summary,
    entities,
    project,
    content='emails',
    content_rowid='id'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS emails_ai AFTER INSERT ON emails BEGIN
  INSERT INTO emails_fts(rowid, subject, body_text) VALUES (new.id, new.subject, new.body_text);
END;

CREATE TRIGGER IF NOT EXISTS emails_ad AFTER DELETE ON emails BEGIN
  INSERT INTO emails_fts(emails_fts, rowid, subject, body_text) VALUES('delete', old.id, old.subject, old.body_text);
END;

CREATE TRIGGER IF NOT EXISTS emails_au AFTER UPDATE ON emails BEGIN
  INSERT INTO emails_fts(emails_fts, rowid, subject, body_text) VALUES('delete', old.id, old.subject, old.body_text);
  INSERT INTO emails_fts(rowid, subject, body_text) VALUES (new.id, new.subject, new.body_text);
END;
