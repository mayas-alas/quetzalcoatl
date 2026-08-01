use std::ffi::c_void;
use std::mem::size_of;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr::null_mut;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use gnx_contracts::{
    Command, ForgejoAdminResponse, MAX_MESSAGE_BYTES, OperationResponse, OperationStage, PIPE_NAME,
    Request, StatusResponse,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_BROKEN_PIPE, ERROR_INSUFFICIENT_BUFFER, ERROR_IO_PENDING,
    ERROR_PIPE_CONNECTED, GetLastError, HANDLE, INVALID_HANDLE_VALUE, LocalFree, WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    CheckTokenMembership, CreateWellKnownSid, GetTokenInformation, PSECURITY_DESCRIPTOR,
    RevertToSelf, SECURITY_ATTRIBUTES, SECURITY_MAX_SID_SIZE, TOKEN_ELEVATION, TOKEN_QUERY,
    TokenElevation, TokenUser, WinBuiltinAdministratorsSid,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX, ReadFile, WriteFile,
};
use windows_sys::Win32::System::IO::{
    CancelIoEx, GetOverlappedResult, GetOverlappedResultEx, OVERLAPPED,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, ImpersonateNamedPipeClient,
    PIPE_READMODE_MESSAGE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_MESSAGE, PIPE_WAIT, PeekNamedPipe,
};
use windows_sys::Win32::System::Threading::{GetCurrentThread, OpenThreadToken};
use zeroize::Zeroize;

use super::service_shutdown::ShutdownToken;

const PIPE_SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;AU)";
const PIPE_INSTANCES: usize = 4;
const CLIENT_IO_TIMEOUT: Duration = Duration::from_secs(5);
const IO_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub fn serve(status: Arc<RwLock<StatusResponse>>, shutdown: ShutdownToken) -> Result<(), String> {
    let mut pipes = Vec::with_capacity(PIPE_INSTANCES);
    pipes.push(create_pipe(true)?);
    for _ in 1..PIPE_INSTANCES {
        pipes.push(create_pipe(false)?);
    }
    let workers: Vec<_> = pipes
        .into_iter()
        .map(|pipe| {
            let status = Arc::clone(&status);
            thread::spawn(move || serve_pipe_instance(pipe, status, shutdown))
        })
        .collect();
    for worker in workers {
        worker
            .join()
            .map_err(|_| "named-pipe worker panicked".to_string())??;
    }
    Ok(())
}

fn serve_pipe_instance(
    pipe: OwnedHandle,
    status: Arc<RwLock<StatusResponse>>,
    shutdown: ShutdownToken,
) -> Result<(), String> {
    while !shutdown.is_requested() {
        match connect(pipe.0, shutdown) {
            Ok(()) => {}
            Err(_) if shutdown.is_requested() => break,
            Err(error) => return Err(error),
        }
        match catch_unwind(AssertUnwindSafe(|| serve_client(pipe.0, &status, shutdown))) {
            Ok(Err(error)) if !shutdown.is_requested() => {
                eprintln!("gnx-service: rejected pipe request: {error}");
            }
            Err(_) if !shutdown.is_requested() => {
                eprintln!("gnx-service: isolated a panicking pipe request");
            }
            _ => {}
        }
        // Safety: pipe is either connected or has a canceled operation; disconnect resets it.
        unsafe { DisconnectNamedPipe(pipe.0) };
    }
    Ok(())
}

fn serve_client(
    pipe: HANDLE,
    status: &Arc<RwLock<StatusResponse>>,
    shutdown: ShutdownToken,
) -> Result<(), String> {
    let mut message = read_message(pipe, shutdown)?;
    let parsed = serde_json::from_slice(&message);
    message.zeroize();
    let request: Request =
        parsed.map_err(|_| "request is not valid protocol v2 JSON".to_string())?;
    let response = match request.command {
        Command::Status => {
            authorize_client(pipe, false)?;
            if request.configuration.is_some() || request.platform_configuration.is_some() {
                return Err("status request cannot contain configuration".into());
            }
            let snapshot = status
                .read()
                .map_err(|_| "runtime status lock is poisoned".to_string())?
                .clone();
            serde_json::to_vec(&snapshot)
        }
        Command::Configure => {
            let operation = match authorize_client(pipe, true) {
                Err(_) => OperationResponse::rejected(
                    "CONFIGURATION_UNAUTHORIZED",
                    "configuration requires an elevated local administrator",
                ),
                Ok(()) if request.platform_configuration.is_some() => OperationResponse::rejected(
                    "CONFIGURATION_INVALID",
                    "configure request contains platform configuration",
                ),
                Ok(()) => match request.configuration.as_ref() {
                    None => OperationResponse::rejected(
                        "CONFIGURATION_INVALID",
                        "configure request is missing configuration",
                    ),
                    Some(configuration) => {
                        match crate::infrastructure::secrets::store(configuration) {
                            Ok(()) => {
                                OperationResponse::accepted(OperationStage::ConfigurationStored)
                            }
                            Err(error) => {
                                OperationResponse::rejected(error.code(), error.message())
                            }
                        }
                    }
                },
            };
            serde_json::to_vec(&operation)
        }
        Command::ConfigurePlatform => {
            let operation = match authorize_client(pipe, true) {
                Err(_) => OperationResponse::rejected(
                    "PLATFORM_CONFIGURATION_UNAUTHORIZED",
                    "platform configuration requires an elevated local administrator",
                ),
                Ok(()) if request.configuration.is_some() => OperationResponse::rejected(
                    "PLATFORM_CONFIGURATION_INVALID",
                    "platform configure request contains installer configuration",
                ),
                Ok(()) => match request.platform_configuration.as_ref() {
                    None => OperationResponse::rejected(
                        "PLATFORM_CONFIGURATION_INVALID",
                        "platform configure request is missing configuration",
                    ),
                    Some(configuration) => {
                        match crate::infrastructure::secrets::store_platform(configuration) {
                            Ok(()) => OperationResponse::accepted(
                                OperationStage::PlatformConfigurationStored,
                            ),
                            Err(error) => {
                                OperationResponse::rejected(error.code(), error.message())
                            }
                        }
                    }
                },
            };
            serde_json::to_vec(&operation)
        }
        command @ (Command::ForgejoAdminShow | Command::ForgejoAdminReset) => {
            let reset = matches!(command, Command::ForgejoAdminReset);
            let operation = match authorize_client(pipe, true) {
                Err(_) => ForgejoAdminResponse::rejected(
                    "FORGEJO_ADMIN_UNAUTHORIZED",
                    "Forgejo administration requires an elevated local administrator",
                ),
                Ok(())
                    if request.configuration.is_some()
                        || request.platform_configuration.is_some() =>
                {
                    ForgejoAdminResponse::rejected(
                        "FORGEJO_ADMIN_INVALID",
                        "Forgejo admin request cannot contain configuration",
                    )
                }
                Ok(()) => match crate::application::platform::forgejo_admin(status, reset) {
                    Ok(response) => response,
                    Err(error) => ForgejoAdminResponse::rejected(error.code, &error.message),
                },
            };
            serde_json::to_vec(&operation)
        }
    }
    .map_err(|e| format!("cannot serialize response: {e}"))?;
    let mut response = response;
    let write_result = write_message(pipe, &response, shutdown);
    response.zeroize();
    write_result?;
    wait_for_client_close(pipe, shutdown)
}

fn create_pipe(first_instance: bool) -> Result<OwnedHandle, String> {
    let pipe_name = wide(PIPE_NAME);
    let sddl = wide(PIPE_SDDL);
    let mut descriptor: PSECURITY_DESCRIPTOR = null_mut();
    // Safety: both strings are NUL-terminated and descriptor receives memory owned by LocalFree.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            null_mut(),
        )
    } == 0
    {
        return Err(last_error("cannot create pipe security descriptor"));
    }
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let first_flag = if first_instance {
        FILE_FLAG_FIRST_PIPE_INSTANCE
    } else {
        0
    };
    // Safety: arguments describe a single local message-mode pipe and a valid security descriptor.
    let handle = unsafe {
        CreateNamedPipeW(
            pipe_name.as_ptr(),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED | first_flag,
            PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_INSTANCES as u32,
            MAX_MESSAGE_BYTES as u32,
            MAX_MESSAGE_BYTES as u32,
            0,
            &attributes,
        )
    };
    // Safety: ConvertStringSecurityDescriptor allocated descriptor with LocalAlloc.
    unsafe { LocalFree(descriptor) };
    if handle == INVALID_HANDLE_VALUE {
        Err(last_error("cannot create named pipe"))
    } else {
        Ok(OwnedHandle(handle))
    }
}

fn connect(pipe: HANDLE, shutdown: ShutdownToken) -> Result<(), String> {
    let mut overlapped = OVERLAPPED::default();
    // Safety: pipe was opened for overlapped I/O and overlapped remains alive until completion.
    if unsafe { ConnectNamedPipe(pipe, &mut overlapped) } != 0 {
        return Ok(());
    }
    // A client can connect between CreateNamedPipe and ConnectNamedPipe.
    match unsafe { GetLastError() } {
        ERROR_PIPE_CONNECTED => Ok(()),
        ERROR_IO_PENDING => wait_overlapped(
            pipe,
            &mut overlapped,
            None,
            shutdown,
            "cannot accept named-pipe client",
        )
        .map(|_| ()),
        _ => Err(last_error("cannot accept named-pipe client")),
    }
}

fn authorize_client(pipe: HANDLE, require_elevated_admin: bool) -> Result<(), String> {
    // Safety: pipe has a connected local client. Windows supplies the client token.
    if unsafe { ImpersonateNamedPipeClient(pipe) } == 0 {
        return Err(last_error("cannot impersonate named-pipe client"));
    }
    let result = read_thread_token(require_elevated_admin);
    // Safety: this thread is impersonating only within this function.
    let reverted = unsafe { RevertToSelf() };
    if reverted == 0 {
        return Err(last_error("cannot revert named-pipe impersonation"));
    }
    result
}

fn read_thread_token(require_elevated_admin: bool) -> Result<(), String> {
    let mut token: HANDLE = null_mut();
    // Safety: token receives a query-only handle for the current impersonated thread.
    if unsafe { OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, 0, &mut token) } == 0 {
        return Err(last_error("cannot open named-pipe client token"));
    }
    let token = OwnedHandle(token);
    let mut required = 0u32;
    // Safety: the first call intentionally obtains the required TOKEN_USER buffer size.
    let first = unsafe { GetTokenInformation(token.0, TokenUser, null_mut(), 0, &mut required) };
    if first != 0 || unsafe { GetLastError() } != ERROR_INSUFFICIENT_BUFFER || required == 0 {
        return Err(last_error("cannot size named-pipe client token"));
    }
    let mut buffer = vec![0u8; required as usize];
    // Safety: buffer has exactly the size requested by GetTokenInformation.
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr() as *mut c_void,
            required,
            &mut required,
        )
    } == 0
    {
        return Err(last_error("cannot read named-pipe client token"));
    }
    if !require_elevated_admin {
        return Ok(());
    }

    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut elevation_size = 0u32;
    // Safety: elevation is a correctly sized TOKEN_ELEVATION output buffer.
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenElevation,
            &mut elevation as *mut _ as *mut c_void,
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut elevation_size,
        )
    } == 0
        || elevation.TokenIsElevated == 0
    {
        return Err("named-pipe client is not elevated".into());
    }

    let mut admin_sid = vec![0u8; SECURITY_MAX_SID_SIZE as usize];
    let mut admin_sid_size = admin_sid.len() as u32;
    // Safety: buffer is large enough for any well-known SID and receives its exact size.
    if unsafe {
        CreateWellKnownSid(
            WinBuiltinAdministratorsSid,
            null_mut(),
            admin_sid.as_mut_ptr().cast(),
            &mut admin_sid_size,
        )
    } == 0
    {
        return Err(last_error("cannot construct local Administrators SID"));
    }
    let mut is_admin = 0;
    // Safety: token is the connected client impersonation token and SID is initialized above.
    if unsafe { CheckTokenMembership(token.0, admin_sid.as_mut_ptr().cast(), &mut is_admin) } == 0 {
        return Err(last_error(
            "cannot check named-pipe client group membership",
        ));
    }
    if is_admin == 0 {
        return Err("named-pipe client is not a local administrator".into());
    }
    Ok(())
}

fn read_message(pipe: HANDLE, shutdown: ShutdownToken) -> Result<Vec<u8>, String> {
    let mut buffer = vec![0u8; MAX_MESSAGE_BYTES];
    let mut read = 0u32;
    let mut overlapped = OVERLAPPED::default();
    // Safety: buffer is writable, pipe is connected/overlapped and all values live through wait.
    let completed = unsafe {
        ReadFile(
            pipe,
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            &mut read,
            &mut overlapped,
        )
    };
    if completed == 0 {
        if unsafe { GetLastError() } != ERROR_IO_PENDING {
            return Err(last_error("cannot read named-pipe request"));
        }
        read = wait_overlapped(
            pipe,
            &mut overlapped,
            Some(CLIENT_IO_TIMEOUT),
            shutdown,
            "cannot read named-pipe request",
        )?;
    }
    buffer.truncate(read as usize);
    Ok(buffer)
}

fn write_message(pipe: HANDLE, bytes: &[u8], shutdown: ShutdownToken) -> Result<(), String> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err("response exceeds protocol v2 message limit".into());
    }
    let mut written = 0u32;
    let mut overlapped = OVERLAPPED::default();
    // Safety: bytes is readable, pipe is connected/overlapped and all values live through wait.
    let completed = unsafe {
        WriteFile(
            pipe,
            bytes.as_ptr(),
            bytes.len() as u32,
            &mut written,
            &mut overlapped,
        )
    };
    if completed == 0 {
        if unsafe { GetLastError() } != ERROR_IO_PENDING {
            return Err(last_error("cannot write named-pipe response"));
        }
        written = wait_overlapped(
            pipe,
            &mut overlapped,
            Some(CLIENT_IO_TIMEOUT),
            shutdown,
            "cannot write named-pipe response",
        )?;
    }
    if written as usize != bytes.len() {
        return Err("named-pipe response was only partially written".into());
    }
    Ok(())
}

fn wait_for_client_close(pipe: HANDLE, shutdown: ShutdownToken) -> Result<(), String> {
    let started = Instant::now();
    loop {
        if shutdown.is_requested() {
            return Err("service shutdown requested".into());
        }
        let mut available = 0u32;
        // Safety: PeekNamedPipe is nonblocking and available points to initialized storage.
        if unsafe { PeekNamedPipe(pipe, null_mut(), 0, null_mut(), &mut available, null_mut()) }
            == 0
        {
            return if unsafe { GetLastError() } == ERROR_BROKEN_PIPE {
                Ok(())
            } else {
                Err(last_error("cannot confirm named-pipe client completion"))
            };
        }
        if started.elapsed() >= CLIENT_IO_TIMEOUT {
            return Err("named-pipe client did not close after reading the response".into());
        }
        thread::sleep(IO_POLL_INTERVAL);
    }
}

fn wait_overlapped(
    pipe: HANDLE,
    overlapped: &mut OVERLAPPED,
    timeout: Option<Duration>,
    shutdown: ShutdownToken,
    context: &str,
) -> Result<u32, String> {
    let started = Instant::now();
    loop {
        if shutdown.is_requested() {
            cancel_overlapped(pipe, overlapped);
            return Err("service shutdown requested".into());
        }
        let remaining = timeout.map(|limit| limit.saturating_sub(started.elapsed()));
        if remaining.is_some_and(|duration| duration.is_zero()) {
            cancel_overlapped(pipe, overlapped);
            return Err(format!("{context}: client I/O timed out"));
        }
        let wait = remaining
            .map(|duration| duration.min(IO_POLL_INTERVAL))
            .unwrap_or(IO_POLL_INTERVAL);
        let wait_ms = u32::try_from(wait.as_millis()).unwrap_or(u32::MAX).max(1);
        let mut transferred = 0u32;
        // Safety: the I/O and OVERLAPPED remain valid, and the bounded wait is non-alertable.
        if unsafe { GetOverlappedResultEx(pipe, overlapped, &mut transferred, wait_ms, 0) } != 0 {
            return Ok(transferred);
        }
        if unsafe { GetLastError() } != WAIT_TIMEOUT {
            return Err(last_error(context));
        }
    }
}

fn cancel_overlapped(pipe: HANDLE, overlapped: &mut OVERLAPPED) {
    // Safety: the OVERLAPPED belongs to a pending operation on pipe. Waiting reaps cancellation
    // before its stack storage is released.
    unsafe {
        CancelIoEx(pipe, overlapped);
        let mut transferred = 0u32;
        GetOverlappedResult(pipe, overlapped, &mut transferred, 1);
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

fn last_error(operation: &str) -> String {
    // Safety: GetLastError has no parameters and follows a failing Win32 call.
    unsafe { format!("{operation} (Win32 {})", GetLastError()) }
}

struct OwnedHandle(HANDLE);

// Safety: this wrapper uniquely owns a kernel handle. Named-pipe handles may be transferred
// between threads and are closed exactly once by Drop.
unsafe impl Send for OwnedHandle {}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // Safety: this type owns a non-null kernel handle exactly once.
        unsafe { CloseHandle(self.0) };
    }
}
