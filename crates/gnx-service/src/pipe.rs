use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::null_mut;
use std::sync::{Arc, RwLock};

use gnx_protocol::{
    Command, MAX_MESSAGE_BYTES, OperationResponse, PIPE_NAME, Request, StatusResponse,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_INSUFFICIENT_BUFFER, ERROR_PIPE_CONNECTED, GetLastError, HANDLE,
    INVALID_HANDLE_VALUE, LocalFree,
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
    FILE_FLAG_FIRST_PIPE_INSTANCE, FlushFileBuffers, PIPE_ACCESS_DUPLEX, ReadFile, WriteFile,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, ImpersonateNamedPipeClient,
    PIPE_READMODE_MESSAGE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_MESSAGE, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{GetCurrentThread, OpenThreadToken};
use zeroize::Zeroize;

const PIPE_SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;AU)";

pub fn serve(status: Arc<RwLock<StatusResponse>>) -> Result<(), String> {
    let pipe = create_pipe()?;
    loop {
        connect(pipe.0)?;
        if let Err(error) = serve_client(pipe.0, &status) {
            eprintln!("gnx-service: rejected pipe request: {error}");
        }
        // Safety: the handle is a connected named-pipe server instance.
        unsafe {
            FlushFileBuffers(pipe.0);
            DisconnectNamedPipe(pipe.0);
        }
    }
}

fn serve_client(pipe: HANDLE, status: &Arc<RwLock<StatusResponse>>) -> Result<(), String> {
    let mut message = read_message(pipe)?;
    let parsed = serde_json::from_slice(&message);
    message.zeroize();
    let request: Request =
        parsed.map_err(|_| "request is not valid protocol v1 JSON".to_string())?;
    let response = match request.command {
        Command::Status => {
            authorize_client(pipe, false)?;
            if request.configuration.is_some() || request.forgejo_configuration.is_some() {
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
                Ok(()) if request.forgejo_configuration.is_some() => OperationResponse::rejected(
                    "CONFIGURATION_INVALID",
                    "configure request contains an unexpected Forgejo configuration",
                ),
                Ok(()) => match request.configuration.as_ref() {
                    None => OperationResponse::rejected(
                        "CONFIGURATION_INVALID",
                        "configure request is missing configuration",
                    ),
                    Some(configuration) => match crate::secrets::store(configuration) {
                        Ok(()) => OperationResponse::accepted("CONFIGURATION_STORED"),
                        Err(error) => OperationResponse::rejected(error.code(), error.message()),
                    },
                },
            };
            serde_json::to_vec(&operation)
        }
        Command::ConfigureForgejo => {
            let operation = match authorize_client(pipe, true) {
                Err(_) => OperationResponse::rejected(
                    "FORGEJO_CONFIGURATION_UNAUTHORIZED",
                    "Forgejo configuration requires an elevated local administrator",
                ),
                Ok(()) if request.configuration.is_some() => OperationResponse::rejected(
                    "FORGEJO_CONFIGURATION_INVALID",
                    "Forgejo configure request contains an unexpected installer configuration",
                ),
                Ok(()) => match request.forgejo_configuration.as_ref() {
                    None => OperationResponse::rejected(
                        "FORGEJO_CONFIGURATION_INVALID",
                        "Forgejo configure request is missing its configuration",
                    ),
                    Some(configuration) => {
                        match crate::runtime_gate::configure_forgejo(
                            &configuration.username,
                            &configuration.password,
                        ) {
                            Ok(()) => OperationResponse::accepted("FORGEJO_CONFIGURATION_STORED"),
                            Err((code, message)) => OperationResponse::rejected(&code, &message),
                        }
                    }
                },
            };
            serde_json::to_vec(&operation)
        }
    }
    .map_err(|e| format!("cannot serialize response: {e}"))?;
    write_message(pipe, &response)
}

fn create_pipe() -> Result<OwnedHandle, String> {
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
    // Safety: arguments describe a single local message-mode pipe and a valid security descriptor.
    let handle = unsafe {
        CreateNamedPipeW(
            pipe_name.as_ptr(),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
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

fn connect(pipe: HANDLE) -> Result<(), String> {
    // Safety: pipe is a synchronous named-pipe server handle.
    if unsafe { ConnectNamedPipe(pipe, null_mut()) } != 0 {
        return Ok(());
    }
    // A client can connect between CreateNamedPipe and ConnectNamedPipe.
    if unsafe { GetLastError() } == ERROR_PIPE_CONNECTED {
        Ok(())
    } else {
        Err(last_error("cannot accept named-pipe client"))
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

fn read_message(pipe: HANDLE) -> Result<Vec<u8>, String> {
    let mut buffer = vec![0u8; MAX_MESSAGE_BYTES];
    let mut read = 0u32;
    // Safety: buffer is writable and pipe is a connected synchronous handle.
    if unsafe {
        ReadFile(
            pipe,
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            &mut read,
            null_mut(),
        )
    } == 0
    {
        return Err(last_error("cannot read named-pipe request"));
    }
    buffer.truncate(read as usize);
    Ok(buffer)
}

fn write_message(pipe: HANDLE, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err("response exceeds protocol v1 message limit".into());
    }
    let mut written = 0u32;
    // Safety: bytes is readable and pipe is a connected synchronous handle.
    if unsafe {
        WriteFile(
            pipe,
            bytes.as_ptr(),
            bytes.len() as u32,
            &mut written,
            null_mut(),
        )
    } == 0
        || written as usize != bytes.len()
    {
        return Err(last_error("cannot write named-pipe response"));
    }
    Ok(())
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

fn last_error(operation: &str) -> String {
    // Safety: GetLastError has no parameters and follows a failing Win32 call.
    unsafe { format!("{operation} (Win32 {})", GetLastError()) }
}

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // Safety: this type owns a non-null kernel handle exactly once.
        unsafe { CloseHandle(self.0) };
    }
}
