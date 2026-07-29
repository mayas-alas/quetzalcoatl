use std::ptr::null;
use std::thread;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_FILE_NOT_FOUND, GetLastError, WAIT_OBJECT_0,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, EVENT_MODIFY_STATE, INFINITE, OpenEventW, SetEvent, WaitForSingleObject,
};

const SHUTDOWN_EVENT: &str = "Local\\Quetzalcoatl.GnxService.Shutdown";

pub(crate) fn arm() -> Result<(), String> {
    let name = wide(SHUTDOWN_EVENT);
    let event = unsafe { CreateEventW(null(), 1, 0, name.as_ptr()) };
    if event.is_null() {
        return Err(last_error("cannot create service shutdown event"));
    }
    let event_address = event as usize;
    thread::spawn(move || {
        let event = event_address as *mut core::ffi::c_void;
        let result = unsafe { WaitForSingleObject(event, INFINITE) };
        unsafe {
            CloseHandle(event);
        }
        if result == WAIT_OBJECT_0 {
            std::process::exit(0);
        }
    });
    Ok(())
}

pub(crate) fn signal() -> Result<(), String> {
    let name = wide(SHUTDOWN_EVENT);
    let event = unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, name.as_ptr()) };
    if event.is_null() {
        return if unsafe { GetLastError() } == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(last_error("cannot open service shutdown event"))
        };
    }
    let signaled = unsafe { SetEvent(event) };
    unsafe {
        CloseHandle(event);
    }
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
