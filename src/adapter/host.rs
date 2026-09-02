use crate::{
    Error, Result,
    port::{Host, HostState},
};

pub struct WindowsHost;

impl Host for WindowsHost {
    fn inspect(&self) -> Result<HostState> {
        if !cfg!(target_os = "windows") || std::env::consts::ARCH != "x86_64" {
            return Err(Error::HostUnsupported);
        }
        Ok(HostState {
            elevated: is_elevated(),
        })
    }
}

#[cfg(target_os = "windows")]
fn is_elevated() -> bool {
    // Windows exposes the effective administrator membership of the process token.
    unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
}

#[cfg(not(target_os = "windows"))]
fn is_elevated() -> bool {
    false
}
