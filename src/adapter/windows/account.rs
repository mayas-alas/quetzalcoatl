use std::ptr;
use windows_sys::Win32::{
    Foundation::{
        GetLastError, LocalFree, ERROR_INSUFFICIENT_BUFFER, ERROR_SERVICE_DOES_NOT_EXIST,
    },
    NetworkManagement::NetManagement::*,
    Security::{
        Authentication::Identity::*,
        Authorization::ConvertSidToStringSidW,
        Cryptography::{BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG},
        LookupAccountNameW,
    },
    System::Services::*,
};
use zeroize::Zeroizing;
pub const SERVICE_ACCOUNT: &str = ".\\gnx-runtime";
const ACCOUNT_NAME: &str = "gnx-runtime";
const SERVICE_NAME: &str = "GNXRuntime";
pub const PRIVATE_ROOT: &str = super::setup::TARGET_DATA;
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn target_binary_path() -> String {
    format!("\"{}\\gnx-service.exe\"", super::setup::TARGET_PROGRAM)
}
struct Sc(SC_HANDLE);
impl Drop for Sc {
    fn drop(&mut self) {
        unsafe {
            CloseServiceHandle(self.0);
        }
    }
}
struct Policy(LSA_HANDLE);
impl Drop for Policy {
    fn drop(&mut self) {
        unsafe {
            LsaClose(self.0);
        }
    }
}

struct AccountGuard {
    active: bool,
}
impl Drop for AccountGuard {
    fn drop(&mut self) {
        if self.active {
            unsafe {
                NetUserDel(ptr::null(), wide(ACCOUNT_NAME).as_ptr());
            }
        }
    }
}

struct ServiceGuard {
    handle: SC_HANDLE,
    binary: Vec<u16>,
    account: Vec<u16>,
    active: bool,
}
impl ServiceGuard {
    fn new(handle: SC_HANDLE, binary: Vec<u16>, account: Vec<u16>) -> Self {
        Self {
            handle,
            binary,
            account,
            active: true,
        }
    }
}
impl Drop for ServiceGuard {
    fn drop(&mut self) {
        unsafe {
            if self.active
                && service_matches_target(self.handle, &self.binary, &self.account).unwrap_or(false)
            {
                if DeleteService(self.handle) != 0 {
                    let _ = NetUserDel(ptr::null(), wide(ACCOUNT_NAME).as_ptr());
                }
            }
            CloseServiceHandle(self.handle);
        }
    }
}

/// Provision never takes over an account or service belonging to an earlier install.
pub fn require_absent() -> Result<(), String> {
    unsafe {
        let mut info = ptr::null_mut();
        let status = NetUserGetInfo(ptr::null(), wide(ACCOUNT_NAME).as_ptr(), 0, &mut info);
        if status == 0 {
            NetApiBufferFree(info as *const _);
            return Err("SETUP_ACCOUNT_CONFLICT".into());
        }
        if status != 2221 {
            return Err("ACCOUNT_QUERY_FAILED".into());
        }
        let manager = OpenSCManagerW(ptr::null(), ptr::null(), SC_MANAGER_CONNECT);
        if manager.is_null() {
            return Err("SCM_OPEN_FAILED".into());
        }
        let manager = Sc(manager);
        let service = OpenServiceW(manager.0, wide(SERVICE_NAME).as_ptr(), SERVICE_QUERY_CONFIG);
        if !service.is_null() {
            drop(Sc(service));
            return Err("SETUP_SERVICE_CONFLICT".into());
        }
        if GetLastError() != ERROR_SERVICE_DOES_NOT_EXIST {
            return Err("SCM_QUERY_FAILED".into());
        }
    }
    Ok(())
}
pub fn install() -> Result<String, String> {
    require_absent()?;
    unsafe {
        let name = wide(ACCOUNT_NAME);
        let account = wide(SERVICE_ACCOUNT);
        let mut entropy = Zeroizing::new(vec![0u8; 48]);
        if BCryptGenRandom(
            ptr::null_mut(),
            entropy.as_mut_ptr(),
            48,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        ) != 0
        {
            return Err("ACCOUNT_RANDOM_FAILED".into());
        }
        let encoded = Zeroizing::new(hex::encode(&*entropy));
        let text = Zeroizing::new(format!("Aa1!{}", encoded.as_str()));
        drop(encoded);
        let password = Zeroizing::new(wide(&text));
        let info = USER_INFO_1 {
            usri1_name: name.as_ptr() as *mut _,
            usri1_password: password.as_ptr() as *mut _,
            usri1_password_age: 0,
            usri1_priv: USER_PRIV_USER,
            usri1_home_dir: ptr::null_mut(),
            usri1_comment: ptr::null_mut(),
            usri1_flags: UF_SCRIPT
                | UF_NORMAL_ACCOUNT
                | UF_DONT_EXPIRE_PASSWD
                | UF_PASSWD_CANT_CHANGE,
            usri1_script_path: ptr::null_mut(),
        };
        let status = NetUserAdd(
            ptr::null(),
            1,
            &info as *const _ as *const u8,
            ptr::null_mut(),
        );
        if status == 2224 {
            return Err("SETUP_ACCOUNT_CONFLICT".into());
        }
        if status != 0 {
            return Err("ACCOUNT_CREATE_FAILED".into());
        }
        let mut account_guard = AccountGuard { active: true };
        let mut sid_len = 0;
        let mut domain_len = 0;
        let mut kind = 0;
        LookupAccountNameW(
            ptr::null(),
            name.as_ptr(),
            ptr::null_mut(),
            &mut sid_len,
            ptr::null_mut(),
            &mut domain_len,
            &mut kind,
        );
        if GetLastError() != ERROR_INSUFFICIENT_BUFFER || sid_len == 0 || domain_len == 0 {
            return Err("ACCOUNT_SID_FAILED".into());
        }
        let mut sid = vec![0u8; sid_len as usize];
        let mut domain = vec![0u16; domain_len as usize];
        if LookupAccountNameW(
            ptr::null(),
            name.as_ptr(),
            sid.as_mut_ptr() as *mut _,
            &mut sid_len,
            domain.as_mut_ptr(),
            &mut domain_len,
            &mut kind,
        ) == 0
        {
            return Err("ACCOUNT_SID_FAILED".into());
        }
        let mut attributes: LSA_OBJECT_ATTRIBUTES = std::mem::zeroed();
        attributes.Length = std::mem::size_of::<LSA_OBJECT_ATTRIBUTES>() as u32;
        let mut raw: LSA_HANDLE = 0;
        if LsaOpenPolicy(
            ptr::null(),
            &attributes,
            (POLICY_LOOKUP_NAMES | POLICY_CREATE_ACCOUNT) as u32,
            &mut raw,
        ) != 0
        {
            return Err("ACCOUNT_POLICY_FAILED".into());
        }
        let policy = Policy(raw);
        let names = [
            "SeServiceLogonRight",
            "SeDenyInteractiveLogonRight",
            "SeDenyRemoteInteractiveLogonRight",
            "SeDenyNetworkLogonRight",
        ];
        let mut backing: Vec<Vec<u16>> = names.iter().map(|n| wide(n)).collect();
        let rights: Vec<LSA_UNICODE_STRING> = backing
            .iter_mut()
            .map(|b| LSA_UNICODE_STRING {
                Length: ((b.len() - 1) * 2) as u16,
                MaximumLength: (b.len() * 2) as u16,
                Buffer: b.as_mut_ptr(),
            })
            .collect();
        if LsaAddAccountRights(
            policy.0,
            sid.as_mut_ptr() as *mut _,
            rights.as_ptr(),
            rights.len() as u32,
        ) != 0
        {
            return Err("ACCOUNT_RIGHTS_FAILED".into());
        }
        let mut observed = ptr::null_mut();
        let mut count = 0;
        if LsaEnumerateAccountRights(
            policy.0,
            sid.as_mut_ptr() as *mut _,
            &mut observed,
            &mut count,
        ) != 0
        {
            return Err("ACCOUNT_RIGHTS_VERIFY_FAILED".into());
        }
        let observed_names: Vec<String> = std::slice::from_raw_parts(observed, count as usize)
            .iter()
            .map(|r| {
                String::from_utf16_lossy(std::slice::from_raw_parts(
                    r.Buffer,
                    r.Length as usize / 2,
                ))
            })
            .collect();
        LsaFreeMemory(observed as *const _);
        if names.iter().any(|n| !observed_names.iter().any(|v| v == n)) {
            return Err("ACCOUNT_RIGHTS_VERIFY_FAILED".into());
        }
        let manager = OpenSCManagerW(
            ptr::null(),
            ptr::null(),
            SC_MANAGER_CONNECT | SC_MANAGER_CREATE_SERVICE,
        );
        if manager.is_null() {
            return Err("SCM_OPEN_FAILED".into());
        }
        let manager = Sc(manager);
        let service_name = wide("GNXRuntime");
        let binary = wide(&target_binary_path());
        let raw = CreateServiceW(
            manager.0,
            service_name.as_ptr(),
            service_name.as_ptr(),
            0x0001_0000 // DELETE standard access right
                | SERVICE_CHANGE_CONFIG
                | SERVICE_QUERY_CONFIG
                | SERVICE_QUERY_STATUS
                | SERVICE_START,
            SERVICE_WIN32_OWN_PROCESS,
            SERVICE_DEMAND_START,
            SERVICE_ERROR_NORMAL,
            binary.as_ptr(),
            ptr::null(),
            ptr::null_mut(),
            ptr::null(),
            account.as_ptr(),
            password.as_ptr(),
        );
        if raw.is_null() {
            return Err("SCM_CREATE_FAILED".into());
        }
        let mut service_guard = ServiceGuard::new(raw, binary.clone(), account.clone());
        account_guard.active = false;
        drop(password);
        drop(text);
        drop(entropy);
        let mut actions = [
            SC_ACTION {
                Type: SC_ACTION_RESTART,
                Delay: 5000,
            },
            SC_ACTION {
                Type: SC_ACTION_RESTART,
                Delay: 15000,
            },
            SC_ACTION {
                Type: SC_ACTION_RESTART,
                Delay: 60000,
            },
            SC_ACTION {
                Type: SC_ACTION_NONE,
                Delay: 0,
            },
        ];
        let recovery = SERVICE_FAILURE_ACTIONSW {
            dwResetPeriod: 86400,
            lpRebootMsg: ptr::null_mut(),
            lpCommand: ptr::null_mut(),
            cActions: 4,
            lpsaActions: actions.as_mut_ptr(),
        };
        if ChangeServiceConfig2W(
            service_guard.handle,
            SERVICE_CONFIG_FAILURE_ACTIONS,
            &recovery as *const _ as *const _,
        ) == 0
        {
            return Err("SCM_RECOVERY_FAILED".into());
        }
        verify_registration(service_guard.handle, &binary, &account)?;
        let mut string_sid = ptr::null_mut();
        if ConvertSidToStringSidW(sid.as_mut_ptr() as *mut _, &mut string_sid) == 0 {
            return Err("ACCOUNT_SID_FAILED".into());
        }
        let mut len = 0;
        while *string_sid.add(len) != 0 {
            len += 1
        }
        let result = String::from_utf16_lossy(std::slice::from_raw_parts(string_sid, len));
        LocalFree(string_sid as *mut _);
        service_guard.active = false;
        Ok(result)
    }
}

unsafe fn same_wide(actual: *const u16, expected: &[u16]) -> bool {
    !actual.is_null()
        && expected
            .iter()
            .enumerate()
            .all(|(i, value)| *actual.add(i) == *value)
}

unsafe fn service_matches_target(
    service: SC_HANDLE,
    binary: &[u16],
    account: &[u16],
) -> Result<bool, String> {
    let mut required = 0;
    QueryServiceConfigW(service, ptr::null_mut(), 0, &mut required);
    if required == 0 || required > 65536 {
        return Err("SCM_VERIFY_FAILED".into());
    }
    let mut buffer = vec![
        0usize;
        (required as usize + std::mem::size_of::<usize>() - 1)
            / std::mem::size_of::<usize>()
    ];
    let config = buffer.as_mut_ptr() as *mut QUERY_SERVICE_CONFIGW;
    if QueryServiceConfigW(service, config, required, &mut required) == 0 {
        return Err("SCM_VERIFY_FAILED".into());
    }
    Ok((*config).dwServiceType == SERVICE_WIN32_OWN_PROCESS
        && (*config).dwStartType == SERVICE_DEMAND_START
        && same_wide((*config).lpBinaryPathName, binary)
        && same_wide((*config).lpServiceStartName, account))
}

/// Roll back only a service whose SCM configuration proves it is this
/// versioned GNX target. An account-only partial install is removed only when
/// setup recovery supplies a durable transaction witness that account creation
/// was reached after preflight proved the account was absent.
pub fn rollback_owned_resources(account_owned: bool) -> Result<(), String> {
    unsafe {
        let manager = OpenSCManagerW(ptr::null(), ptr::null(), SC_MANAGER_CONNECT);
        if manager.is_null() {
            return Err("SCM_OPEN_FAILED".into());
        }
        let manager = Sc(manager);
        let raw = OpenServiceW(
            manager.0,
            wide(SERVICE_NAME).as_ptr(),
            0x0001_0000 | SERVICE_QUERY_CONFIG,
        );
        if raw.is_null() {
            if GetLastError() == ERROR_SERVICE_DOES_NOT_EXIST {
                if account_owned {
                    let status = NetUserDel(ptr::null(), wide(ACCOUNT_NAME).as_ptr());
                    if status != 0 && status != 2221 {
                        return Err("SETUP_ROLLBACK_FAILED".into());
                    }
                }
                return Ok(());
            }
            return Err("SCM_QUERY_FAILED".into());
        }
        let service = Sc(raw);
        let binary = wide(&target_binary_path());
        let account = wide(SERVICE_ACCOUNT);
        if !service_matches_target(service.0, &binary, &account)? {
            return Ok(());
        }
        if DeleteService(service.0) == 0 {
            return Err("SETUP_ROLLBACK_FAILED".into());
        }
        let status = NetUserDel(ptr::null(), wide(ACCOUNT_NAME).as_ptr());
        if status != 0 && status != 2221 {
            return Err("SETUP_ROLLBACK_FAILED".into());
        }
        Ok(())
    }
}

unsafe fn verify_registration(
    service: SC_HANDLE,
    binary: &[u16],
    account: &[u16],
) -> Result<(), String> {
    let mut required = 0;
    QueryServiceConfigW(service, ptr::null_mut(), 0, &mut required);
    if required == 0 || required > 65536 {
        return Err("SCM_VERIFY_FAILED".into());
    }
    let mut buffer = vec![
        0usize;
        (required as usize + std::mem::size_of::<usize>() - 1)
            / std::mem::size_of::<usize>()
    ];
    let config = buffer.as_mut_ptr() as *mut QUERY_SERVICE_CONFIGW;
    if QueryServiceConfigW(service, config, required, &mut required) == 0 {
        return Err("SCM_VERIFY_FAILED".into());
    }
    if (*config).dwServiceType != SERVICE_WIN32_OWN_PROCESS
        || (*config).dwStartType != SERVICE_DEMAND_START
        || !same_wide((*config).lpBinaryPathName, binary)
        || !same_wide((*config).lpServiceStartName, account)
    {
        return Err("SCM_VERIFY_FAILED".into());
    }
    let mut status: SERVICE_STATUS = std::mem::zeroed();
    if QueryServiceStatus(service, &mut status) == 0 || status.dwCurrentState != SERVICE_STOPPED {
        return Err("SCM_VERIFY_FAILED".into());
    }
    required = 0;
    QueryServiceConfig2W(
        service,
        SERVICE_CONFIG_FAILURE_ACTIONS,
        ptr::null_mut(),
        0,
        &mut required,
    );
    if required == 0 || required > 65536 {
        return Err("SCM_RECOVERY_VERIFY_FAILED".into());
    }
    let mut buffer = vec![
        0usize;
        (required as usize + std::mem::size_of::<usize>() - 1)
            / std::mem::size_of::<usize>()
    ];
    if QueryServiceConfig2W(
        service,
        SERVICE_CONFIG_FAILURE_ACTIONS,
        buffer.as_mut_ptr() as *mut u8,
        required,
        &mut required,
    ) == 0
    {
        return Err("SCM_RECOVERY_VERIFY_FAILED".into());
    }
    let recovery = &*(buffer.as_ptr() as *const SERVICE_FAILURE_ACTIONSW);
    if recovery.dwResetPeriod != 86400 || recovery.cActions != 4 || recovery.lpsaActions.is_null() {
        return Err("SCM_RECOVERY_VERIFY_FAILED".into());
    }
    let actions = std::slice::from_raw_parts(recovery.lpsaActions, 4);
    if actions[0].Type != SC_ACTION_RESTART
        || actions[0].Delay != 5000
        || actions[1].Type != SC_ACTION_RESTART
        || actions[1].Delay != 15000
        || actions[2].Type != SC_ACTION_RESTART
        || actions[2].Delay != 60000
        || actions[3].Type != SC_ACTION_NONE
    {
        return Err("SCM_RECOVERY_VERIFY_FAILED".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::target_binary_path;

    #[test]
    fn ownership_witness_is_versioned_and_exact() {
        assert_eq!(
            target_binary_path(),
            r#""C:\Program Files\GNX-0.3.1\gnx-service.exe""#
        );
        assert_ne!(
            target_binary_path(),
            r#""C:\Program Files\GNX\gnx-service.exe""#
        );
    }
}
