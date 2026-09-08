use std::ptr;
use windows_sys::Win32::{
    Foundation::LocalFree,
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
pub const PRIVATE_ROOT: &str = "C:\\ProgramData\\GNX";
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
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
pub fn install() -> Result<String, String> {
    unsafe {
        let name = wide("gnx-runtime");
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
        let text = Zeroizing::new(format!("Aa1!{}", hex::encode(&*entropy)));
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
            let info = USER_INFO_1003 {
                usri1003_password: password.as_ptr() as *mut _,
            };
            if NetUserSetInfo(
                ptr::null(),
                name.as_ptr(),
                1003,
                &info as *const _ as *const u8,
                ptr::null_mut(),
            ) != 0
            {
                return Err("ACCOUNT_RECONCILE_FAILED".into());
            }
        } else if status != 0 {
            return Err("ACCOUNT_CREATE_FAILED".into());
        }
        let mut sid_len = 0;
        let mut domain_len = 0;
        let mut kind = 0;
        LookupAccountNameW(
            ptr::null(),
            account.as_ptr(),
            ptr::null_mut(),
            &mut sid_len,
            ptr::null_mut(),
            &mut domain_len,
            &mut kind,
        );
        let mut sid = vec![0u8; sid_len as usize];
        let mut domain = vec![0u16; domain_len as usize];
        if LookupAccountNameW(
            ptr::null(),
            account.as_ptr(),
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
        let binary = wide("\"C:\\Program Files\\GNX\\gnx-service.exe\"");
        let existing = OpenServiceW(
            manager.0,
            service_name.as_ptr(),
            SERVICE_CHANGE_CONFIG | SERVICE_QUERY_CONFIG,
        );
        let service = if existing.is_null() {
            let raw = CreateServiceW(
                manager.0,
                service_name.as_ptr(),
                service_name.as_ptr(),
                SERVICE_CHANGE_CONFIG | SERVICE_QUERY_CONFIG,
                SERVICE_WIN32_OWN_PROCESS,
                SERVICE_AUTO_START,
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
            Sc(raw)
        } else {
            let service = Sc(existing);
            if ChangeServiceConfigW(
                service.0,
                SERVICE_WIN32_OWN_PROCESS,
                SERVICE_AUTO_START,
                SERVICE_ERROR_NORMAL,
                binary.as_ptr(),
                ptr::null(),
                ptr::null_mut(),
                ptr::null(),
                account.as_ptr(),
                password.as_ptr(),
                ptr::null(),
            ) == 0
            {
                return Err("SCM_UPDATE_FAILED".into());
            }
            service
        };
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
        ];
        let recovery = SERVICE_FAILURE_ACTIONSW {
            dwResetPeriod: 86400,
            lpRebootMsg: ptr::null_mut(),
            lpCommand: ptr::null_mut(),
            cActions: 3,
            lpsaActions: actions.as_mut_ptr(),
        };
        if ChangeServiceConfig2W(
            service.0,
            SERVICE_CONFIG_FAILURE_ACTIONS,
            &recovery as *const _ as *const _,
        ) == 0
        {
            return Err("SCM_RECOVERY_FAILED".into());
        }
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
        Ok(result)
    }
}
