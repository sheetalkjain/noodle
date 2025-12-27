use crate::com::ComDispatch;
use chrono::{DateTime, Duration, Utc};
use noodle_core::error::{NoodleError, Result};
use noodle_core::types::Email;
use tracing::{info, warn};
use windows::core::VARIANT;
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
            let namespace: IDispatch = namespace_var.to_interface().map_err(|e| {
                NoodleError::Outlook(format!("Failed to get Namespace dispatch: {}", e))
            })?;

            Ok(Self {
                namespace: ComDispatch(namespace),
            })
        }
    }

    pub fn get_emails_last_n_days(&self, days: i64) -> Result<Vec<Email>> {
        let folder_var = self
            .namespace
            .call_method("GetDefaultFolder", &mut [VARIANT::from(6i32)])?; // 6 = olFolderInbox
        let inbox = ComDispatch(
            folder_var
                .to_interface()
                .map_err(|e| NoodleError::Outlook(format!("Failed to get Inbox: {}", e)))?,
        );

        let items_var = inbox.get_property("Items")?;
        let items = ComDispatch(
            items_var
                .to_interface()
                .map_err(|e| NoodleError::Outlook(format!("Failed to get Items: {}", e)))?,
        );

        // Filter items
        let filter_date = Utc::now() - Duration::days(days);
        let filter = format!(
            "[ReceivedTime] >= '{}'",
            filter_date.format("%m/%d/%Y %H:%M %p")
        );
        let filtered_items_var =
            items.call_method("Restrict", &mut [VARIANT::from(filter.as_str())])?;
        let filtered_items = ComDispatch(
            filtered_items_var
                .to_interface()
                .map_err(|e| NoodleError::Outlook(format!("Failed to restrict items: {}", e)))?,
        );

        self.parse_items(filtered_items)
    }

    fn parse_items(&self, items: ComDispatch) -> Result<Vec<Email>> {
        let count_var = items.get_property("Count")?;
        let count = count_var.as_i32().unwrap_or(0);
        let mut emails = Vec::new();

        for i in 1..=count {
            let item_var = items.call_method("Item", &mut [VARIANT::from(i)])?;
            let item_dispatch: Result<IDispatch> = item_var.to_interface().map_err(|e| {
                NoodleError::Outlook(format!("Failed to get IDispatch from VARIANT: {}", e))
            });
            if let Ok(dispatch) = item_dispatch {
                let item = ComDispatch(dispatch);
                if let Ok(email) = self.map_item_to_email(&item) {
                    emails.push(email);
                }
            }
        }

        Ok(emails)
    }

    fn map_item_to_email(&self, _item: &ComDispatch) -> Result<Email> {
        // This would extract all fields from the MailItem
        // EntryID, Subject, Body, SenderEmailAddress, etc.
        // For the sake of the implementation, return a skeleton
        unimplemented!("MailItem field extraction logic goes here")
    }
}
