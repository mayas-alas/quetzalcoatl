use super::security::{self, Handle, denied};
use crate::protocol::{ACCOUNT, DISTRO, invalid};
use serde::Deserialize;
use std::{
    io,
    ptr::{null, null_mut},
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{GetLastError, HANDLE, LocalFree, WAIT_OBJECT_0, WAIT_TIMEOUT},
    Security::{
        Cryptography::{CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptUnprotectData},
        LOGON32_LOGON_INTERACTIVE, LOGON32_PROVIDER_DEFAULT, LogonUserW, SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::{
        CreateFileW, FILE_GENERIC_READ, FILE_SHARE_READ, OPEN_EXISTING, ReadFile,
    },
    System::{
        Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock},
        Pipes::{CreatePipe, PeekNamedPipe},
        Registry::{HKEY_USERS, KEY_READ, RRF_RT_REG_SZ, RegCloseKey, RegGetValueW, RegOpenKeyExW},
        Threading::{
            CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT, CreateProcessAsUserW,
            DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess,
            InitializeProcThreadAttributeList, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
            PROCESS_INFORMATION, STARTF_USESTDHANDLES, STARTUPINFOEXW, TerminateProcess,
            UpdateProcThreadAttribute, WaitForSingleObject,
        },
    },
    UI::Shell::{GetUserProfileDirectoryW, LoadUserProfileW, PROFILEINFOW, UnloadUserProfile},
};
use zeroize::Zeroizing;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ownership {
    pub version: u16,
    pub sid: String,
    pub distribution_id: String,
}

impl Ownership {
    pub fn read() -> io::Result<Self> {
        if security::read_private("prepared", 16)? != b"1" {
            return Err(invalid("preparación incompleta"));
        }
        let bytes = security::read_private("identity.json", 4096)?;
        let value: Self = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("registro de propiedad inválido"))?;
        if value.version != 1 || !valid_sid(&value.sid) || !valid_guid(&value.distribution_id) {
            return Err(invalid("registro de propiedad inválido"));
        }
        Ok(value)
    }
}

fn valid_sid(value: &str) -> bool {
    value.starts_with("S-1-5-21-")
        && value.len() < 128
        && value[2..].bytes().all(|b| b.is_ascii_digit() || b == b'-')
}

fn valid_guid(value: &str) -> bool {
    value.len() == 38
        && value.starts_with('{')
        && value.ends_with('}')
        && value[1..37].bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

pub struct Identity {
    token: Handle,
    profile: HANDLE,
    environment: *mut std::ffi::c_void,
    directory: Vec<u16>,
}

impl Drop for Identity {
    fn drop(&mut self) {
        unsafe {
            if !self.environment.is_null() {
                DestroyEnvironmentBlock(self.environment);
            }
            if !self.profile.is_null() {
                UnloadUserProfile(self.token.0, self.profile);
            }
        }
    }
}

impl Identity {
    pub fn open(ownership: &Ownership) -> io::Result<Self> {
        Self::open_inner(ownership, false)
    }

    pub fn for_setup(sid: &str) -> io::Result<Self> {
        Self::open_inner(
            &Ownership {
                version: 1,
                sid: sid.into(),
                distribution_id: String::new(),
            },
            true,
        )
    }

    fn open_inner(ownership: &Ownership, setup: bool) -> io::Result<Self> {
        let mut encrypted = Zeroizing::new(security::read_private("account.dpapi", 65536)?);
        let input = CRYPT_INTEGER_BLOB {
            cbData: encrypted.len() as u32,
            pbData: encrypted.as_mut_ptr(),
        };
        let mut output: CRYPT_INTEGER_BLOB = unsafe { std::mem::zeroed() };
        if unsafe {
            CryptUnprotectData(
                &input,
                null_mut(),
                null(),
                null(),
                null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } == 0
        {
            return Err(denied("identidad no disponible"));
        }
        if output.pbData.is_null() || output.cbData == 0 || output.cbData > 65536 {
            if !output.pbData.is_null() {
                unsafe { LocalFree(output.pbData.cast()) };
            }
            return Err(denied("credencial inválida"));
        }
        let plain =
            unsafe { std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize) };
        let decoded = std::str::from_utf8(plain)
            .ok()
            .filter(|s| !s.is_empty() && !s.contains('\0'))
            .map(|s| Zeroizing::new(wide(s)));
        plain.fill(0);
        unsafe { LocalFree(output.pbData.cast()) };
        let password = decoded.ok_or_else(|| denied("credencial inválida"))?;
        let mut token = null_mut();
        if unsafe {
            LogonUserW(
                wide(ACCOUNT).as_ptr(),
                wide(".").as_ptr(),
                password.as_ptr(),
                LOGON32_LOGON_INTERACTIVE,
                LOGON32_PROVIDER_DEFAULT,
                &mut token,
            )
        } == 0
        {
            return Err(denied("no se pudo abrir la identidad dedicada"));
        }
        drop(password);
        let token = Handle(token);
        if security::token_sid(token.0)? != ownership.sid {
            return Err(denied("la cuenta no coincide con el SID propio"));
        }
        security::require_unprivileged(token.0)?;
        // Reading status may load an existing profile hive, but must never
        // provision a missing profile as an accidental side effect.
        if !setup {
            let mut directory_size = 0;
            unsafe { GetUserProfileDirectoryW(token.0, null_mut(), &mut directory_size) };
            if directory_size == 0 || directory_size > 32768 {
                return Err(denied("perfil dedicado ausente"));
            }
            let mut directory = vec![0; directory_size as usize];
            if unsafe {
                GetUserProfileDirectoryW(token.0, directory.as_mut_ptr(), &mut directory_size)
            } == 0
            {
                return Err(denied("perfil dedicado ausente"));
            }
            let end = directory
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(directory.len());
            let path = std::path::PathBuf::from(String::from_utf16_lossy(&directory[..end]));
            use std::os::windows::fs::MetadataExt;
            if let Ok(metadata) = std::fs::symlink_metadata(&path) {
                if !metadata.is_dir()
                    || metadata.file_attributes()
                        & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
                        != 0
                {
                    return Err(denied("perfil dedicado inválido"));
                }
            } else {
                return Err(denied("perfil dedicado ausente"));
            }
        }
        let mut username = wide(ACCOUNT);
        let mut profile: PROFILEINFOW = unsafe { std::mem::zeroed() };
        profile.dwSize = std::mem::size_of::<PROFILEINFOW>() as u32;
        profile.dwFlags = 1; // PI_NOUI
        profile.lpUserName = username.as_mut_ptr();
        if unsafe { LoadUserProfileW(token.0, &mut profile) } == 0 {
            return Err(denied("perfil dedicado no disponible"));
        }
        let mut identity = Self {
            token,
            profile: profile.hProfile,
            environment: null_mut(),
            directory: Vec::new(),
        };
        if unsafe { CreateEnvironmentBlock(&mut identity.environment, identity.token.0, 0) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut size = 0;
        unsafe { GetUserProfileDirectoryW(identity.token.0, null_mut(), &mut size) };
        if size == 0 || size > 32768 {
            return Err(denied("perfil inválido"));
        }
        identity.directory = vec![0; size as usize];
        if unsafe {
            GetUserProfileDirectoryW(identity.token.0, identity.directory.as_mut_ptr(), &mut size)
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if !setup {
            identity.verify_registration(ownership)?;
        }
        Ok(identity)
    }

    pub fn profile_dir(&self) -> std::path::PathBuf {
        let end = self
            .directory
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(self.directory.len());
        std::path::PathBuf::from(String::from_utf16_lossy(&self.directory[..end]))
    }

    fn verify_registration(&self, ownership: &Ownership) -> io::Result<()> {
        let key = wide(&format!(
            r"{}\Software\Microsoft\Windows\CurrentVersion\Lxss\{}",
            ownership.sid, ownership.distribution_id
        ));
        let mut handle = null_mut();
        if unsafe { RegOpenKeyExW(HKEY_USERS, key.as_ptr(), 0, KEY_READ, &mut handle) } != 0 {
            return Err(denied("registro WSL propio no disponible"));
        }
        let mut data = [0u16; 256];
        let mut size = std::mem::size_of_val(&data) as u32;
        let code = unsafe {
            RegGetValueW(
                handle,
                null(),
                wide("DistributionName").as_ptr(),
                RRF_RT_REG_SZ,
                null_mut(),
                data.as_mut_ptr().cast(),
                &mut size,
            )
        };
        unsafe { RegCloseKey(handle) };
        let length = data.iter().position(|&n| n == 0).unwrap_or(data.len());
        if code != 0 || String::from_utf16_lossy(&data[..length]) != DISTRO {
            return Err(denied("la distro no coincide con su registro propio"));
        }
        Ok(())
    }

    pub fn run(&self, args: &[&str], limit: Duration) -> io::Result<Output> {
        let security = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: null_mut(),
            bInheritHandle: 1,
        };
        let mut read = null_mut();
        let mut write = null_mut();
        if unsafe { CreatePipe(&mut read, &mut write, &security, 65536) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let read = Handle(read);
        let write = Handle(write);
        let nul = unsafe {
            CreateFileW(
                wide("NUL").as_ptr(),
                FILE_GENERIC_READ,
                FILE_SHARE_READ,
                &security,
                OPEN_EXISTING,
                0,
                null_mut(),
            )
        };
        if nul == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let nul = Handle(nul);
        let mut size = 0;
        unsafe { InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut size) };
        if size == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut storage = vec![0u64; size.div_ceil(8)];
        let attributes = storage.as_mut_ptr().cast();
        if unsafe { InitializeProcThreadAttributeList(attributes, 1, 0, &mut size) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let handles = [write.0, nul.0];
        let updated = unsafe {
            UpdateProcThreadAttribute(
                attributes,
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                handles.as_ptr().cast(),
                std::mem::size_of_val(&handles),
                null_mut(),
                null(),
            )
        };
        if updated == 0 {
            unsafe { DeleteProcThreadAttributeList(attributes) };
            return Err(io::Error::last_os_error());
        }
        let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
        startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = nul.0;
        startup.StartupInfo.hStdOutput = write.0;
        startup.StartupInfo.hStdError = write.0;
        startup.lpAttributeList = attributes;
        let system = std::env::var("SystemRoot")
            .map_err(|_| io::Error::other("SystemRoot no disponible"))?;
        let executable = format!(r"{system}\System32\wsl.exe");
        let mut command = quote(&executable);
        for arg in args {
            command.push(' ');
            command.push_str(&quote(arg));
        }
        let mut command = wide(&command);
        let mut process: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
        let created = unsafe {
            CreateProcessAsUserW(
                self.token.0,
                wide(&executable).as_ptr(),
                command.as_mut_ptr(),
                null(),
                null(),
                1,
                CREATE_NO_WINDOW | CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT,
                self.environment,
                self.directory.as_ptr(),
                &startup.StartupInfo,
                &mut process,
            )
        };
        let error = unsafe { GetLastError() };
        unsafe { DeleteProcThreadAttributeList(attributes) };
        if created == 0 {
            return Err(io::Error::from_raw_os_error(error as i32));
        }
        drop(write);
        let process_handle = Handle(process.hProcess);
        let _thread = Handle(process.hThread);
        let start = Instant::now();
        let mut bytes = Vec::new();
        let mut overflow = false;
        loop {
            if start.elapsed() >= limit {
                // Kill only this client; the systemd transaction may continue.
                unsafe { TerminateProcess(process_handle.0, 1) };
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "resultado nativo desconocido",
                ));
            }
            let mut available = 0;
            if unsafe {
                PeekNamedPipe(
                    read.0,
                    null_mut(),
                    0,
                    null_mut(),
                    &mut available,
                    null_mut(),
                )
            } != 0
                && available > 0
            {
                let mut buffer = [0u8; 4096];
                let mut count = 0;
                if unsafe {
                    ReadFile(
                        read.0,
                        buffer.as_mut_ptr(),
                        (available as usize).min(buffer.len()) as u32,
                        &mut count,
                        null_mut(),
                    )
                } != 0
                {
                    if bytes.len() + count as usize <= 16384 {
                        bytes.extend_from_slice(&buffer[..count as usize]);
                    } else {
                        overflow = true;
                    }
                    continue;
                }
            }
            match unsafe { WaitForSingleObject(process_handle.0, 20) } {
                WAIT_OBJECT_0 => {
                    // Final bytes remain in the pipe after process exit.
                    let mut remaining = 0;
                    if unsafe {
                        PeekNamedPipe(
                            read.0,
                            null_mut(),
                            0,
                            null_mut(),
                            &mut remaining,
                            null_mut(),
                        )
                    } != 0
                        && remaining > 0
                    {
                        continue;
                    }
                    break;
                }
                WAIT_TIMEOUT => {}
                _ => return Err(io::Error::last_os_error()),
            }
        }
        if overflow {
            return Err(invalid("respuesta nativa fuera de límite"));
        }
        let mut code = 1;
        if unsafe { GetExitCodeProcess(process_handle.0, &mut code) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Output { code, bytes })
    }
}

pub struct Output {
    pub code: u32,
    pub bytes: Vec<u8>,
}
impl Output {
    pub fn text(&self) -> String {
        if self.bytes.contains(&0) {
            let units: Vec<_> = self
                .bytes
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect();
            String::from_utf16_lossy(&units)
                .trim_start_matches('\u{feff}')
                .to_owned()
        } else {
            String::from_utf8_lossy(&self.bytes).into_owned()
        }
    }
}

fn quote(arg: &str) -> String {
    let mut output = String::from("\"");
    let mut slashes = 0;
    for c in arg.chars() {
        if c == '\\' {
            slashes += 1;
            continue;
        }
        output.extend(std::iter::repeat_n(
            '\\',
            if c == '"' { slashes * 2 + 1 } else { slashes },
        ));
        output.push(c);
        slashes = 0;
    }
    output.extend(std::iter::repeat_n('\\', slashes * 2));
    output.push('"');
    output
}
