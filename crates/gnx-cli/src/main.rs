#[cfg(windows)]
mod pipe;

use std::env;
#[cfg(windows)]
use std::io::{self, Write};
#[cfg(windows)]
use std::mem::size_of;
#[cfg(windows)]
use std::ptr::{null, null_mut};
#[cfg(windows)]
use std::thread;
#[cfg(windows)]
use std::time::{Duration, Instant};

use gnx_protocol::{InstallerConfiguration, StatusResponse};
#[cfg(windows)]
use windows_sys::Win32::Foundation::{GetLastError, HANDLE, INVALID_HANDLE_VALUE};
#[cfg(windows)]
use windows_sys::Win32::System::Console::{
    ENABLE_ECHO_INPUT, GetConsoleMode, GetStdHandle, STD_INPUT_HANDLE, SetConsoleMode,
};
#[cfg(windows)]
use windows_sys::Win32::System::Services::{
    CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW, QueryServiceStatusEx,
    SC_MANAGER_CONNECT, SC_STATUS_PROCESS_INFO, SERVICE_CONTROL_STOP, SERVICE_QUERY_STATUS,
    SERVICE_RUNNING, SERVICE_START, SERVICE_STATUS, SERVICE_STATUS_PROCESS, SERVICE_STOP,
    SERVICE_STOPPED, StartServiceW,
};
#[cfg(windows)]
use zeroize::Zeroize;

enum Action {
    Status { json: bool },
    Configure,
    Restart,
}

fn usage() -> &'static str {
    "Usage:\n  gnx status [--json]\n  gnx configure\n  gnx restart"
}

fn parse_args() -> Result<Action, ()> {
    let mut args = env::args_os().skip(1);
    match (args.next(), args.next(), args.next()) {
        (Some(command), None, None) if command == "status" => Ok(Action::Status { json: false }),
        (Some(command), Some(format), None) if command == "status" && format == "--json" => {
            Ok(Action::Status { json: true })
        }
        (Some(command), None, None) if command == "configure" => Ok(Action::Configure),
        (Some(command), None, None) if command == "restart" => Ok(Action::Restart),
        _ => Err(()),
    }
}

#[cfg(windows)]
fn run(action: Action) -> Result<(), String> {
    match action {
        Action::Status { json } => {
            let status = pipe::status()?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string(&status)
                        .map_err(|e| format!("cannot encode status: {e}"))?
                );
            } else {
                print_human(&status);
            }
        }
        Action::Configure => {
            let configuration = collect_configuration()?;
            let response = pipe::configure(configuration)?;
            if !response.accepted {
                return Err(format!(
                    "{}: {}",
                    response
                        .error_code
                        .as_deref()
                        .unwrap_or("CONFIGURATION_REJECTED"),
                    response
                        .message
                        .as_deref()
                        .unwrap_or("configuration was rejected")
                ));
            }
            println!("configuration accepted: {}", response.stage);
        }
        Action::Restart => {
            restart_service()?;
            println!("Quetzalcoatl service restarted");
        }
    }
    Ok(())
}

#[cfg(windows)]
fn restart_service() -> Result<(), String> {
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

#[cfg(windows)]
fn query_service_status(
    service: windows_sys::Win32::System::Services::SC_HANDLE,
) -> Result<SERVICE_STATUS_PROCESS, String> {
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

#[cfg(windows)]
fn wait_for_service_state(
    service: windows_sys::Win32::System::Services::SC_HANDLE,
    expected: u32,
) -> Result<(), String> {
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

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

#[cfg(windows)]
fn last_win32(operation: &str) -> String {
    unsafe { format!("{operation} (Win32 {})", GetLastError()) }
}

#[cfg(windows)]
struct OwnedServiceHandle(windows_sys::Win32::System::Services::SC_HANDLE);

#[cfg(windows)]
impl Drop for OwnedServiceHandle {
    fn drop(&mut self) {
        unsafe { CloseServiceHandle(self.0) };
    }
}

#[cfg(not(windows))]
fn run(_action: Action) -> Result<(), String> {
    Err("gnx requires Windows".into())
}

#[cfg(windows)]
fn collect_configuration() -> Result<InstallerConfiguration, String> {
    let tailnet = read_public("Tailnet DNS name (example: tetra-balance.ts.net): ")?
        .trim()
        .to_ascii_lowercase();
    let auth_key = read_secret("Tailscale auth_key: ")?;
    let pve_root_password = read_secret("New PVE root password: ")?;
    let confirmation = read_secret("Confirm PVE root password: ")?;
    if pve_root_password.as_str() != confirmation.as_str() {
        return Err("PVE root password confirmation does not match".into());
    }
    Ok(InstallerConfiguration {
        tailnet,
        auth_key: auth_key.into_inner(),
        pve_root_password: pve_root_password.into_inner(),
    })
}

#[cfg(windows)]
fn read_public(prompt: &str) -> Result<String, String> {
    print!("{prompt}");
    io::stdout()
        .flush()
        .map_err(|error| format!("cannot write prompt: {error}"))?;
    read_console_line()
}

#[cfg(windows)]
fn read_secret(prompt: &str) -> Result<SecretInput, String> {
    print!("{prompt}");
    io::stdout()
        .flush()
        .map_err(|error| format!("cannot write prompt: {error}"))?;
    // Safety: STD_INPUT_HANDLE is a process-owned pseudo handle.
    let input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    if input.is_null() || input == INVALID_HANDLE_VALUE {
        return Err("secret input requires an interactive Windows console".into());
    }
    let mut mode = 0u32;
    // Safety: input is the standard console handle and mode points to writable memory.
    if unsafe { GetConsoleMode(input, &mut mode) } == 0 {
        return Err("secret input requires an interactive Windows console".into());
    }
    // Safety: input is a console handle and all original mode bits except echo are retained.
    if unsafe { SetConsoleMode(input, mode & !ENABLE_ECHO_INPUT) } == 0 {
        return Err("cannot disable console echo for secret input".into());
    }
    let guard = ConsoleEchoGuard { input, mode };
    let result = read_console_line();
    drop(guard);
    println!();
    result.map(SecretInput::new)
}

#[cfg(windows)]
fn read_console_line() -> Result<String, String> {
    let mut value = String::new();
    let read = match io::stdin().read_line(&mut value) {
        Ok(read) => read,
        Err(error) => {
            value.zeroize();
            return Err(format!("cannot read console input: {error}"));
        }
    };
    if read == 0 {
        value.zeroize();
        return Err("console input ended before configuration was complete".into());
    }
    while value.ends_with(['\r', '\n']) {
        value.pop();
    }
    Ok(value)
}

#[cfg(windows)]
struct ConsoleEchoGuard {
    input: HANDLE,
    mode: u32,
}

#[cfg(windows)]
struct SecretInput(Option<String>);

#[cfg(windows)]
impl SecretInput {
    fn new(value: String) -> Self {
        Self(Some(value))
    }

    fn as_str(&self) -> &str {
        self.0.as_deref().unwrap_or_default()
    }

    fn into_inner(mut self) -> String {
        self.0.take().unwrap_or_default()
    }
}

#[cfg(windows)]
impl Drop for SecretInput {
    fn drop(&mut self) {
        if let Some(value) = self.0.as_mut() {
            value.zeroize();
        }
    }
}

#[cfg(windows)]
impl Drop for ConsoleEchoGuard {
    fn drop(&mut self) {
        // Safety: input remains the process standard console handle for this short-lived guard.
        unsafe { SetConsoleMode(self.input, self.mode) };
    }
}

fn print_human(status: &StatusResponse) {
    println!("overall: {}", status.overall);
    println!("stage: {}", status.stage);
    println!("role: {}", status.role.as_deref().unwrap_or("not_resolved"));
    println!("service: {}", status.components.service);
    println!("wsl: {}", status.components.wsl);
    println!("podman_machine: {}", status.components.podman_machine);
    println!("kvm: {}", status.components.kvm);
    if let Some(error) = &status.last_error {
        println!("last_error: {error}");
    }
}

fn main() {
    let action = match parse_args() {
        Ok(action) => action,
        Err(()) => {
            eprintln!("{}", usage());
            std::process::exit(64);
        }
    };
    if let Err(error) = run(action) {
        eprintln!("gnx: {error}");
        std::process::exit(1);
    }
}
