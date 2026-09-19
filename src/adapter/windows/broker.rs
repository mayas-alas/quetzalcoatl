use crate::{domain::secret::Secret, report::Report};
use std::{
    io,
    path::Path,
    process::{Command, Stdio},
    ptr,
};
use windows_sys::Win32::{
    Foundation::*,
    Security::{
        Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW, SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::*,
    System::{Pipes::*, Threading::*, IO::*},
};
use zeroize::Zeroizing;
const PIPE: &str = "\\\\.\\pipe\\GNX";
const RESPONSE_LIMIT: usize = 1024 * 1024;
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
pub fn validate_operator_sid() -> io::Result<()> {
    let sid = std::fs::read_to_string(super::setup::TARGET_DATA.to_owned() + "\\operator.sid")?;
    let sid = sid.trim();
    let valid = sid
        .strip_prefix("S-1-")
        .map(|rest| {
            let parts: Vec<_> = rest.split('-').collect();
            !parts.is_empty()
                && parts.iter().all(|part| {
                    !part.is_empty()
                        && part.bytes().all(|b| b.is_ascii_digit())
                        && part.parse::<u64>().is_ok()
                })
        })
        .unwrap_or(false);
    if !valid {
        return Err(invalid());
    }
    Ok(())
}
fn event() -> io::Result<Handle> {
    let h = unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) };
    if h.is_null() {
        Err(io::Error::last_os_error())
    } else {
        Ok(Handle(h))
    }
}
// Pending overlapped operations are cancelled and drained before stack/buffer destruction.
unsafe fn finish(h: HANDLE, over: &mut OVERLAPPED, initial: i32, timeout: u32) -> io::Result<u32> {
    if initial == 0 {
        let error = GetLastError();
        if error != ERROR_IO_PENDING {
            return Err(io::Error::from_raw_os_error(error as i32));
        }
        if WaitForSingleObject(over.hEvent, timeout) != WAIT_OBJECT_0 {
            CancelIoEx(h, over);
            let mut n = 0;
            GetOverlappedResult(h, over, &mut n, 1);
            return Err(io::ErrorKind::TimedOut.into());
        }
    }
    let mut n = 0;
    if GetOverlappedResult(h, over, &mut n, 0) == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(n)
}
fn read_message(h: HANDLE, limit: usize, timeout: u32) -> io::Result<Zeroizing<Vec<u8>>> {
    let e = event()?;
    let mut over: OVERLAPPED = unsafe { std::mem::zeroed() };
    over.hEvent = e.0;
    let mut b = Zeroizing::new(vec![0; limit + 1]);
    let n = unsafe {
        let initial = ReadFile(
            h,
            b.as_mut_ptr(),
            b.len() as u32,
            ptr::null_mut(),
            &mut over,
        );
        finish(h, &mut over, initial, timeout)?
    } as usize;
    if n > limit {
        return Err(invalid());
    }
    b.truncate(n);
    Ok(b)
}
fn write_message(h: HANDLE, b: &[u8], timeout: u32) -> io::Result<()> {
    let e = event()?;
    let mut over: OVERLAPPED = unsafe { std::mem::zeroed() };
    over.hEvent = e.0;
    let n = unsafe {
        let initial = WriteFile(h, b.as_ptr(), b.len() as u32, ptr::null_mut(), &mut over);
        finish(h, &mut over, initial, timeout)?
    };
    if n as usize != b.len() {
        return Err(invalid());
    }
    Ok(())
}
pub fn request(op: &str, intent: &str) -> io::Result<Report> {
    request_secret(op, intent, None)
}
pub fn request_secret(op: &str, intent: &str, secret: Option<&Secret>) -> io::Result<Report> {
    let name = wide(PIPE);
    let raw = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED,
            ptr::null_mut(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let h = Handle(raw);
    let mut pid = 0;
    if unsafe { GetNamedPipeServerProcessId(h.0, &mut pid) } == 0 {
        return Err(io::Error::last_os_error());
    }
    use windows_service::{
        service::ServiceAccess,
        service_manager::{ServiceManager, ServiceManagerAccess},
    };
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|_| invalid())?;
    let service = manager
        .open_service("GNXRuntime", ServiceAccess::QUERY_STATUS)
        .map_err(|_| invalid())?;
    let service_status = service.query_status().map_err(|_| invalid())?;
    if service_status.current_state != windows_service::service::ServiceState::Running
        || service_status.process_id != Some(pid)
    {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let mode = PIPE_READMODE_MESSAGE;
    if unsafe { SetNamedPipeHandleState(h.0, &mode, ptr::null(), ptr::null()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let b = crate::wire::encode(op, intent, secret).map_err(|_| invalid())?;
    write_message(h.0, &b, 5000)?;
    let bytes = read_message(h.0, RESPONSE_LIMIT, 650_000)?;
    let r: Report = serde_json::from_slice(&bytes)?;
    if let Some(root) = r.public_root.as_deref() {
        // The service authenticates this response through the protected broker.
        // Persist only the public CA and install it for the interactive operator
        // so browser TLS works without a manual certificate action. Failure is
        // deliberately best-effort: the report remains truthful and the next
        // broker call retries the bounded import.
        let _ = install_public_root(root);
    }
    if r.schema != 1 || r.operation != op || (r.secret_kind.is_some() && op != "apply") {
        return Err(invalid());
    }
    write_message(h.0, b"A", 5000)?;
    Ok(r)
}
fn install_public_root(pem: &str) -> io::Result<()> {
    if pem.len() > 16 * 1024
        || !pem.starts_with("-----BEGIN CERTIFICATE-----")
        || !pem.trim_end().ends_with("-----END CERTIFICATE-----")
        || pem.contains('\0')
    {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let path = Path::new(super::setup::TARGET_DATA).join("root.crt");
    crate::adapter::filesystem::atomic_write(&path, pem.as_bytes(), 0o644)
        .map_err(|_| io::ErrorKind::PermissionDenied)?;
    let status = Command::new(r"C:\Windows\System32\certutil.exe")
        .args(["-user", "-f", "-addstore", "Root"])
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status.code().unwrap_or(1)))
    }
}

pub fn serve_one() -> io::Result<()> {
    validate_operator_sid()?;
    let sid = std::fs::read_to_string(super::setup::TARGET_DATA.to_owned() + "\\operator.sid")?;
    let sid = sid.trim();
    // Do not create a readiness pipe while SCM still considers the service
    // stopped, stopping, or start-pending. This also makes status evidence
    // honest when startup failed before the broker became usable.
    {
        use windows_service::{
            service::ServiceAccess,
            service_manager::{ServiceManager, ServiceManagerAccess},
        };
        let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
            .map_err(|_| invalid())?;
        let service = manager
            .open_service("GNXRuntime", ServiceAccess::QUERY_STATUS)
            .map_err(|_| invalid())?;
        if service.query_status().map_err(|_| invalid())?.current_state
            != windows_service::service::ServiceState::Running
        {
            return Err(io::ErrorKind::WouldBlock.into());
        }
    }
    let sddl = wide(&format!("D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;{sid})"));
    let mut sd = ptr::null_mut();
    unsafe {
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut sd,
            ptr::null_mut(),
        ) == 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    let sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd,
        bInheritHandle: 0,
    };
    let name = wide(PIPE);
    let raw = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE | FILE_FLAG_OVERLAPPED,
            PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            RESPONSE_LIMIT as u32,
            crate::wire::MAX_FRAME as u32,
            5000,
            &sa,
        )
    };
    unsafe {
        LocalFree(sd);
    }
    if raw == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let h = Handle(raw);
    let e = event()?;
    let mut over: OVERLAPPED = unsafe { std::mem::zeroed() };
    over.hEvent = e.0;
    unsafe {
        let initial = ConnectNamedPipe(h.0, &mut over);
        if initial == 0 && GetLastError() != ERROR_PIPE_CONNECTED {
            finish(h.0, &mut over, initial, 1000)?;
        }
    }
    let bytes = read_message(h.0, crate::wire::MAX_FRAME, 5000)?;
    let q = crate::wire::decode(&bytes).map_err(|_| invalid())?;
    let mut available = 0;
    if unsafe {
        PeekNamedPipe(
            h.0,
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            &mut available,
            ptr::null_mut(),
        )
    } == 0
        || available != 0
    {
        return Err(invalid());
    }
    let r = super::runtime::invoke_secret(q.operation, &q.intent, q.secret.as_ref());
    let reply = serde_json::to_vec(&r)?;
    if reply.len() > RESPONSE_LIMIT {
        return Err(invalid());
    }
    write_message(h.0, &reply, 5000)?;
    if read_message(h.0, 1, 5000)?.as_slice() != b"A" {
        return Err(invalid());
    }
    unsafe {
        DisconnectNamedPipe(h.0);
    }
    Ok(())
}
