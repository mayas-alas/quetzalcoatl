use std::mem::size_of;
use std::path::Path;
use std::ptr::{null, null_mut};
use std::process::Command;

use windows_sys::Win32::NetworkManagement::NetManagement::{
    NERR_Success, NERR_UserExists, NetUserAdd, NetUserSetInfo, UF_DONT_EXPIRE_PASSWD,
    UF_NORMAL_ACCOUNT, USER_INFO_1, USER_INFO_1003, USER_PRIV_USER,
};
use windows_sys::Win32::Security::Authentication::Identity::{
    LSA_HANDLE, LSA_OBJECT_ATTRIBUTES, LSA_UNICODE_STRING, LsaAddAccountRights, LsaClose,
    LsaOpenPolicy, POLICY_CREATE_ACCOUNT, POLICY_LOOKUP_NAMES,
};
use windows_sys::Win32::Security::{LookupAccountNameW, PSID, SID_NAME_USE};

use crate::{Error, Result};

pub const RUNTIME_ACCOUNT_NAME: &str = "gnx-runtime";
pub const SERVICE_ACCOUNT: &str = r".\gnx-runtime";

pub struct RuntimeCredential {
    pub account_name: String,
    pub password: String,
}

pub fn ensure_runtime_account() -> Result<RuntimeCredential> {
    let password = random_password()?;
    let mut name = wide(RUNTIME_ACCOUNT_NAME);
    let mut password_wide = wide(&password);
    let mut comment = wide("GNX isolated runtime");
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

    // SAFETY: USER_INFO_1 points to live, NUL-terminated UTF-16 buffers.
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
            // SAFETY: account name and password buffers remain valid for this call.
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
                return Err(Error::Operation("RUNTIME_ACCOUNT_PASSWORD"));
            }
        }
        _ => return Err(Error::Operation("RUNTIME_ACCOUNT_CREATE")),
    }

    grant_account_rights()?;
    hide_from_logon_ui()?;
    Ok(RuntimeCredential {
        account_name: SERVICE_ACCOUNT.into(),
        password,
    })
}

pub fn grant_data_access(path: &Path, operator_sid: &str) -> Result<()> {
    if !valid_sid(operator_sid) {
        return Err(Error::Operation("OPERATOR_SID"));
    }
    std::fs::create_dir_all(path).map_err(Error::ConfigRead)?;

    let runtime = format!(r"{RUNTIME_ACCOUNT_NAME}:(OI)(CI)F");
    run(
        Command::new(r"C:\Windows\System32\icacls.exe")
            .arg(path)
            .args([
                "/inheritance:r",
                "/grant:r",
                r"*S-1-5-18:(OI)(CI)F",
                r"*S-1-5-32-544:(OI)(CI)F",
                runtime.as_str(),
            ]),
        "RUNTIME_DATA_ACL",
    )?;

    // Recreate after the parent DACL is private so the SID file inherits it.
    let operator_file = path.join("operator.sid");
    let _ = std::fs::remove_file(&operator_file);
    std::fs::write(operator_file, operator_sid).map_err(Error::ConfigRead)?;
    Ok(())
}

fn grant_account_rights() -> Result<()> {
    let account = wide(RUNTIME_ACCOUNT_NAME);
    let mut sid_length = 0_u32;
    let mut domain_length = 0_u32;
    let mut sid_type: SID_NAME_USE = 0;
    // SAFETY: sizing call intentionally passes null output buffers.
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
        return Err(Error::Operation("RUNTIME_ACCOUNT_SID"));
    }

    let mut sid = vec![0_u8; sid_length as usize];
    let mut domain = vec![0_u16; domain_length as usize];
    // SAFETY: buffers use the sizes requested by LookupAccountNameW.
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
        return Err(Error::Operation("RUNTIME_ACCOUNT_SID"));
    }

    let attributes = LSA_OBJECT_ATTRIBUTES {
        Length: size_of::<LSA_OBJECT_ATTRIBUTES>() as u32,
        ..Default::default()
    };
    let mut policy: LSA_HANDLE = 0;
    // SAFETY: attributes and output handle are valid for the call.
    let opened = unsafe {
        LsaOpenPolicy(
            null(),
            &attributes,
            (POLICY_LOOKUP_NAMES | POLICY_CREATE_ACCOUNT) as u32,
            &mut policy,
        )
    };
    if opened != 0 {
        return Err(lsa_error(opened));
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
    // SAFETY: policy was returned by LsaOpenPolicy and is closed once.
    unsafe { LsaClose(policy) };
    if status != 0 {
        return Err(lsa_error(status));
    }
    Ok(())
}

fn hide_from_logon_ui() -> Result<()> {
    run(
        Command::new(r"C:\Windows\System32\reg.exe").args([
            "ADD",
            r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon\SpecialAccounts\UserList",
            "/v",
            RUNTIME_ACCOUNT_NAME,
            "/t",
            "REG_DWORD",
            "/d",
            "0",
            "/f",
        ]),
        "RUNTIME_ACCOUNT_HIDE",
    )
}

fn random_password() -> Result<String> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| Error::Operation("SYSTEM_ENTROPY"))?;
    let mut value = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut value, "{byte:02x}").map_err(|_| Error::Operation("SYSTEM_ENTROPY"))?;
    }
    Ok(value)
}

fn valid_sid(value: &str) -> bool {
    value.starts_with("S-1-")
        && value.len() <= 184
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'-' || byte == b'S')
}

fn lsa_error(_status: i32) -> Error {
    Error::Operation("RUNTIME_ACCOUNT_RIGHTS")
}

fn run(command: &mut Command, operation: &'static str) -> Result<()> {
    let status = command.status().map_err(Error::Spawn)?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Operation(operation))
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_identity_is_not_interactive() {
        assert_eq!(SERVICE_ACCOUNT, r".\gnx-runtime");
        assert!(valid_sid("S-1-5-21-1-2-3-1001"));
        assert!(!valid_sid("BUILTIN\\Users"));
    }
}
