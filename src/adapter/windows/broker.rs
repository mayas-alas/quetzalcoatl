use crate::{domain::secret::Secret, report::Report};
use std::{io, ptr};
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
    if service.query_status().map_err(|_| invalid())?.process_id != Some(pid) {
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
    if r.schema != 1 || r.operation != op || (r.secret_kind.is_some() && op != "apply") {
        return Err(invalid());
    }
    write_message(h.0, b"A", 5000)?;
    Ok(r)
}
pub fn serve_one() -> io::Result<()> {
    let sid = std::fs::read_to_string("C:\\ProgramData\\GNX\\operator.sid")?;
    let sid = sid.trim();
    if !sid.starts_with("S-1-")
        || !sid
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'S' || b == b'-')
    {
        return Err(invalid());
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
