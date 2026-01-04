// macOS-specific Outlook client using AppleScript

use chrono::{DateTime, Duration, Utc};
use noodle_core::error::{NoodleError, Result};
use noodle_core::types::Email;
use serde::Deserialize;
use std::process::Command;
use tracing::{error, info};

#[derive(Clone)]
pub struct MacOutlookClient;

#[derive(Debug, Deserialize)]
struct AppleScriptEmail {
    id: String,
    subject: String,
    sender: String,
    to_recipients: String,
    cc_recipients: Option<String>,
    body: String,
    received_time: String,
}

impl MacOutlookClient {
    pub fn new() -> Result<Self> {
        // Verify Outlook is installed
        let check = Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to (name of processes) contains \"Microsoft Outlook\"")
            .output();

        match check {
            Ok(output) => {
                info!(
                    "Outlook check: {:?}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            Err(e) => {
                info!("Could not check Outlook status (this is OK): {}", e);
            }
        }

        Ok(Self)
    }

    pub async fn get_emails_last_n_days(
        &self,
        days: i64,
        _folder_id: i32,
        folder_name: &str,
    ) -> Result<Vec<Email>> {
        info!("Starting macOS Outlook sync for folder: {}", folder_name);

        let folder_script_name = match folder_name {
            "Inbox" => "inbox",
            "Sent Items" => "sent mail",
            _ => "inbox",
        };

        // AppleScript to fetch emails from Outlook for Mac
        let script = format!(
            r#"
set emailList to ""
set cutoffDate to (current date) - ({} * days)

tell application "Microsoft Outlook"
    set theFolder to {}
    set theMessages to messages of theFolder whose time received > cutoffDate
    
    repeat with msg in theMessages
        try
            set msgId to id of msg as string
            set msgSubject to subject of msg
            set msgSender to sender of msg
            set senderAddr to ""
            try
                set senderAddr to address of msgSender
            on error
                try
                    set senderAddr to name of msgSender
                end try
            end try
            
            set toRecips to ""
            try
                set toList to to recipients of msg
                repeat with r in toList
                    if toRecips is not "" then set toRecips to toRecips & ", "
                    try
                        set toRecips to toRecips & (address of r)
                    on error
                        set toRecips to toRecips & (name of r)
                    end try
                end repeat
            end try
            
            set ccRecips to ""
            try
                set ccList to cc recipients of msg
                repeat with r in ccList
                    if ccRecips is not "" then set ccRecips to ccRecips & ", "
                    try
                        set ccRecips to ccRecips & (address of r)
                    on error
                        set ccRecips to ccRecips & (name of r)
                    end try
                end repeat
            end try
            
            set msgBody to ""
            try
                set msgBody to plain text content of msg
            on error
                try
                    set msgBody to content of msg
                end try
            end try
            
            set recvTime to time received of msg
            set recvTimeStr to (year of recvTime as string) & "-"
            set m to (month of recvTime as integer)
            if m < 10 then set recvTimeStr to recvTimeStr & "0"
            set recvTimeStr to recvTimeStr & (m as string) & "-"
            set d to (day of recvTime as integer)
            if d < 10 then set recvTimeStr to recvTimeStr & "0"
            set recvTimeStr to recvTimeStr & (d as string) & "T"
            set h to (hours of recvTime as integer)
            if h < 10 then set recvTimeStr to recvTimeStr & "0"
            set recvTimeStr to recvTimeStr & (h as string) & ":"
            set mins to (minutes of recvTime as integer)
            if mins < 10 then set recvTimeStr to recvTimeStr & "0"
            set recvTimeStr to recvTimeStr & (mins as string) & ":00Z"
            
            -- Escape special characters for JSON
            set msgSubject to my escapeForJson(msgSubject)
            set senderAddr to my escapeForJson(senderAddr)
            set toRecips to my escapeForJson(toRecips)
            set ccRecips to my escapeForJson(ccRecips)
            set msgBody to my escapeForJson(msgBody)
            
            set emailJson to "{{"
            set emailJson to emailJson & "\"id\":\"" & msgId & "\","
            set emailJson to emailJson & "\"subject\":\"" & msgSubject & "\","
            set emailJson to emailJson & "\"sender\":\"" & senderAddr & "\","
            set emailJson to emailJson & "\"to_recipients\":\"" & toRecips & "\","
            set emailJson to emailJson & "\"cc_recipients\":\"" & ccRecips & "\","
            set emailJson to emailJson & "\"body\":\"" & msgBody & "\","
            set emailJson to emailJson & "\"received_time\":\"" & recvTimeStr & "\""
            set emailJson to emailJson & "}}"
            
            if emailList is not "" then set emailList to emailList & ","
            set emailList to emailList & emailJson
        on error errMsg
            -- Skip problematic emails
        end try
    end repeat
end tell

return "[" & emailList & "]"

on escapeForJson(theText)
    set escaped to ""
    repeat with c in theText
        set c to c as string
        if c is "\"" then
            set escaped to escaped & "\\\""
        else if c is "\\" then
            set escaped to escaped & "\\\\"
        else if c is (ASCII character 10) then
            set escaped to escaped & "\\n"
        else if c is (ASCII character 13) then
            set escaped to escaped & "\\n"
        else if c is (ASCII character 9) then
            set escaped to escaped & "\\t"
        else
            set escaped to escaped & c
        end if
    end repeat
    return escaped
end escapeForJson
"#,
            days, folder_script_name
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| NoodleError::Outlook(format!("Failed to run osascript: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("AppleScript error: {}", stderr);
            return Err(NoodleError::Outlook(format!(
                "AppleScript failed: {}",
                stderr
            )));
        }

        let json_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        info!(
            "Received {} bytes of email data from Outlook",
            json_str.len()
        );

        if json_str.is_empty() || json_str == "[]" {
            info!("No emails found in {}", folder_name);
            return Ok(vec![]);
        }

        let parsed: Vec<AppleScriptEmail> = serde_json::from_str(&json_str).map_err(|e| {
            error!(
                "Failed to parse AppleScript output: {} - Data: {}",
                e,
                &json_str[..json_str.len().min(500)]
            );
            NoodleError::Outlook(format!("Failed to parse email data: {}", e))
        })?;

        let emails: Vec<Email> = parsed
            .into_iter()
            .map(|ae| {
                let received_at = DateTime::parse_from_rfc3339(&ae.received_time)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Email {
                    id: 0,
                    store_id: "outlook-mac".into(),
                    entry_id: ae.id,
                    conversation_id: None,
                    folder: folder_name.to_string(),
                    subject: ae.subject,
                    sender: ae.sender,
                    to: ae.to_recipients,
                    cc: ae.cc_recipients.filter(|s| !s.is_empty()),
                    bcc: None,
                    sent_at: received_at,
                    received_at,
                    body_text: ae.body,
                    body_html: None,
                    importance: 1,
                    categories: None,
                    flags: None,
                    internet_message_id: None,
                    last_indexed_at: Utc::now(),
                    hash: "".into(),
                    excluded_reason: None,
                }
            })
            .collect();

        info!(
            "Outlook search in {} returned {} emails",
            folder_name,
            emails.len()
        );

        Ok(emails)
    }
}
