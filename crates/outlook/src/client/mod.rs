use crate::com::ComDispatch;
use chrono::{DateTime, Duration, Utc};
use noodle_core::error::{NoodleError, Result};
use noodle_core::types::Email;
use windows::core::{BSTR, VARIANT};
use windows::Win32::System::Com::IDispatch;

pub struct OutlookClient {
    namespace: ComDispatch,
}

impl OutlookClient {
    pub fn new() -> Result<Self> {
        let app_clsid = windows::core::GUID::from("0006F03A-0000-0000-C000-000000000046");
        unsafe {
            let app: IDispatch = windows::Win32::System::Com::CoCreateInstance(
                &app_clsid,
                None,
                windows::Win32::System::Com::CLSCTX_LOCAL_SERVER,
            )
            .map_err(|e| NoodleError::Outlook(format!("Failed to start Outlook: {}", e)))?;

            let app_dispatch = ComDispatch(app);
            let namespace_var =
                app_dispatch.call_method("GetNamespace", &mut [VARIANT::from("MAPI")])?;
            let namespace: IDispatch = IDispatch::try_from(&namespace_var).map_err(|e| {
                NoodleError::Outlook(format!("Failed to get Namespace dispatch: {}", e))
            })?;

            Ok(Self {
                namespace: ComDispatch(namespace),
            })
        }
    }

    pub fn get_emails_last_n_days(
        &self,
        days: i64,
        folder_id: i32,
        folder_name: &str,
    ) -> Result<Vec<Email>> {
        tracing::info!(
            "Starting Outlook sync for folder: {} (ID: {})",
            folder_name,
            folder_id
        );

        let folder_var = self
            .namespace
            .call_method("GetDefaultFolder", &mut [VARIANT::from(folder_id)])?;

        let folder = ComDispatch(IDispatch::try_from(&folder_var).map_err(|e| {
            NoodleError::Outlook(format!("Failed to get folder {}: {}", folder_name, e))
        })?);

        let items_var = folder.get_property("Items")?;
        let items = ComDispatch(IDispatch::try_from(&items_var).map_err(|e| {
            NoodleError::Outlook(format!("Failed to get Items for {}: {}", folder_name, e))
        })?);

        // Filter items
        let filter_date = Utc::now() - Duration::days(days);
        // Using a more standard format for Outlook filters (English month is most robust)
        let filter = format!(
            "[ReceivedTime] >= '{}'",
            filter_date.format("%d %b %Y %H:%M %p")
        );

        tracing::info!("Applying Outlook filter for {}: {}", folder_name, filter);

        // We catch errors here and log them specifically
        let filtered_items_var =
            match items.call_method("Restrict", &mut [VARIANT::from(filter.as_str())]) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("Outlook Restrict method failed for {}: {}", folder_name, e);
                    return Err(e);
                }
            };

        let filtered_items =
            ComDispatch(IDispatch::try_from(&filtered_items_var).map_err(|e| {
                NoodleError::Outlook(format!(
                    "Failed to restrict items in {}: {}",
                    folder_name, e
                ))
            })?);

        let emails = self.parse_items(filtered_items, folder_name)?;
        tracing::info!(
            "Outlook search in {} returned {} emails",
            folder_name,
            emails.len()
        );
        Ok(emails)
    }

    fn parse_items(&self, items: ComDispatch, folder_name: &str) -> Result<Vec<Email>> {
        let count_var = items.get_property("Count")?;
        let count = i32::try_from(&count_var).unwrap_or(0);
        let mut emails = Vec::new();

        for i in 1..=count {
            let item_var = items.call_method("Item", &mut [VARIANT::from(i)])?;
            let item_dispatch = IDispatch::try_from(&item_var);
            if let Ok(dispatch) = item_dispatch {
                let item = ComDispatch(dispatch);
                if let Ok(mut email) = self.map_item_to_email(&item) {
                    email.folder = folder_name.to_string();
                    emails.push(email);
                } else {
                    tracing::warn!(
                        "Failed to map Outlook item to Email struct in {} at index {}",
                        folder_name,
                        i
                    );
                }
            }
        }

        Ok(emails)
    }

    fn map_item_to_email(&self, item: &ComDispatch) -> Result<Email> {
        let entry_id_var = item.get_property("EntryID")?;
        let entry_id_bstr = BSTR::try_from(&entry_id_var)
            .map_err(|_| NoodleError::Outlook("Invalid EntryID".into()))?;
        let entry_id = entry_id_bstr.to_string();

        let subject_var = item.get_property("Subject")?;
        let subject = BSTR::try_from(&subject_var)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| "No Subject".into());

        let body_var = item.get_property("Body")?;
        let body_text = BSTR::try_from(&body_var)
            .map(|s| s.to_string())
            .unwrap_or_default();

        let sender_var = item.get_property("SenderEmailAddress")?;
        let sender = BSTR::try_from(&sender_var)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| "Unknown".into());

        let received_at_var = item.get_property("ReceivedTime")?;
        let received_at_double = f64::try_from(&received_at_var).unwrap_or(0.0);

        // Convert DATE (double) to chrono::DateTime<Utc>
        // OLE Automation DATE is days since Dec 30, 1899
        let unix_epoch_offset_days = 25569.0;
        let seconds_in_day = 86400.0;
        let unix_timestamp = (received_at_double - unix_epoch_offset_days) * seconds_in_day;
        let received_at =
            DateTime::from_timestamp(unix_timestamp as i64, 0).unwrap_or_else(|| Utc::now());

        Ok(Email {
            id: 0, // Assigned by storage
            store_id: "outlook".into(),
            entry_id,
            conversation_id: None, // Could be extracted if needed
            folder: "Inbox".into(),
            subject,
            sender,
            to: "".into(), // Could be extracted if needed
            cc: None,
            bcc: None,
            sent_at: received_at, // Simplification
            received_at,
            body_text,
            body_html: None,
            importance: 1, // Normal
            categories: None,
            flags: None,
            internet_message_id: None,
            last_indexed_at: Utc::now(),
            hash: "".into(), // Computed by sync manager
            excluded_reason: None,
        })
    }
}
