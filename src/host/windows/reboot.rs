use std::path::Path;

use crate::error::GnxError;
use crate::process::CommandSpec;

pub fn register_resume(executable: &Path) -> Result<(), GnxError> {
    let command = format!("\"{}\" __resume", executable.display());
    CommandSpec::new(r"C:\Windows\System32\reg.exe")
        .args([
            "ADD",
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce",
            "/v",
            "QuetzalcoatlNext",
            "/t",
            "REG_SZ",
            "/d",
        ])
        .arg(command)
        .arg("/f")
        .run_checked("register_resume")?;
    Ok(())
}

pub fn unregister_resume() -> Result<(), GnxError> {
    let output = CommandSpec::new(r"C:\Windows\System32\reg.exe")
        .args([
            "DELETE",
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce",
            "/v",
            "QuetzalcoatlNext",
            "/f",
        ])
        .run("unregister_resume")?;
    if output.success() || output.exit_code == Some(1) {
        Ok(())
    } else {
        Err(GnxError::process(
            "unregister_resume",
            Path::new(r"C:\Windows\System32\reg.exe"),
            output.stderr,
            true,
        ))
    }
}
