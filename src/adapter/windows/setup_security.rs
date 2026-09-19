use crate::adapter::setup_state::reject_link;
use std::{path::Path, ptr};
use windows_sys::Win32::{
    Foundation::{GetLastError, LocalFree, ERROR_ALREADY_EXISTS},
    Security::{Authorization::*, *},
    Storage::FileSystem::CreateDirectoryW,
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

struct Descriptor(PSECURITY_DESCRIPTOR);
impl Drop for Descriptor {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0);
        }
    }
}

pub fn require_elevation() -> Result<(), String> {
    unsafe {
        let mut token = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err("SETUP_ELEVATION_REQUIRED".into());
        }
        let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
        let mut length = 0;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut length,
        );
        windows_sys::Win32::Foundation::CloseHandle(token);
        if ok == 0 || elevation.TokenIsElevated == 0 {
            return Err("SETUP_ELEVATION_REQUIRED".into());
        }
    }
    Ok(())
}

pub fn check_ancestors(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("SETUP_PATH_INVALID".into());
    }
    for ancestor in path.ancestors() {
        reject_link(ancestor)?;
    }
    Ok(())
}

fn descriptor(sddl: &str) -> Result<Descriptor, String> {
    unsafe {
        let mut raw = ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide(sddl).as_ptr(),
            1,
            &mut raw,
            ptr::null_mut(),
        ) == 0
        {
            return Err("SETUP_ACL_FAILED".into());
        }
        Ok(Descriptor(raw))
    }
}

fn dacl_string(sd: PSECURITY_DESCRIPTOR) -> Result<String, String> {
    unsafe {
        let mut text = ptr::null_mut();
        if ConvertSecurityDescriptorToStringSecurityDescriptorW(
            sd,
            1,
            DACL_SECURITY_INFORMATION,
            &mut text,
            ptr::null_mut(),
        ) == 0
        {
            return Err("SETUP_ACL_VERIFY_FAILED".into());
        }
        let mut len = 0;
        while *text.add(len) != 0 {
            len += 1;
        }
        let result = String::from_utf16_lossy(std::slice::from_raw_parts(text, len));
        LocalFree(text as *mut _);
        Ok(result)
    }
}

fn verify_dacl(path: &Path, expected: PSECURITY_DESCRIPTOR) -> Result<(), String> {
    unsafe {
        let mut raw = ptr::null_mut();
        if GetNamedSecurityInfoW(
            wide(path.to_str().ok_or("SETUP_PATH_INVALID")?).as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut raw,
        ) != 0
        {
            return Err("SETUP_ACL_VERIFY_FAILED".into());
        }
        let observed = Descriptor(raw);
        // Windows may add the auto-inheritance bookkeeping flag when propagating ACEs.
        let actual = dacl_string(observed.0)?.replacen("D:PAI", "D:P", 1);
        if actual != dacl_string(expected)? {
            return Err("SETUP_ACL_VERIFY_FAILED".into());
        }
        Ok(())
    }
}

/// Create with an explicit ACL atomically; existing setup directories must match it exactly.
pub fn protected_dir(path: &Path, allow_existing: bool) -> Result<(), String> {
    check_ancestors(path)?;
    let expected = "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)";
    let sd = descriptor(&format!("O:BAG:BA{expected}"))?;
    let name = wide(path.to_str().ok_or("SETUP_PATH_INVALID")?);
    unsafe {
        let attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd.0,
            bInheritHandle: 0,
        };
        if CreateDirectoryW(name.as_ptr(), &attributes) == 0 {
            if GetLastError() != ERROR_ALREADY_EXISTS || !allow_existing {
                return Err("SETUP_DIRECTORY_CONFLICT".into());
            }
            if !path.is_dir() {
                return Err("SETUP_DIRECTORY_CONFLICT".into());
            }
        }
        let mut raw = ptr::null_mut();
        if GetNamedSecurityInfoW(
            name.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut raw,
        ) != 0
        {
            return Err("SETUP_ACL_VERIFY_FAILED".into());
        }
        let observed = Descriptor(raw);
        let mut text = ptr::null_mut();
        if ConvertSecurityDescriptorToStringSecurityDescriptorW(
            observed.0,
            1,
            DACL_SECURITY_INFORMATION,
            &mut text,
            ptr::null_mut(),
        ) == 0
        {
            return Err("SETUP_ACL_VERIFY_FAILED".into());
        }
        let mut len = 0;
        while *text.add(len) != 0 {
            len += 1;
        }
        let actual = String::from_utf16_lossy(std::slice::from_raw_parts(text, len));
        LocalFree(text as *mut _);
        if actual != expected {
            return Err("SETUP_ACL_VERIFY_FAILED".into());
        }
    }
    Ok(())
}

pub fn grant_runtime(path: &Path, sid: &str, writable: bool) -> Result<(), String> {
    check_ancestors(path)?;
    if !sid.starts_with("S-1-")
        || !sid
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'-' || b == b'S')
    {
        return Err("ACCOUNT_SID_FAILED".into());
    }
    let rights = if writable { "FA" } else { "FRFX" };
    let sd = descriptor(&format!(
        "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;{rights};;;{sid})"
    ))?;
    unsafe {
        let mut dacl = ptr::null_mut();
        let mut present = 0;
        let mut defaulted = 0;
        if GetSecurityDescriptorDacl(sd.0, &mut present, &mut dacl, &mut defaulted) == 0
            || present == 0
        {
            return Err("SETUP_ACL_FAILED".into());
        }
        if SetNamedSecurityInfoW(
            wide(path.to_str().ok_or("SETUP_PATH_INVALID")?).as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            dacl,
            ptr::null_mut(),
        ) != 0
        {
            return Err("SETUP_ACL_FAILED".into());
        }
    }
    verify_dacl(path, sd.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_acl_has_only_system_admin_and_runtime_identity() {
        for rights in ["FA", "FRFX"] {
            let sd = descriptor(&format!(
                "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;{rights};;;S-1-5-21-1-2-3-1001)"
            ))
            .unwrap();
            let text = dacl_string(sd.0).unwrap();
            assert!(text.starts_with("D:P"));
            assert_eq!(text.matches("(A;").count(), 3);
            assert!(!text.contains(";;;BU)"));
            assert!(!text.contains(";;;WD)"));
        }
    }
}

pub fn operator_sid() -> Result<String, String> {
    unsafe {
        let mut token = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err("SETUP_OPERATOR_FAILED".into());
        }
        let mut size = 0;
        GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut size);
        let mut buffer = vec![
            0usize;
            (size as usize + std::mem::size_of::<usize>() - 1)
                / std::mem::size_of::<usize>()
        ];
        let ok = GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr() as *mut _,
            size,
            &mut size,
        );
        windows_sys::Win32::Foundation::CloseHandle(token);
        if ok == 0 {
            return Err("SETUP_OPERATOR_FAILED".into());
        }
        let user = &*(buffer.as_ptr() as *const TOKEN_USER);
        let mut sid = ptr::null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut sid) == 0 {
            return Err("SETUP_OPERATOR_FAILED".into());
        }
        let mut len = 0;
        while *sid.add(len) != 0 {
            len += 1;
        }
        let result = String::from_utf16_lossy(std::slice::from_raw_parts(sid, len));
        LocalFree(sid as *mut _);
        Ok(result)
    }
}
