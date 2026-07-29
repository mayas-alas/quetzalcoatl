use std::mem::size_of;
use std::ptr::{null, null_mut};
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::System::Services::{
    CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW, QueryServiceStatusEx,
    SC_MANAGER_CONNECT, SC_STATUS_PROCESS_INFO, SERVICE_CONTROL_STOP, SERVICE_QUERY_STATUS,
    SERVICE_RUNNING, SERVICE_START, SERVICE_STATUS, SERVICE_STATUS_PROCESS, SERVICE_STOP,
    SERVICE_STOPPED, StartServiceW,
};

use crate::error::CliResult;

pub(crate) fn run() -> CliResult<()> {
    restart_service()?;
    println!("Quetzalcoatl service restarted");
    Ok(())
}

fn restart_service() -> CliResult<()> {
    let manager = unsafe { OpenSCManagerW(null(), null(), SC_MANAGER_CONNECT) };
    if manager.is_null() {
        return Err(last_win32("cannot open Windows Service Control Manager"));
    }
    let manager = OwnedServiceHandle(manager);
    let name = wide("Quetzalcoatl");
    let service = unsafe {
        OpenServiceW(
            manager.0,
            name.as_ptr(),
            SERVICE_STOP | SERVICE_START | SERVICE_QUERY_STATUS,
        )
    };
    if service.is_null() {
        return Err(last_win32(
            "cannot open Quetzalcoatl service; run gnx from an elevated administrator console",
        ));
    }
    let service = OwnedServiceHandle(service);
    let current = query_service_status(service.0)?;
    if current.dwCurrentState != SERVICE_STOPPED {
        let mut status = SERVICE_STATUS::default();
        if unsafe { ControlService(service.0, SERVICE_CONTROL_STOP, &mut status) } == 0 {
            return Err(last_win32(
                "cannot stop Quetzalcoatl service; run gnx from an elevated administrator console",
            ));
        }
        wait_for_service_state(service.0, SERVICE_STOPPED)?;
    }
    if unsafe { StartServiceW(service.0, 0, null_mut()) } == 0 {
        return Err(last_win32("cannot start Quetzalcoatl service"));
    }
    wait_for_service_state(service.0, SERVICE_RUNNING)
}

fn query_service_status(
    service: windows_sys::Win32::System::Services::SC_HANDLE,
) -> CliResult<SERVICE_STATUS_PROCESS> {
    let mut status = SERVICE_STATUS_PROCESS::default();
    let mut required = 0u32;
    if unsafe {
        QueryServiceStatusEx(
            service,
            SC_STATUS_PROCESS_INFO,
            (&mut status as *mut SERVICE_STATUS_PROCESS).cast(),
            size_of::<SERVICE_STATUS_PROCESS>() as u32,
            &mut required,
        )
    } == 0
    {
        Err(last_win32("cannot query Quetzalcoatl service status"))
    } else {
        Ok(status)
    }
}

fn wait_for_service_state(
    service: windows_sys::Win32::System::Services::SC_HANDLE,
    expected: u32,
) -> CliResult<()> {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let status = query_service_status(service)?;
        if status.dwCurrentState == expected {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("timed out waiting for Quetzalcoatl service state transition".into());
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

fn last_win32(operation: &str) -> String {
    unsafe { format!("{operation} (Win32 {})", GetLastError()) }
}

struct OwnedServiceHandle(windows_sys::Win32::System::Services::SC_HANDLE);

impl Drop for OwnedServiceHandle {
    fn drop(&mut self) {
        unsafe { CloseServiceHandle(self.0) };
    }
}
