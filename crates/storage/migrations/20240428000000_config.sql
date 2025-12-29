-- Add app_config and logs tables

CREATE TABLE IF NOT EXISTS app_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp DATETIME NOT NULL,
    level TEXT NOT NULL, -- INFO, WARN, ERROR, DEBUG
    source TEXT NOT NULL, -- BACKEND, FRONTEND, OUTLOOK, etc.
    message TEXT NOT NULL,
    metadata_json TEXT -- Flexible JSON for extra details
);

-- Index for faster log retrieval
CREATE INDEX IF NOT EXISTS idx_logs_timestamp ON logs(timestamp);
