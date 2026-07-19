use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::null_mut;

use gnx_protocol::{Command, MAX_MESSAGE_BYTES, PIPE_NAME, Request, StatusResponse};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_INSUFFICIENT_BUFFER, ERROR_PIPE_CONNECTED, GetLastError, HANDLE,
    INVALID_HANDLE_VALUE, LocalFree,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, PSECURITY_DESCRIPTOR, RevertToSelf, SECURITY_ATTRIBUTES, TOKEN_QUERY,
    TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_FIRST_PIPE_INSTANCE, FlushFileBuffers, PIPE_ACCESS_DUPLEX, ReadFile, WriteFile,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, ImpersonateNamedPipeClient,
    PIPE_READMODE_MESSAGE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_MESSAGE, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{GetCurrentThread, OpenThreadToken};

const PIPE_SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;AU)";

pub fn serve() -> Result<(), String> {
    let pipe = create_pipe()?;
    loop {
        connect(pipe.0)?;
        if let Err(error) = serve_client(pipe.0) {
            eprintln!("gnx-service: rejected pipe request: {error}");
        }
        // Safety: the handle is a connected named-pipe server instance.
        unsafe {
            FlushFileBuffers(pipe.0);
            DisconnectNamedPipe(pipe.0);
        }
    }
}

fn serve_client(pipe: HANDLE) -> Result<(), String> {
    let message = read_message(pipe)?;
    authorize_client(pipe)?;
    let request: Request = serde_json::from_slice(&message)
        .map_err(|_| "request is not valid protocol v1 JSON".to_string())?;
    let response = match request.command {
        Command::Status => serde_json::to_vec(&StatusResponse::service_ready()),
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

fn authorize_client(pipe: HANDLE) -> Result<(), String> {
    // Safety: pipe has a connected local client. Windows supplies the client token.
    if unsafe { ImpersonateNamedPipeClient(pipe) } == 0 {
        return Err(last_error("cannot impersonate named-pipe client"));
    }
    let result = read_thread_token();
    // Safety: this thread is impersonating only within this function.
    let reverted = unsafe { RevertToSelf() };
    if reverted == 0 {
        return Err(last_error("cannot revert named-pipe impersonation"));
    }
    result
}

fn read_thread_token() -> Result<(), String> {
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
