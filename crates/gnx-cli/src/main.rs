#[cfg(windows)]
mod pipe;

use std::env;
#[cfg(windows)]
use std::io::{self, Write};

use gnx_protocol::{InstallerConfiguration, StatusResponse};
#[cfg(windows)]
use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
#[cfg(windows)]
use windows_sys::Win32::System::Console::{
    ENABLE_ECHO_INPUT, GetConsoleMode, GetStdHandle, STD_INPUT_HANDLE, SetConsoleMode,
};
#[cfg(windows)]
use zeroize::Zeroize;

enum Action {
    Status { json: bool },
    Configure,
}

fn usage() -> &'static str {
    "Usage:\n  gnx status [--json]\n  gnx configure"
}

fn parse_args() -> Result<Action, ()> {
    let mut args = env::args_os().skip(1);
    match (args.next(), args.next(), args.next()) {
        (Some(command), None, None) if command == "status" => Ok(Action::Status { json: false }),
        (Some(command), Some(format), None) if command == "status" && format == "--json" => {
            Ok(Action::Status { json: true })
        }
        (Some(command), None, None) if command == "configure" => Ok(Action::Configure),
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
    }
    Ok(())
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
    let install_garage = read_boolean("Install Garage? [y/N]: ")?;
    let install_forgejo = read_boolean("Install Forgejo? [y/N]: ")?;
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
        install_garage,
        install_forgejo,
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
fn read_boolean(prompt: &str) -> Result<bool, String> {
    match read_public(prompt)?.trim().to_ascii_lowercase().as_str() {
        "" | "n" | "no" => Ok(false),
        "y" | "yes" => Ok(true),
        _ => Err("answer y or n".into()),
    }
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
