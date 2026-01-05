//! Windows COM (Component Object Model) wrapper for Outlook automation.
//!
//! This module provides a safe Rust wrapper around the Windows IDispatch interface,
//! which is used to interact with Outlook on Windows via COM automation. It enables
//! dynamic method calls and property access on COM objects without requiring compile-time
//! type information.
//!
//! # Platform
//! This module is only compiled on Windows (`#[cfg(windows)]`).
//!
//! # Usage
//! The `ComDispatch` wrapper is used by the Windows Outlook client to:
//! - Get Outlook application instance
//! - Access mail folders (Inbox, Sent Items, etc.)
//! - Retrieve email properties (subject, sender, body, etc.)

use noodle_core::error::{NoodleError, Result};
use windows::core::{BSTR, PCWSTR, VARIANT};
use windows::Win32::System::Com::{
    IDispatch, DISPATCH_FLAGS, DISPATCH_METHOD, DISPATCH_PROPERTYGET, DISPPARAMS, EXCEPINFO,
};

/// Default locale for COM operations (user's default locale).
const LOCALE_USER_DEFAULT: u32 = 0x0400;

/// A wrapper around Windows IDispatch interface for dynamic COM automation.
///
/// `ComDispatch` simplifies making dynamic calls to COM objects like Outlook.
/// It provides methods to get properties and call methods on COM objects
/// without needing to know their interfaces at compile time.
///
/// # Thread Safety
/// Marked as `Send` and `Sync` to allow use across async contexts.
/// The actual thread safety depends on the COM threading model being used.
pub struct ComDispatch(pub IDispatch);

unsafe impl Send for ComDispatch {}
unsafe impl Sync for ComDispatch {}

impl ComDispatch {
    /// Gets a property value from the COM object.
    ///
    /// # Arguments
    /// * `name` - The name of the property to retrieve (e.g., "Subject", "Body")
    ///
    /// # Returns
    /// A `VARIANT` containing the property value, or an error if the property doesn't exist.
    ///
    /// # Example
    /// ```ignore
    /// let subject = com_obj.get_property("Subject")?;
    /// ```
    pub fn get_property(&self, name: &str) -> Result<VARIANT> {
        self.invoke(name, DISPATCH_PROPERTYGET.0 as u32, &mut [])
    }

    /// Calls a method on the COM object with optional arguments.
    ///
    /// # Arguments
    /// * `name` - The name of the method to call (e.g., "GetDefaultFolder")
    /// * `args` - Mutable slice of VARIANT arguments to pass to the method
    ///
    /// # Returns
    /// A `VARIANT` containing the method's return value, or an error if the call fails.
    ///
    /// # Note
    /// Arguments are automatically reversed internally as COM expects them in reverse order.
    pub fn call_method(&self, name: &str, args: &mut [VARIANT]) -> Result<VARIANT> {
        self.invoke(name, DISPATCH_METHOD.0 as u32, args)
    }

    /// Internal method that performs the actual COM invocation.
    ///
    /// This method:
    /// 1. Resolves the method/property name to a dispatch ID (DISPID)
    /// 2. Sets up the DISPPARAMS structure with arguments
    /// 3. Calls IDispatch::Invoke to execute the operation
    ///
    /// # Arguments
    /// * `name` - The name of the method or property
    /// * `flags` - Dispatch flags (DISPATCH_METHOD or DISPATCH_PROPERTYGET)
    /// * `args` - Arguments to pass (will be reversed for COM convention)
    fn invoke(&self, name: &str, flags: u32, args: &mut [VARIANT]) -> Result<VARIANT> {
        let mut dispid = 0;
        let name_bstr = BSTR::from(name);

        unsafe {
            // Get the dispatch ID for the named method/property
            let name_pcwstr = PCWSTR(name_bstr.as_ptr());
            self.0
                .GetIDsOfNames(
                    &windows::core::GUID::zeroed(),
                    &name_pcwstr,
                    1,
                    LOCALE_USER_DEFAULT,
                    &mut dispid,
                )
                .map_err(|e| {
                    NoodleError::Outlook(format!("Failed to get ID for {}: {}", name, e))
                })?;

            // Set up parameters - COM expects arguments in reverse order
            let mut params = DISPPARAMS::default();
            if !args.is_empty() {
                args.reverse();
                params.cArgs = args.len() as u32;
                params.rgvarg = args.as_mut_ptr() as *mut VARIANT;
            }

            // Invoke the method/property
            let mut result = VARIANT::default();
            let mut excep_info = EXCEPINFO::default();
            let mut arg_err = 0;

            self.0
                .Invoke(
                    dispid,
                    &windows::core::GUID::zeroed(),
                    LOCALE_USER_DEFAULT,
                    DISPATCH_FLAGS(flags as u16),
                    &params,
                    Some(&mut result),
                    Some(&mut excep_info),
                    Some(&mut arg_err),
                )
                .map_err(|e| NoodleError::Outlook(format!("Failed to invoke {}: {}", name, e)))?;

            Ok(result)
        }
    }
}

/// Allows creating a `ComDispatch` directly from an `IDispatch` interface.
impl From<IDispatch> for ComDispatch {
    fn from(d: IDispatch) -> Self {
        Self(d)
    }
}
