//! macOS-specific Outlook client implementation using AppleScript.
//!
//! This module provides email fetching capabilities for Outlook on Mac by executing
//! AppleScript commands via the `osascript` command-line tool. It extracts emails
//! in tab-separated format for reliable parsing.
//!
//! # How It Works
//! 1. Builds an AppleScript that queries Outlook for Mac
//! 2. Iterates through all mail folders to find the one with matching name and most messages
//! 3. Filters emails by received date
//! 4. Extracts: id, subject, sender, to, cc, body, received time
//! 5. Returns data as tab-separated values for parsing in Rust
//!
//! # Folder Handling
//! Outlook for Mac often has duplicate folder names (e.g., a top-level empty "Inbox"
//! and an account-specific "Inbox" with actual emails). This implementation finds
//! the folder with the most messages to ensure we get the right one.

use chrono::{DateTime, Utc};
use noodle_core::error::{NoodleError, Result};
use noodle_core::types::Email;
use std::process::Command;
use tracing::{error, info, warn};

/// macOS Outlook client that uses AppleScript for email access.
///
/// This client interacts with Microsoft Outlook for Mac through AppleScript,
/// executed via the `osascript` command. It provides an async interface
/// compatible with the Windows implementation.
#[derive(Clone)]
pub struct MacOutlookClient;

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

        // AppleScript to fetch emails from Outlook for Mac
        // We iterate through ALL mail folders to find the one with the matching name AND messages
        // This handles the case where there are duplicate folder names (e.g., top-level empty "Inbox"
        // vs account-level "Inbox" with actual emails)
        let script = format!(
            r#"set emailList to ""
set cutoffDate to (current date) - ({days} * days)
set lf to ASCII character 10
set tb to ASCII character 9
set targetFolderName to "{folder_name}"
set theFolder to missing value

tell application "Microsoft Outlook"
    -- Find the folder with matching name that has the most messages
    set allFolders to every mail folder
    set maxCount to 0
    repeat with f in allFolders
        if name of f is targetFolderName then
            set msgCount to count of messages of f
            if msgCount > maxCount then
                set maxCount to msgCount
                set theFolder to f
            end if
        end if
    end repeat
    
    if theFolder is missing value then
        return ""
    end if
    
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
                        set toRecips to toRecips & (address of (email address of r))
                    on error
                        try
                            set toRecips to toRecips & (name of r)
                        end try
                    end try
                end repeat
            end try
            
            set ccRecips to ""
            try
                set ccList to cc recipients of msg
                repeat with r in ccList
                    if ccRecips is not "" then set ccRecips to ccRecips & ", "
                    try
                        set ccRecips to ccRecips & (address of (email address of r))
                    on error
                        try
                            set ccRecips to ccRecips & (name of r)
                        end try
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
            set theMonth to (month of recvTime as integer)
            if theMonth < 10 then set recvTimeStr to recvTimeStr & "0"
            set recvTimeStr to recvTimeStr & (theMonth as string) & "-"
            set theDay to (day of recvTime as integer)
            if theDay < 10 then set recvTimeStr to recvTimeStr & "0"
            set recvTimeStr to recvTimeStr & (theDay as string) & "T"
            set theHours to (hours of recvTime as integer)
            if theHours < 10 then set recvTimeStr to recvTimeStr & "0"
            set recvTimeStr to recvTimeStr & (theHours as string) & ":"
            set theMins to (minutes of recvTime as integer)
            if theMins < 10 then set recvTimeStr to recvTimeStr & "0"
            set recvTimeStr to recvTimeStr & (theMins as string) & ":00Z"
            
            set emailLine to msgId & tb & msgSubject & tb & senderAddr & tb & toRecips & tb & ccRecips & tb & msgBody & tb & recvTimeStr
            
            if emailList is not "" then set emailList to emailList & lf
            set emailList to emailList & emailLine
        on error errMsg
            log errMsg
        end try
    end repeat
end tell

return emailList"#,
            days = days,
            folder_name = folder_name
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

        let tsv_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        info!(
            "Received {} bytes of email data from Outlook",
            tsv_str.len()
        );

        if tsv_str.is_empty() {
            warn!("No emails found in {} (or folder not found)", folder_name);
            return Ok(vec![]);
        }

        // Parse tab-separated values
        let emails: Vec<Email> = tsv_str
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 7 {
                    let received_at = DateTime::parse_from_rfc3339(parts[6])
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());

                    Some(Email {
                        id: 0,
                        store_id: "outlook-mac".into(),
                        entry_id: parts[0].to_string(),
                        conversation_id: None,
                        folder: folder_name.to_string(),
                        subject: parts[1].to_string(),
                        sender: parts[2].to_string(),
                        to: parts[3].to_string(),
                        cc: if parts[4].is_empty() {
                            None
                        } else {
                            Some(parts[4].to_string())
                        },
                        bcc: None,
                        sent_at: received_at,
                        received_at,
                        body_text: parts[5].to_string(),
                        body_html: None,
                        importance: 1,
                        categories: None,
                        flags: None,
                        internet_message_id: None,
                        last_indexed_at: Utc::now(),
                        hash: "".into(),
                        excluded_reason: None,
                    })
                } else {
                    None
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
