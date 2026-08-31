use crate::error::GnxError;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "windows")]
pub mod windows;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallOptions {
    pub elevated: bool,
    pub resume: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallOutcome {
    Installed,
    RebootRequired,
    RelaunchedElevated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UninstallOutcome {
    Removed,
    RebootRequired,
    RelaunchedElevated,
}

pub fn install(options: InstallOptions) -> Result<InstallOutcome, GnxError> {
    if std::env::consts::ARCH != "x86_64" {
        return Err(GnxError::unsupported_host(format!(
            "Arquitectura {} no soportada.",
            std::env::consts::ARCH
        )));
    }

    #[cfg(target_os = "windows")]
    {
        windows::install::install(options)
    }

    #[cfg(target_os = "linux")]
    {
        linux::install(options)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = options;
        Err(GnxError::unsupported_host(format!(
            "Sistema {} no soportado.",
            std::env::consts::OS
        )))
    }
}

pub fn uninstall(elevated: bool) -> Result<UninstallOutcome, GnxError> {
    #[cfg(target_os = "windows")]
    {
        windows::install::uninstall(elevated)
    }

    #[cfg(target_os = "linux")]
    {
        linux::uninstall(elevated)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = elevated;
        Err(GnxError::unsupported_host(format!(
            "Sistema {} no soportado.",
            std::env::consts::OS
        )))
    }
}

pub fn running_from_installed_path() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows::install::running_from_installed_path()
    }

    #[cfg(target_os = "linux")]
    {
        linux::running_from_installed_path()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        false
    }
}
