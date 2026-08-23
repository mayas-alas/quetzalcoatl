use std::mem::size_of;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, ERROR_FILE_NOT_FOUND, GetLastError, LocalFree, WAIT_OBJECT_0,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows_sys::Win32::System::Threading::{
    CreateEventW, EVENT_MODIFY_STATE, INFINITE, OpenEventW, SetEvent, WaitForSingleObject,
};

use gnx_contracts::WINDOWS_SERVICE_SID;

use crate::domain::errors::GateError;
use crate::domain::lifecycle::Component;

const SHUTDOWN_EVENT: &str = "Local\\Quetzalcoatl.GnxService.Shutdown";
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy)]
pub(crate) struct ShutdownToken;

impl ShutdownToken {
    pub(crate) fn is_requested(self) -> bool {
        requested()
    }
}

pub(crate) fn requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::Acquire)
}

pub(crate) fn ensure_running() -> Result<(), GateError> {
    if requested() {
        Err(GateError::new(
            "SERVICE_STOPPING",
            Component::None,
            "service shutdown was requested",
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn arm() -> Result<ShutdownToken, String> {
    SHUTDOWN_REQUESTED.store(false, Ordering::Release);
    let descriptor = SecurityDescriptor::new()?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    let name = wide(SHUTDOWN_EVENT);
    // Safety: attributes and name remain valid for the duration of CreateEventW.
    let event = unsafe { CreateEventW(&attributes, 1, 0, name.as_ptr()) };
    if event.is_null() {
        return Err(last_error("cannot create service shutdown event"));
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe { CloseHandle(event) };
        return Err("service shutdown event was precreated; refusing to start".into());
    }
    let event_address = event as usize;
    thread::spawn(move || {
        let event = event_address as *mut core::ffi::c_void;
        let result = unsafe { WaitForSingleObject(event, INFINITE) };
        if result == WAIT_OBJECT_0 {
            SHUTDOWN_REQUESTED.store(true, Ordering::Release);
        }
        unsafe { CloseHandle(event) };
    });
    Ok(ShutdownToken)
}

pub(crate) fn signal() -> Result<(), String> {
    let name = wide(SHUTDOWN_EVENT);
    // Safety: name is NUL-terminated and access is restricted to signaling.
    let event = unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, name.as_ptr()) };
    if event.is_null() {
        return if unsafe { GetLastError() } == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(last_error("cannot open service shutdown event"))
        };
    }
    let signaled = unsafe { SetEvent(event) };
    unsafe { CloseHandle(event) };
    if signaled == 0 {
        Err(last_error("cannot signal service shutdown event"))
    } else {
        Ok(())
    }
}

fn last_error(context: &str) -> String {
    format!("{context}: Win32 {}", unsafe { GetLastError() })
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

struct SecurityDescriptor(PSECURITY_DESCRIPTOR);

impl SecurityDescriptor {
    fn new() -> Result<Self, String> {
        let sddl = wide(&format!("D:P(A;;GA;;;SY)(A;;GA;;;{WINDOWS_SERVICE_SID})"));
        let mut descriptor: PSECURITY_DESCRIPTOR = null_mut();
        // Safety: sddl is NUL-terminated and descriptor receives LocalAlloc-owned memory.
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                null_mut(),
            )
        } == 0
        {
            Err(last_error(
                "cannot create service shutdown security descriptor",
            ))
        } else {
            Ok(Self(descriptor))
        }
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        // Safety: ConvertStringSecurityDescriptor allocated this descriptor with LocalAlloc.
        unsafe { LocalFree(self.0) };
    }
}
