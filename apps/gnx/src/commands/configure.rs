use std::io::{self, Write};

use gnx_contracts::{InstallerConfiguration, PlatformConfiguration};
use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Console::{
    ENABLE_ECHO_INPUT, GetConsoleMode, GetStdHandle, STD_INPUT_HANDLE, SetConsoleMode,
};
use zeroize::Zeroize;

use crate::client;
use crate::error::CliResult;

pub(crate) fn run() -> CliResult<()> {
    let configuration = collect_configuration()?;
    let response = client::configure(configuration)?;
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
    Ok(())
}

pub(crate) fn run_platform() -> CliResult<()> {
    let auth_key = read_secret("Tailscale service enrollment auth key: ")?;
    let response = client::configure_platform(PlatformConfiguration::new(auth_key.into_inner()))?;
    if !response.accepted {
        return Err(format!(
            "{}: {}",
            response
                .error_code
                .as_deref()
                .unwrap_or("PLATFORM_CONFIGURATION_REJECTED"),
            response
                .message
                .as_deref()
                .unwrap_or("platform configuration was rejected")
        ));
    }
    println!("platform configuration accepted: {}", response.stage);
    Ok(())
}

fn collect_configuration() -> CliResult<InstallerConfiguration> {
    let tailnet = read_public("Tailnet DNS name (example: your-tailnet.ts.net): ")?
        .trim()
        .to_ascii_lowercase();
    let auth_key = read_secret("Tailscale auth_key: ")?;
    let pve_root_password = read_secret("New PVE root password: ")?;
    let confirmation = read_secret("Confirm PVE root password: ")?;
    if pve_root_password.as_str() != confirmation.as_str() {
        return Err("PVE root password confirmation does not match".into());
    }
    Ok(InstallerConfiguration {
        tailnet: tailnet.into(),
        auth_key: auth_key.into_inner(),
        pve_root_password: pve_root_password.into_inner(),
    })
}

fn read_public(prompt: &str) -> CliResult<String> {
    print!("{prompt}");
    io::stdout()
        .flush()
        .map_err(|error| format!("cannot write prompt: {error}"))?;
    read_console_line()
}

fn read_secret(prompt: &str) -> CliResult<SecretInput> {
    print!("{prompt}");
    io::stdout()
        .flush()
        .map_err(|error| format!("cannot write prompt: {error}"))?;
    let input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    if input.is_null() || input == INVALID_HANDLE_VALUE {
        return Err("secret input requires an interactive Windows console".into());
    }
    let mut mode = 0u32;
    if unsafe { GetConsoleMode(input, &mut mode) } == 0 {
        return Err("secret input requires an interactive Windows console".into());
    }
    if unsafe { SetConsoleMode(input, mode & !ENABLE_ECHO_INPUT) } == 0 {
        return Err("cannot disable console echo for secret input".into());
    }
    let guard = ConsoleEchoGuard { input, mode };
    let result = read_console_line();
    drop(guard);
    println!();
    result.map(SecretInput::new)
}

fn read_console_line() -> CliResult<String> {
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

struct ConsoleEchoGuard {
    input: HANDLE,
    mode: u32,
}

impl Drop for ConsoleEchoGuard {
    fn drop(&mut self) {
        unsafe { SetConsoleMode(self.input, self.mode) };
    }
}

struct SecretInput(Option<String>);

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

impl Drop for SecretInput {
    fn drop(&mut self) {
        if let Some(value) = self.0.as_mut() {
            value.zeroize();
        }
    }
}
