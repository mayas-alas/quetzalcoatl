use std::ffi::OsString;
use std::mem::size_of;
use std::path::Path;
use std::ptr::{null, null_mut};

use windows_sys::Win32::NetworkManagement::NetManagement::{
    NERR_Success, NERR_UserExists, NetUserAdd, NetUserSetInfo, UF_DONT_EXPIRE_PASSWD,
    UF_NORMAL_ACCOUNT, USER_INFO_1, USER_INFO_1003, USER_PRIV_USER,
};
use windows_sys::Win32::Security::Authentication::Identity::{
    LSA_HANDLE, LSA_OBJECT_ATTRIBUTES, LSA_UNICODE_STRING, LsaAddAccountRights, LsaClose,
    LsaNtStatusToWinError, LsaOpenPolicy, POLICY_CREATE_ACCOUNT, POLICY_LOOKUP_NAMES,
};
use windows_sys::Win32::Security::{LookupAccountNameW, PSID, SID_NAME_USE};

use crate::error::GnxError;
use crate::process::CommandSpec;

pub const SERVICE_NAME: &str = "QuetzalcoatlNext";
pub const SERVICE_DISPLAY_NAME: &str = "Quetzalcoatl Next";
pub const RUNTIME_ACCOUNT_NAME: &str = "gnx-runtime";
pub const SERVICE_ACCOUNT: &str = r".\gnx-runtime";

pub struct RuntimeCredential {
    pub account_name: OsString,
    pub password: OsString,
}

pub fn ensure_runtime_account() -> Result<RuntimeCredential, GnxError> {
    let password = crate::secrets::random_hex(32)?;
    let mut name = wide(RUNTIME_ACCOUNT_NAME);
    let mut password_wide = wide(&password);
    let mut comment = wide("Quetzalcoatl Next isolated runtime");
    let mut parameter_error = 0;
    let info = USER_INFO_1 {
        usri1_name: name.as_mut_ptr(),
        usri1_password: password_wide.as_mut_ptr(),
        usri1_password_age: 0,
        usri1_priv: USER_PRIV_USER,
        usri1_home_dir: null_mut(),
        usri1_comment: comment.as_mut_ptr(),
        usri1_flags: UF_NORMAL_ACCOUNT | UF_DONT_EXPIRE_PASSWD,
        usri1_script_path: null_mut(),
    };

    // SAFETY: USER_INFO_1 points to live, NUL-terminated UTF-16 buffers for this call.
    let added = unsafe {
        NetUserAdd(
            null(),
            1,
            (&info as *const USER_INFO_1).cast(),
            &mut parameter_error,
        )
    };
    match added {
        status if status == NERR_Success => {}
        status if status == NERR_UserExists => {
            let password_info = USER_INFO_1003 {
                usri1003_password: password_wide.as_mut_ptr(),
            };
            // SAFETY: account name and password are live, NUL-terminated UTF-16 buffers.
            let updated = unsafe {
                NetUserSetInfo(
                    null(),
                    name.as_ptr(),
                    1003,
                    (&password_info as *const USER_INFO_1003).cast(),
                    &mut parameter_error,
                )
            };
            if updated != NERR_Success {
                return Err(account_error("runtime_account_password", updated));
            }
        }
        status => return Err(account_error("runtime_account_create", status)),
    }

    grant_account_rights()?;
    hide_from_logon_ui()?;
    Ok(RuntimeCredential {
        account_name: OsString::from(SERVICE_ACCOUNT),
        password: OsString::from(password),
    })
}

pub fn grant_data_access(path: &Path) -> Result<(), GnxError> {
    let grant = format!(r"{SERVICE_ACCOUNT}:(OI)(CI)M");
    CommandSpec::new(r"C:\Windows\System32\icacls.exe")
        .arg(path)
        .args([
            "/inheritance:r",
            "/grant:r",
            r"*S-1-5-18:(OI)(CI)F",
            r"*S-1-5-32-544:(OI)(CI)F",
            &grant,
            r"*S-1-5-32-545:(OI)(CI)RX",
            "/T",
            "/C",
        ])
        .run_checked("service_data_acl")?;
    Ok(())
}

fn grant_account_rights() -> Result<(), GnxError> {
    let account = wide(RUNTIME_ACCOUNT_NAME);
    let mut sid_length = 0_u32;
    let mut domain_length = 0_u32;
    let mut sid_type: SID_NAME_USE = 0;
    // SAFETY: this sizing call intentionally passes null output buffers.
    unsafe {
        LookupAccountNameW(
            null(),
            account.as_ptr(),
            null_mut(),
            &mut sid_length,
            null_mut(),
            &mut domain_length,
            &mut sid_type,
        );
    }
    if sid_length == 0 {
        return Err(GnxError::io(
            "runtime_account_sid",
            std::io::Error::last_os_error().to_string(),
        ));
    }

    let mut sid = vec![0_u8; sid_length as usize];
    let mut domain = vec![0_u16; domain_length as usize];
    // SAFETY: buffers use the exact sizes requested by LookupAccountNameW.
    let found = unsafe {
        LookupAccountNameW(
            null(),
            account.as_ptr(),
            sid.as_mut_ptr().cast::<core::ffi::c_void>() as PSID,
            &mut sid_length,
            domain.as_mut_ptr(),
            &mut domain_length,
            &mut sid_type,
        )
    };
    if found == 0 {
        return Err(GnxError::io(
            "runtime_account_sid",
            std::io::Error::last_os_error().to_string(),
        ));
    }

    let attributes = LSA_OBJECT_ATTRIBUTES {
        Length: size_of::<LSA_OBJECT_ATTRIBUTES>() as u32,
        ..Default::default()
    };
    let mut policy: LSA_HANDLE = 0;
    // SAFETY: attributes and output policy handle are valid for the duration of the call.
    let opened = unsafe {
        LsaOpenPolicy(
            null(),
            &attributes,
            (POLICY_LOOKUP_NAMES | POLICY_CREATE_ACCOUNT) as u32,
            &mut policy,
        )
    };
    if opened != 0 {
        return Err(lsa_error("runtime_account_policy", opened));
    }

    let rights_storage = [
        wide("SeServiceLogonRight"),
        wide("SeDenyInteractiveLogonRight"),
        wide("SeDenyRemoteInteractiveLogonRight"),
        wide("SeDenyNetworkLogonRight"),
    ];
    let rights: Vec<LSA_UNICODE_STRING> = rights_storage
        .iter()
        .map(|right| LSA_UNICODE_STRING {
            Length: ((right.len() - 1) * 2) as u16,
            MaximumLength: (right.len() * 2) as u16,
            Buffer: right.as_ptr() as *mut u16,
        })
        .collect();
    // SAFETY: SID, policy and right buffers remain valid until the call returns.
    let status = unsafe {
        LsaAddAccountRights(
            policy,
            sid.as_mut_ptr().cast::<core::ffi::c_void>() as PSID,
            rights.as_ptr(),
            rights.len() as u32,
        )
    };
    // SAFETY: policy was returned by LsaOpenPolicy and is closed exactly once.
    unsafe { LsaClose(policy) };
    if status != 0 {
        return Err(lsa_error("runtime_account_rights", status));
    }
    Ok(())
}

fn hide_from_logon_ui() -> Result<(), GnxError> {
    CommandSpec::new(r"C:\Windows\System32\reg.exe")
        .args([
            "ADD",
            r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon\SpecialAccounts\UserList",
            "/v",
            RUNTIME_ACCOUNT_NAME,
            "/t",
            "REG_DWORD",
            "/d",
            "0",
            "/f",
        ])
        .run_checked("runtime_account_hide")?;
    Ok(())
}

fn account_error(operation: &'static str, status: u32) -> GnxError {
    GnxError::new(
        "INSTALL_RUNTIME_ACCOUNT_FAILED",
        "install",
        operation,
        format!("NetAPI devolvió {status}; parámetro inválido si aplica."),
        "Ejecute gnx repair desde una consola elevada.",
        true,
        14,
    )
}

fn lsa_error(operation: &'static str, status: i32) -> GnxError {
    // SAFETY: LsaNtStatusToWinError accepts the NTSTATUS returned by LSA APIs.
    let windows_error = unsafe { LsaNtStatusToWinError(status) };
    GnxError::new(
        "INSTALL_RUNTIME_ACCOUNT_RIGHTS_FAILED",
        "install",
        operation,
        format!("LSA devolvió {status:#x} (Win32 {windows_error})."),
        "Revise la política local de derechos de usuario y ejecute gnx repair.",
        false,
        14,
    )
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_account_is_separate_from_service_name() {
        assert_eq!(SERVICE_ACCOUNT, format!(r".\{RUNTIME_ACCOUNT_NAME}"));
        assert_ne!(RUNTIME_ACCOUNT_NAME, SERVICE_NAME);
    }
}
