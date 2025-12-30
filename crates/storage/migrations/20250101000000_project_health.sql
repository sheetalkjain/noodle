-- Project Health Refactor Migration
-- Dropping old facts table to enforce new schema structure
DROP TABLE IF EXISTS extracted_email_facts;

CREATE TABLE extracted_email_facts (
    email_id INTEGER PRIMARY KEY,
    primary_type TEXT NOT NULL, -- Update|Request|Decision|FYI
    intent TEXT NOT NULL, -- Inform|Ask|Escalate|Commit|Clarify|Resolve
    urgency TEXT NOT NULL, -- Low|Medium|High
    sentiment TEXT NOT NULL, -- Neutral|Positive|Concerned|Hostile
    client_or_project_json TEXT NOT NULL, -- {name, confidence}
    due_by DATETIME,
    needs_response BOOLEAN NOT NULL,
    waiting_on TEXT NOT NULL, -- me|them|third_party|none
    
    summary TEXT NOT NULL,
    key_points_json TEXT NOT NULL, -- string[]
    
    risks_json TEXT NOT NULL, -- [{title, details, owner, severity, confidence}]
    issues_json TEXT NOT NULL, -- [{title, details, owner, severity, confidence}]
    blockers_json TEXT NOT NULL, -- [{title, details, owner, severity, confidence}]
    
    open_questions_json TEXT NOT NULL, -- [{question, asked_by, owner, due_by, confidence}]
    answered_questions_json TEXT NOT NULL, -- [{question, answer_summary, confidence}]
    
    confidence REAL NOT NULL,
    provenance_json TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    FOREIGN KEY(email_id) REFERENCES emails(id) ON DELETE CASCADE
);
