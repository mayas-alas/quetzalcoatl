use crate::report::{Report, State};
use std::process::{Command, Stdio};
pub fn invoke(op: &str, intent: &str) -> Report {
    invoke_secret(op, intent, None)
}
pub fn invoke_secret(
    op: &str,
    intent: &str,
    secret: Option<&crate::domain::secret::Secret>,
) -> Report {
    let fail = || {
        Report::new(
            op,
            State::ActionRequired,
            "WSL_RUNTIME_UNAVAILABLE",
            Some("Verify the service-owned GNX runtime and installed Linux release."),
        )
    };
    let Ok(frame) = crate::wire::encode(op, intent, secret) else {
        return fail();
    };
    let result = super::super::process::run(
        "C:\\Windows\\System32\\wsl.exe",
        &[
            "--distribution",
            "GNX",
            "--user",
            "root",
            "--exec",
            "/usr/bin/timeout",
            "600",
            "/usr/local/bin/gnx",
            op,
            "--broker",
        ],
        Some(&frame),
        std::time::Duration::from_secs(620),
        1024 * 1024,
    );
    match result {
        Ok(o) => match serde_json::from_slice::<Report>(&o.stdout) {
            Ok(r) if r.schema == 1 && r.operation == op && r.exit() == o.code => r,
            _ => fail(),
        },
        _ => fail(),
    }
}
pub fn bootstrap() -> Result<(), String> {
    let root = std::path::Path::new("C:\\ProgramData\\GNX");
    if !root.join("bundle.tar").exists() {
        return Ok(());
    }
    let wsl = "C:\\Windows\\System32\\wsl.exe";
    if root.join("rootfs.tar").exists() && !root.join("wsl\\ext4.vhdx").exists() {
        let s = Command::new(wsl)
            .args([
                "--import",
                "GNX",
                "C:\\ProgramData\\GNX\\wsl",
                "C:\\ProgramData\\GNX\\rootfs.tar",
                "--version",
                "2",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|_| "WSL_IMPORT_FAILED")?;
        if !s.success() {
            return Err("WSL_IMPORT_FAILED".into());
        }
    }
    let mut child = Command::new(wsl)
        .args([
            "-d", "GNX", "-u", "root", "--exec", "/bin/tar", "-xf", "-", "-C", "/",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "BUNDLE_INSTALL_FAILED")?;
    let mut f = std::fs::File::open(root.join("bundle.tar")).map_err(|_| "BUNDLE_MISSING")?;
    std::io::copy(&mut f, &mut child.stdin.take().unwrap()).map_err(|_| "BUNDLE_STREAM_FAILED")?;
    if !child.wait().map_err(|_| "BUNDLE_INSTALL_FAILED")?.success() {
        return Err("BUNDLE_INSTALL_FAILED".into());
    }
    let script="set -eu; umask 077; printf '[boot]\\nsystemd=true\\n[automount]\\nenabled=false\\n[interop]\\nenabled=false\\nappendWindowsPath=false\\n' > /etc/wsl.conf; test -f /etc/gnx/gnx.toml || cp /etc/gnx/gnx.example.toml /etc/gnx/gnx.toml; chmod 600 /etc/gnx/gnx.toml; chmod 755 /usr/local/bin/gnx";
    if !Command::new(wsl)
        .args(["-d", "GNX", "-u", "root", "--exec", "/bin/sh", "-c", script])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| "ISOLATION_FAILED")?
        .success()
    {
        return Err("ISOLATION_FAILED".into());
    }
    if !Command::new(wsl)
        .args(["--terminate", "GNX"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| "WSL_RESTART_FAILED")?
        .success()
    {
        return Err("WSL_RESTART_FAILED".into());
    }
    std::fs::remove_file(root.join("bundle.tar")).map_err(|_| "BOOTSTRAP_CLEANUP_FAILED")?;
    if root.join("rootfs.tar").exists() {
        std::fs::remove_file(root.join("rootfs.tar")).map_err(|_| "BOOTSTRAP_CLEANUP_FAILED")?;
    }
    Ok(())
}
