use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{FromRawHandle, RawHandle};
use std::ptr::null_mut;
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{ERROR_PIPE_CONNECTED, GetLastError, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows_sys::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX;
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_TYPE_BYTE, PIPE_WAIT,
};
use zeroize::Zeroizing;

use crate::{Error, Result};

const PIPE_PATH: &str = r"\\.\pipe\GNX";
const MAGIC: &[u8; 4] = b"GNX1";
const MAX_CONFIG: usize = 1024 * 1024;
const MAX_SECRET: usize = 64 * 1024;
const MAX_RESPONSE: usize = 4 * 1024 * 1024;

pub struct BrokerResponse {
    pub exit_code: u8,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub fn request(action: [&str; 2], config: &[u8], secret: Option<&[u8]>) -> Result<BrokerResponse> {
    let code = action_code(action).ok_or(Error::Operation("BROKER_ACTION"))?;
    exchange(code, config, secret.unwrap_or_default())
}

pub fn ping() -> Result<()> {
    let response = exchange(0, &[], &[])?;
    if response.exit_code == 0 && response.stdout == b"PONG\n" {
        Ok(())
    } else {
        Err(Error::Operation("BROKER_PING"))
    }
}

pub fn serve() -> Result<()> {
    let security = PipeSecurity::new()?;
    loop {
        let mut pipe = create_pipe(&security.attrs)?;
        // SAFETY: pipe is a valid named-pipe handle and synchronous connection is intentional.
        let connected = unsafe { ConnectNamedPipe(pipe.raw_handle(), null_mut()) };
        if connected == 0 {
            // SAFETY: GetLastError has no preconditions.
            let error = unsafe { GetLastError() };
            if error != ERROR_PIPE_CONNECTED {
                return Err(Error::Operation("BROKER_CONNECT"));
            }
        }
        if let Err(error) = handle(&mut pipe) {
            crate::windows::runtime::write_failure(error.label());
        }
    }
}

fn handle(pipe: &mut PipeFile) -> Result<()> {
    let mut magic = [0_u8; 4];
    pipe.file.read_exact(&mut magic).map_err(Error::Spawn)?;
    if &magic != MAGIC {
        return Err(Error::Operation("BROKER_PROTOCOL"));
    }
    let code = read_u8(&mut pipe.file)?;
    let config_len = read_u32(&mut pipe.file)? as usize;
    let secret_len = read_u32(&mut pipe.file)? as usize;
    if config_len > MAX_CONFIG || secret_len > MAX_SECRET {
        return Err(Error::Operation("BROKER_FRAME_SIZE"));
    }
    let mut config = vec![0_u8; config_len];
    pipe.file.read_exact(&mut config).map_err(Error::Spawn)?;
    let mut secret = Zeroizing::new(vec![0_u8; secret_len]);
    pipe.file.read_exact(&mut secret).map_err(Error::Spawn)?;

    let response = if code == 0 {
        BrokerResponse {
            exit_code: 0,
            stdout: b"PONG\n".to_vec(),
            stderr: Vec::new(),
        }
    } else {
        let action = code_action(code).ok_or(Error::Operation("BROKER_ACTION"))?;
        crate::windows::runtime::sync_config(&config)?;
        let output = crate::windows::runtime::run_action(
            action,
            if secret.is_empty() { None } else { Some(secret.as_slice()) },
        )?;
        if output.stdout.len() > MAX_RESPONSE || output.stderr.len() > MAX_RESPONSE {
            return Err(Error::Operation("BROKER_RESPONSE_SIZE"));
        }
        BrokerResponse {
            exit_code: output.status.code().unwrap_or(6).clamp(0, 255) as u8,
            stdout: output.stdout,
            stderr: output.stderr,
        }
    };
    write_response(&mut pipe.file, &response)
}

fn exchange(code: u8, config: &[u8], secret: &[u8]) -> Result<BrokerResponse> {
    if config.len() > MAX_CONFIG || secret.len() > MAX_SECRET {
        return Err(Error::Operation("BROKER_FRAME_SIZE"));
    }
    let mut pipe = open_pipe()?;
    pipe.write_all(MAGIC).map_err(Error::Spawn)?;
    pipe.write_all(&[code]).map_err(Error::Spawn)?;
    pipe.write_all(&(config.len() as u32).to_le_bytes())
        .map_err(Error::Spawn)?;
    pipe.write_all(&(secret.len() as u32).to_le_bytes())
        .map_err(Error::Spawn)?;
    pipe.write_all(config).map_err(Error::Spawn)?;
    pipe.write_all(secret).map_err(Error::Spawn)?;
    pipe.flush().map_err(Error::Spawn)?;

    let mut magic = [0_u8; 4];
    pipe.read_exact(&mut magic).map_err(Error::Spawn)?;
    if &magic != MAGIC {
        return Err(Error::Operation("BROKER_PROTOCOL"));
    }
    let exit_code = read_u8(&mut pipe)?;
    let stdout_len = read_u32(&mut pipe)? as usize;
    let stderr_len = read_u32(&mut pipe)? as usize;
    if stdout_len > MAX_RESPONSE || stderr_len > MAX_RESPONSE {
        return Err(Error::Operation("BROKER_RESPONSE_SIZE"));
    }
    let mut stdout = vec![0_u8; stdout_len];
    let mut stderr = vec![0_u8; stderr_len];
    pipe.read_exact(&mut stdout).map_err(Error::Spawn)?;
    pipe.read_exact(&mut stderr).map_err(Error::Spawn)?;
    Ok(BrokerResponse {
        exit_code,
        stdout,
        stderr,
    })
}

fn write_response(file: &mut File, response: &BrokerResponse) -> Result<()> {
    file.write_all(MAGIC).map_err(Error::Spawn)?;
    file.write_all(&[response.exit_code]).map_err(Error::Spawn)?;
    file.write_all(&(response.stdout.len() as u32).to_le_bytes())
        .map_err(Error::Spawn)?;
    file.write_all(&(response.stderr.len() as u32).to_le_bytes())
        .map_err(Error::Spawn)?;
    file.write_all(&response.stdout).map_err(Error::Spawn)?;
    file.write_all(&response.stderr).map_err(Error::Spawn)?;
    file.flush().map_err(Error::Spawn)
}

fn open_pipe() -> Result<File> {
    let started = Instant::now();
    loop {
        match OpenOptions::new().read(true).write(true).open(PIPE_PATH) {
            Ok(pipe) => return Ok(pipe),
            Err(_) if started.elapsed() < Duration::from_secs(10) => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(error) => return Err(Error::Spawn(error)),
        }
    }
}

struct PipeFile {
    file: File,
}

impl PipeFile {
    fn raw_handle(&self) -> windows_sys::Win32::Foundation::HANDLE {
        use std::os::windows::io::AsRawHandle;
        self.file.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE
    }
}

fn create_pipe(attrs: &SECURITY_ATTRIBUTES) -> Result<PipeFile> {
    let name = wide(PIPE_PATH);
    // SAFETY: name and security attributes are valid for the duration of the call.
    let handle = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            4,
            64 * 1024,
            64 * 1024,
            0,
            attrs,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(Error::Operation("BROKER_PIPE"));
    }
    // SAFETY: ownership of the freshly-created HANDLE is transferred to File exactly once.
    let file = unsafe { File::from_raw_handle(handle as RawHandle) };
    Ok(PipeFile { file })
}

struct PipeSecurity {
    attrs: SECURITY_ATTRIBUTES,
    _descriptor: PSECURITY_DESCRIPTOR,
}

impl PipeSecurity {
    fn new() -> Result<Self> {
        let sid = std::fs::read_to_string(crate::windows::runtime::data_path("operator.sid"))
            .map_err(Error::ConfigRead)?;
        let sid = sid.trim();
        if !sid.starts_with("S-1-")
            || !sid
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'-' || byte == b'S')
        {
            return Err(Error::Operation("OPERATOR_SID"));
        }
        let sddl = format!("D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;{sid})");
        let encoded = wide(&sddl);
        let mut descriptor: PSECURITY_DESCRIPTOR = null_mut();
        // SAFETY: encoded is NUL-terminated and descriptor is a valid out pointer.
        let converted = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                encoded.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                null_mut(),
            )
        };
        if converted == 0 || descriptor.is_null() {
            return Err(Error::Operation("BROKER_ACL"));
        }
        // The descriptor is a single process-lifetime allocation. Keeping it alive avoids
        // per-connection allocation and the service process owns it until exit.
        Ok(Self {
            attrs: SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: descriptor,
                bInheritHandle: 0,
            },
            _descriptor: descriptor,
        })
    }
}

fn action_code(action: [&str; 2]) -> Option<u8> {
    match action {
        ["access", "configure"] => Some(1),
        ["access", "apply"] => Some(2),
        ["access", "dns"] => Some(3),
        ["compute", "apply"] => Some(4),
        ["compute", "status"] => Some(5),
        ["compute", "credentials"] => Some(6),
        ["controller", "apply"] => Some(7),
        ["controller", "status"] => Some(8),
        _ => None,
    }
}

fn code_action(code: u8) -> Option<[&'static str; 2]> {
    match code {
        1 => Some(["access", "configure"]),
        2 => Some(["access", "apply"]),
        3 => Some(["access", "dns"]),
        4 => Some(["compute", "apply"]),
        5 => Some(["compute", "status"]),
        6 => Some(["compute", "credentials"]),
        7 => Some(["controller", "apply"]),
        8 => Some(["controller", "status"]),
        _ => None,
    }
}

fn read_u8(reader: &mut impl Read) -> Result<u8> {
    let mut value = [0_u8; 1];
    reader.read_exact(&mut value).map_err(Error::Spawn)?;
    Ok(value[0])
}

fn read_u32(reader: &mut impl Read) -> Result<u32> {
    let mut value = [0_u8; 4];
    reader.read_exact(&mut value).map_err(Error::Spawn)?;
    Ok(u32::from_le_bytes(value))
}

fn wide(value: &str) -> Vec<u16> {
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broker_allowlist_is_closed() {
        assert_eq!(action_code(["compute", "status"]), Some(5));
        assert_eq!(code_action(8), Some(["controller", "status"]));
        assert_eq!(action_code(["exec", "sh"]), None);
        assert_eq!(code_action(99), None);
    }
}
