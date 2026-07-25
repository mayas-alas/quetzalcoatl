use std::ptr::{null, null_mut};

use gnx_protocol::{
    Command, InstallerConfiguration, MAX_MESSAGE_BYTES, OperationResponse, PIPE_NAME,
    PROTOCOL_SCHEMA_VERSION, Request, StatusResponse,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, GENERIC_READ, GENERIC_WRITE, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING, ReadFile, WriteFile,
};
use windows_sys::Win32::System::Pipes::{
    PIPE_READMODE_MESSAGE, SetNamedPipeHandleState, WaitNamedPipeW,
};
use zeroize::Zeroize;

pub fn status() -> Result<StatusResponse, String> {
    let pipe = connect()?;
    let request = serde_json::to_vec(&Request {
        command: Command::Status,
        configuration: None,
    })
    .map_err(|e| format!("cannot encode request: {e}"))?;
    write_message(pipe.0, &request)?;
    decode_status_response(&read_message(pipe.0)?)
}

pub fn configure(configuration: InstallerConfiguration) -> Result<OperationResponse, String> {
    let pipe = connect()?;
    let request = Request {
        command: Command::Configure,
        configuration: Some(configuration),
    };
    let mut bytes = serde_json::to_vec(&request)
        .map_err(|e| format!("cannot encode configure request: {e}"))?;
    drop(request);
    let write_result = write_message(pipe.0, &bytes);
    bytes.zeroize();
    write_result?;
    decode_operation_response(&read_message(pipe.0)?)
}

fn decode_status_response(bytes: &[u8]) -> Result<StatusResponse, String> {
    let response: StatusResponse = serde_json::from_slice(bytes)
        .map_err(|e| format!("service returned invalid protocol v2 JSON: {e}"))?;
    if response.schema_version != PROTOCOL_SCHEMA_VERSION {
        return Err(format!(
            "service protocol schema mismatch: expected {}, received {}",
            PROTOCOL_SCHEMA_VERSION, response.schema_version
        ));
    }
    Ok(response)
}

fn decode_operation_response(bytes: &[u8]) -> Result<OperationResponse, String> {
    let response: OperationResponse = serde_json::from_slice(bytes)
        .map_err(|e| format!("service returned invalid protocol v2 JSON: {e}"))?;
    if response.schema_version != PROTOCOL_SCHEMA_VERSION {
        return Err(format!(
            "service protocol schema mismatch: expected {}, received {}",
            PROTOCOL_SCHEMA_VERSION, response.schema_version
        ));
    }
    Ok(response)
}

fn connect() -> Result<OwnedHandle, String> {
    let name = wide(PIPE_NAME);
    // Safety: name is NUL-terminated. A five-second wait avoids an unbounded CLI hang.
    if unsafe { WaitNamedPipeW(name.as_ptr(), 5_000) } == 0 {
        return Err(last_error("service pipe is unavailable"));
    }
    // Safety: name is NUL-terminated and all optional pointers are null.
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(last_error("cannot connect to service pipe"));
    }
    let handle = OwnedHandle(handle);
    let mode = PIPE_READMODE_MESSAGE;
    // Safety: handle is a connected named-pipe client and mode points to a valid value.
    if unsafe { SetNamedPipeHandleState(handle.0, &mode, null(), null()) } == 0 {
        return Err(last_error("cannot set service pipe message mode"));
    }
    Ok(handle)
}

fn write_message(pipe: HANDLE, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err("request exceeds protocol v2 message limit".into());
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
        return Err(last_error("cannot write service request"));
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
        return Err(last_error("cannot read service response"));
    }
    buffer.truncate(read as usize);
    Ok(buffer)
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
