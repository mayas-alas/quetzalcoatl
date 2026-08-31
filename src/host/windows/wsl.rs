use std::time::Duration;

use crate::error::GnxError;
use crate::process::CommandSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WslOutcome {
    Ready,
    RebootRequired,
}

pub fn ensure() -> Result<WslOutcome, GnxError> {
    if is_ready() {
        return Ok(WslOutcome::Ready);
    }

    let output = CommandSpec::new(wsl_executable())
        .args(["--install", "--no-distribution", "--web-download"])
        .timeout(Duration::from_secs(1800))
        .run("wsl_install")?;
    if !output.success() && output.exit_code != Some(3010) {
        return Err(GnxError::new(
            "HOST_WSL_INSTALL_FAILED",
            "host",
            "wsl_install",
            output.stderr,
            "Revise virtualización de hardware y Windows Update.",
            true,
            13,
        ));
    }

    if is_ready() {
        Ok(WslOutcome::Ready)
    } else {
        Ok(WslOutcome::RebootRequired)
    }
}

pub fn is_ready() -> bool {
    CommandSpec::new(wsl_executable())
        .arg("--status")
        .timeout(Duration::from_secs(30))
        .run("wsl_status")
        .is_ok_and(|output| output.success())
}

fn wsl_executable() -> &'static str {
    r"C:\Windows\System32\wsl.exe"
}
