// Platform-specific Outlook client implementations

#[cfg(windows)]
mod windows_impl;

#[cfg(target_os = "macos")]
mod macos_impl;

use noodle_core::error::Result;
use noodle_core::types::Email;

/// Cross-platform Outlook client
#[derive(Clone)]
pub struct OutlookClient {
    #[cfg(windows)]
    inner: windows_impl::WindowsOutlookClient,

    #[cfg(target_os = "macos")]
    inner: macos_impl::MacOutlookClient,
}

impl OutlookClient {
    pub fn new() -> Result<Self> {
        #[cfg(windows)]
        {
            Ok(Self {
                inner: windows_impl::WindowsOutlookClient::new()?,
            })
        }

        #[cfg(target_os = "macos")]
        {
            Ok(Self {
                inner: macos_impl::MacOutlookClient::new()?,
            })
        }

        #[cfg(not(any(windows, target_os = "macos")))]
        {
            Err(noodle_core::error::NoodleError::Outlook(
                "Outlook client is only supported on Windows and macOS".to_string(),
            ))
        }
    }

    pub async fn get_emails_last_n_days(
        &self,
        days: i64,
        folder_id: i32,
        folder_name: &str,
    ) -> Result<Vec<Email>> {
        self.inner
            .get_emails_last_n_days(days, folder_id, folder_name)
            .await
    }
}
