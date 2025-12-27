use noodle_core::error::{NoodleError, Result};
use windows::core::{BSTR, PCWSTR, VARIANT};
use windows::Win32::System::Com::{
    IDispatch, DISPATCH_FLAGS, DISPATCH_METHOD, DISPATCH_PROPERTYGET, DISPPARAMS, EXCEPINFO,
};

const LOCALE_USER_DEFAULT: u32 = 0x0400;

/// A wrapper around IDispatch to make dynamic calls easier.
pub struct ComDispatch(pub IDispatch);

impl ComDispatch {
    pub fn get_property(&self, name: &str) -> Result<VARIANT> {
        self.invoke(name, DISPATCH_PROPERTYGET.0 as u32, &mut [])
    }

    pub fn call_method(&self, name: &str, args: &mut [VARIANT]) -> Result<VARIANT> {
        self.invoke(name, DISPATCH_METHOD.0 as u32, args)
    }

    fn invoke(&self, name: &str, flags: u32, args: &mut [VARIANT]) -> Result<VARIANT> {
        let mut dispid = 0;
        let name_bstr = BSTR::from(name);

        unsafe {
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

impl From<IDispatch> for ComDispatch {
    fn from(d: IDispatch) -> Self {
        Self(d)
    }
}
