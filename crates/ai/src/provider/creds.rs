use noodle_core::error::{NoodleError, Result};
use std::ptr;
use windows::core::{BSTR, PWSTR};
use windows::Win32::Security::Credentials::{
    CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
};

pub struct CredentialStore;

impl CredentialStore {
    pub fn save_api_key(provider: &str, key: &str) -> Result<()> {
        let target = format!("Noodle/{}", provider);
        let target_bstr = BSTR::from(target.as_str());
        let key_bytes = key.as_bytes();

        unsafe {
            let cred = CREDENTIALW {
                Type: CRED_TYPE_GENERIC,
                TargetName: PWSTR(target_bstr.as_ptr() as *mut _),
                CredentialBlob: key_bytes.as_ptr() as *mut _,
                CredentialBlobSize: key_bytes.len() as u32,
                Persist: CRED_PERSIST_LOCAL_MACHINE,
                ..Default::default()
            };

            CredWriteW(&cred, 0)
                .map_err(|e| NoodleError::Internal(format!("Failed to write credential: {}", e)))?;
        }

        Ok(())
    }

    pub fn get_api_key(provider: &str) -> Result<Option<String>> {
        let target = format!("Noodle/{}", provider);
        let target_bstr = BSTR::from(target.as_str());

        unsafe {
            let mut cred_ptr: *mut CREDENTIALW = ptr::null_mut();
            let res = CredReadW(&target_bstr, CRED_TYPE_GENERIC, 0, &mut cred_ptr);

            if res.is_err() {
                return Ok(None);
            }

            let cred = &*cred_ptr;
            let blob =
                std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize);
            let key = String::from_utf8_lossy(blob).to_string();

            // Note: In real usage, you must call CredFree(cred_ptr)

            Ok(Some(key))
        }
    }
}
