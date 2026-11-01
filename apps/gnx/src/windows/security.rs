use std::{
    fs::{File, OpenOptions},
    io::{self, Read},
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::{Path, PathBuf},
    ptr::null_mut,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, LocalFree},
    Security::{
        ACCESS_ALLOWED_ACE, ACL,
        Authorization::{ConvertSidToStringSidW, GetSecurityInfo, SE_FILE_OBJECT},
        CheckTokenMembership, CreateWellKnownSid, DACL_SECURITY_INFORMATION, GetAce,
        GetTokenInformation, IsValidSid, OWNER_SECURITY_INFORMATION, TOKEN_QUERY, TOKEN_USER,
        TokenUser, WinBuiltinAdministratorsSid,
    },
    Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
        GetFileInformationByHandle,
    },
    System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    },
};

pub struct Handle(pub HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

pub fn denied(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}

pub fn state_dir() -> io::Result<PathBuf> {
    let root = std::env::var_os("ProgramData")
        .ok_or_else(|| io::Error::other("ProgramData no disponible"))?;
    Ok(PathBuf::from(root).join("GnX Core"))
}

pub fn random(bytes: &mut [u8]) -> io::Result<()> {
    use windows_sys::Win32::Security::Cryptography::{
        BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom,
    };
    if unsafe {
        BCryptGenRandom(
            null_mut(),
            bytes.as_mut_ptr(),
            bytes.len() as u32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    } < 0
    {
        return Err(io::Error::other("generador aleatorio no disponible"));
    }
    Ok(())
}

pub fn sid_text(sid: *mut std::ffi::c_void) -> io::Result<String> {
    let mut text = null_mut();
    if unsafe { IsValidSid(sid) } == 0 || unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 {
        return Err(denied("SID inválido"));
    }
    let mut len = 0;
    unsafe {
        while *text.add(len) != 0 {
            len += 1;
        }
    }
    let result = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, len) });
    unsafe { LocalFree(text.cast()) };
    Ok(result)
}

pub fn token_sid(token: HANDLE) -> io::Result<String> {
    let mut needed = 0;
    unsafe { GetTokenInformation(token, TokenUser, null_mut(), 0, &mut needed) };
    if needed == 0 || needed > 65536 {
        return Err(denied("token inválido"));
    }
    // TOKEN_USER requires pointer alignment; a u64 backing allocation supplies it.
    let mut data = vec![0u64; (needed as usize).div_ceil(8)];
    if unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            data.as_mut_ptr().cast(),
            needed,
            &mut needed,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    sid_text(unsafe { (*(data.as_ptr().cast::<TOKEN_USER>())).User.Sid })
}

pub fn process_token(process: HANDLE) -> io::Result<Handle> {
    let mut token = null_mut();
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Handle(token))
}

pub fn require_system() -> io::Result<()> {
    let token = process_token(unsafe { GetCurrentProcess() })?;
    if token_sid(token.0)? != "S-1-5-18" {
        return Err(denied("el servicio requiere LocalSystem"));
    }
    Ok(())
}

pub fn require_unprivileged(token: HANDLE) -> io::Result<()> {
    use windows_sys::Win32::Security::{TOKEN_GROUPS, TokenGroups};
    let mut needed = 0;
    unsafe { GetTokenInformation(token, TokenGroups, null_mut(), 0, &mut needed) };
    if needed == 0 || needed > 65536 {
        return Err(denied("grupos de identidad no verificables"));
    }
    let mut data = vec![0u64; (needed as usize).div_ceil(8)];
    if unsafe {
        GetTokenInformation(
            token,
            TokenGroups,
            data.as_mut_ptr().cast(),
            needed,
            &mut needed,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let groups = unsafe { &*data.as_ptr().cast::<TOKEN_GROUPS>() };
    let entries =
        unsafe { std::slice::from_raw_parts(groups.Groups.as_ptr(), groups.GroupCount as usize) };
    // Reject even a deny-only Administrators SID: a dedicated account must not
    // become a privileged identity merely because its current token is filtered.
    for entry in entries {
        if sid_text(entry.Sid)? == "S-1-5-32-544" {
            return Err(denied("la identidad dedicada no puede ser administradora"));
        }
    }
    Ok(())
}

pub fn process_is_system(pid: u32) -> io::Result<()> {
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return Err(denied("servidor no verificable"));
    }
    let process = Handle(process);
    let token = process_token(process.0)?;
    if token_sid(token.0)? != "S-1-5-18" {
        return Err(denied("servidor no es LocalSystem"));
    }
    Ok(())
}

pub fn effective_admin() -> io::Result<()> {
    let mut sid = [0u64; 16];
    let mut len = std::mem::size_of_val(&sid) as u32;
    if unsafe {
        CreateWellKnownSid(
            WinBuiltinAdministratorsSid,
            null_mut(),
            sid.as_mut_ptr().cast(),
            &mut len,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let mut member = 0;
    if unsafe { CheckTokenMembership(null_mut(), sid.as_mut_ptr().cast(), &mut member) } == 0
        || member == 0
    {
        return Err(denied("se requiere administrador elevado"));
    }
    Ok(())
}

// Replacing a failed force transaction is a new explicit destructive request,
// never an automatic retry while an earlier product process is still alive.
pub(super) fn require_idle_maintenance(
    binary: &Path,
    helper: &Path,
    main_pid: Option<u32>,
) -> io::Result<()> {
    use windows_sys::Win32::{
        Foundation::{
            ERROR_INVALID_PARAMETER, ERROR_NO_MORE_FILES, GetLastError, INVALID_HANDLE_VALUE,
        },
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
                TH32CS_SNAPPROCESS,
            },
            Threading::QueryFullProcessImageNameW,
        },
    };
    if let Some(pid) = main_pid {
        process_is_system(pid)?;
    }
    let binary = std::fs::canonicalize(binary)?;
    let helper = std::fs::canonicalize(helper)?;
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let snapshot = Handle(snapshot);
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    let mut available = unsafe { Process32FirstW(snapshot.0, &mut entry) };
    let mut last_error = if available == 0 {
        unsafe { GetLastError() }
    } else {
        0
    };
    while available != 0 {
        let name = String::from_utf16_lossy(
            &entry.szExeFile[..entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len())],
        );
        if entry.th32ProcessID != std::process::id()
            && Some(entry.th32ProcessID) != main_pid
            && ["gnx.exe", "product-update.exe"].contains(&name.to_ascii_lowercase().as_str())
        {
            let handle =
                unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, entry.th32ProcessID) };
            if handle.is_null() {
                if unsafe { GetLastError() } != ERROR_INVALID_PARAMETER {
                    return Err(denied(
                        "no se puede confirmar la salida de un proceso previo",
                    ));
                }
            } else {
                let handle = Handle(handle);
                let mut path = [0u16; 32768];
                let mut size = path.len() as u32;
                if unsafe { QueryFullProcessImageNameW(handle.0, 0, path.as_mut_ptr(), &mut size) }
                    == 0
                {
                    return Err(io::Error::last_os_error());
                }
                let path = std::fs::canonicalize(
                    String::from_utf16(&path[..size as usize])
                        .map_err(|_| denied("ruta de proceso inválida"))?,
                )?;
                if path == binary || path == helper {
                    return Err(denied(
                        "hay un proceso de mantenimiento previo vivo; no se repite la retirada",
                    ));
                }
            }
        }
        available = unsafe { Process32NextW(snapshot.0, &mut entry) };
        last_error = if available == 0 {
            unsafe { GetLastError() }
        } else {
            0
        };
    }
    if last_error != ERROR_NO_MORE_FILES {
        return Err(io::Error::from_raw_os_error(last_error as i32));
    }
    Ok(())
}

fn check_acl(file: &File) -> io::Result<()> {
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(file.as_raw_handle().cast(), &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(denied("no se admiten enlaces en el estado"));
    }
    let mut owner = null_mut();
    let mut dacl: *mut ACL = null_mut();
    let mut descriptor = null_mut();
    let code = unsafe {
        GetSecurityInfo(
            file.as_raw_handle().cast(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            &mut dacl,
            null_mut(),
            &mut descriptor,
        )
    };
    if code != 0 {
        return Err(io::Error::from_raw_os_error(code as i32));
    }
    let result = (|| {
        let allowed = |sid| -> io::Result<bool> {
            Ok(matches!(
                sid_text(sid)?.as_str(),
                "S-1-5-18" | "S-1-5-32-544"
            ))
        };
        if dacl.is_null() || !allowed(owner)? {
            return Err(denied("propiedad o ACL del estado inválida"));
        }
        for index in 0..unsafe { (*dacl).AceCount } as u32 {
            let mut ace = null_mut();
            if unsafe { GetAce(dacl, index, &mut ace) } == 0 {
                return Err(denied("ACL no verificable"));
            }
            let header = unsafe { &*ace.cast::<windows_sys::Win32::Security::ACE_HEADER>() };
            match header.AceType {
                0 => {
                    if (header.AceSize as usize) < std::mem::size_of::<ACCESS_ALLOWED_ACE>() {
                        return Err(denied("ACE inválida"));
                    }
                    let allowed_ace = unsafe { &*ace.cast::<ACCESS_ALLOWED_ACE>() };
                    if !allowed(std::ptr::addr_of!(allowed_ace.SidStart).cast_mut().cast())? {
                        return Err(denied("el estado concede acceso a otro usuario"));
                    }
                }
                1 => {} // A deny ACE cannot grant another principal access.
                _ => return Err(denied("tipo de ACL no admitido")),
            }
        }
        Ok(())
    })();
    unsafe { LocalFree(descriptor) };
    result
}

fn open_checked(path: &Path, directory: bool) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | if directory { FILE_SHARE_WRITE } else { 0 })
        .custom_flags(
            FILE_FLAG_OPEN_REPARSE_POINT
                | if directory {
                    FILE_FLAG_BACKUP_SEMANTICS
                } else {
                    0
                },
        )
        .open(path)?;
    check_acl(&file)?;
    Ok(file)
}

pub fn private_file(name: &str) -> io::Result<File> {
    // Callers supply fixed source-code names only. Keep the checked parent open
    // without delete sharing while opening the child, preventing rename races.
    let root = state_dir()?;
    let _parent = open_checked(&root, true)?;
    open_checked(&root.join(name), false)
}

pub fn private_directory() -> io::Result<File> {
    open_checked(&state_dir()?, true)
}

pub fn enable_identity_privileges() -> io::Result<()> {
    use windows_sys::Win32::{
        Foundation::{ERROR_NOT_ALL_ASSIGNED, GetLastError, LUID},
        Security::{
            AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW,
            SE_ASSIGNPRIMARYTOKEN_NAME, SE_BACKUP_NAME, SE_INCREASE_QUOTA_NAME,
            SE_PRIVILEGE_ENABLED, SE_RESTORE_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
        },
    };
    let mut token = null_mut();
    if unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_QUERY | TOKEN_ADJUST_PRIVILEGES,
            &mut token,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let token = Handle(token);
    for name in [
        SE_ASSIGNPRIMARYTOKEN_NAME,
        SE_INCREASE_QUOTA_NAME,
        SE_BACKUP_NAME,
        SE_RESTORE_NAME,
    ] {
        let mut luid: LUID = unsafe { std::mem::zeroed() };
        if unsafe { LookupPrivilegeValueW(std::ptr::null(), name, &mut luid) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let privilege = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        if unsafe { AdjustTokenPrivileges(token.0, 0, &privilege, 0, null_mut(), null_mut()) } == 0
            || unsafe { GetLastError() } == ERROR_NOT_ALL_ASSIGNED
        {
            return Err(denied("privilegios de identidad no disponibles"));
        }
    }
    Ok(())
}

pub fn read_private(name: &str, max: u64) -> io::Result<Vec<u8>> {
    let file = private_file(name)?;
    if file.metadata()?.len() > max {
        return Err(crate::protocol::invalid("estado fuera de límite"));
    }
    let mut bytes = Vec::new();
    file.take(max + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        return Err(crate::protocol::invalid("estado fuera de límite"));
    }
    Ok(bytes)
}
