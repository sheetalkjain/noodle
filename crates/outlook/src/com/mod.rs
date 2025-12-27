use core::error::{NoodleError, Result};
use std::ptr;
use windows::core::{BSTR, HRESULT, VARIANT};
use windows::Win32::System::Com::{
    IDispatch, DISPATCH_FLAGS, DISPATCH_METHOD, DISPATCH_PROPERTYGET, DISPPARAMS,
};
use windows::Win32::System::Ole::{DISPID_PROPERTYPUT, EXCEPINFO};

/// A wrapper around IDispatch to make dynamic calls easier.
pub struct ComDispatch(pub IDispatch);

impl ComDispatch {
    pub fn get_property(&self, name: &str) -> Result<VARIANT> {
        self.invoke(name, DISPATCH_PROPERTYGET, &mut [])
    }

    pub fn call_method(&self, name: &str, args: &mut [VARIANT]) -> Result<VARIANT> {
        self.invoke(name, DISPATCH_METHOD, args)
    }

    fn invoke(&self, name: &str, flags: u32, args: &mut [VARIANT]) -> Result<VARIANT> {
        unsafe {
            let mut dispid = 0;
            let name_bstr = BSTR::from(name);

            self.0
                .GetIDsOfNames(
                    &windows::core::GUID::zeroed(),
                    &name_bstr,
                    1,
                    windows::Win32::System::Com::LOCALE_USER_DEFAULT,
                    &mut dispid,
                )
                .map_err(|e| {
                    NoodleError::Outlook(format!("Failed to get ID for {}: {}", name, e))
                })?;

            let mut params = DISPPARAMS::default();
            if !args.is_empty() {
                args.reverse(); // COM args are passed in reverse order
                params.cArgs = args.len() as u32;
                params.rgvarg = args.as_mut_ptr() as *mut VARIANT;
            }

            let mut result = VARIANT::default();
            let mut excep_info = EXCEPINFO::default();
            let mut arg_err = 0;

            self.0
                .Invoke(
                    dispid,
                    &windows::core::GUID::zeroed(),
                    windows::Win32::System::Com::LOCALE_USER_DEFAULT,
                    DISPATCH_FLAGS(flags),
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

impl From<IDispatch> for ComDispatch {
    fn from(d: IDispatch) -> Self {
        Self(d)
    }
}
