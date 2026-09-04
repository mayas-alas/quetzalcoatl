use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use crate::{Error, Result};

pub const DATA_ROOT: &str = r"C:\ProgramData\GNX";
pub const DISTRO_NAME: &str = "GNX";
const WSL: &str = r"C:\Windows\System32\wsl.exe";
const WSL_CONF: &str = "[boot]\nsystemd=true\n\n[automount]\nenabled=false\n\n[interop]\nenabled=false\nappendWindowsPath=false\n";

pub fn bootstrap() -> Result<()> {
    let root = Path::new(DATA_ROOT);
    let distro = root.join("wsl");
    let bootstrap = root.join("bootstrap");
    fs::create_dir_all(&bootstrap).map_err(Error::ConfigRead)?;

    if !distro_exists() {
        let rootfs = bootstrap.join("ubuntu.rootfs.tar.gz");
        if !rootfs.is_file() {
            return Err(Error::Operation("WSL_ROOTFS_MISSING"));
        }
        fs::create_dir_all(&distro).map_err(Error::ConfigRead)?;
        let status = Command::new(WSL)
            .args(["--import", DISTRO_NAME])
            .arg(&distro)
            .arg(&rootfs)
            .args(["--version", "2"])
            .status()
            .map_err(Error::Spawn)?;
        if !status.success() {
            return Err(Error::Operation("WSL_IMPORT"));
        }
        write_wsl_conf()?;
        let _ = Command::new(WSL)
            .args(["--terminate", DISTRO_NAME])
            .status();
    }

    ensure_packages()?;
    install_staged_bundle(&bootstrap)?;
    export_public_ca();
    Ok(())
}

pub fn sync_config(config: &[u8]) -> Result<()> {
    if config.len() > 1024 * 1024 {
        return Err(Error::Operation("BROKER_CONFIG_SIZE"));
    }
    checked(
        run_wsl(&["/usr/bin/install", "-d", "-m", "700", "/etc/gnx"], None)?,
        "BROKER_CONFIG_DIR",
    )?;
    checked(
        run_wsl(&["/usr/bin/tee", "/etc/gnx/gnx.toml"], Some(config))?,
        "BROKER_CONFIG_WRITE",
    )?;
    checked(
        run_wsl(&["/usr/bin/chmod", "600", "/etc/gnx/gnx.toml"], None)?,
        "BROKER_CONFIG_MODE",
    )?;
    Ok(())
}

pub fn run_action(action: [&str; 2], secret: Option<&[u8]>) -> Result<Output> {
    let mut args = vec![
        "/usr/bin/env",
        "GNX_BROKER_STDIN=1",
        "/usr/local/bin/gnx",
        "--config",
        "/etc/gnx/gnx.toml",
    ];
    args.extend_from_slice(&action);
    let output = run_wsl(&args, secret)?;
    if action[0] == "controller" && output.status.success() {
        export_public_ca();
    }
    Ok(output)
}

pub fn export_public_ca() {
    let Ok(output) = run_wsl(
        &["/usr/bin/cat", "/var/lib/gnx/controller/public/root.crt"],
        None,
    ) else {
        return;
    };
    if !output.status.success() || !output.stdout.starts_with(b"-----BEGIN CERTIFICATE-----") {
        return;
    }
    let public = Path::new(DATA_ROOT).join("public");
    if fs::create_dir_all(&public).is_ok() {
        let _ = fs::write(public.join("root.crt"), output.stdout);
    }
}

pub fn write_failure(label: &str) {
    let _ = fs::write(Path::new(DATA_ROOT).join("last-error.txt"), label.as_bytes());
}

pub fn clear_failure() {
    let _ = fs::remove_file(Path::new(DATA_ROOT).join("last-error.txt"));
}

fn distro_exists() -> bool {
    run_wsl(&["/usr/bin/true"], None)
        .is_ok_and(|output| output.status.success())
}

fn write_wsl_conf() -> Result<()> {
    checked(
        run_wsl(&["/usr/bin/tee", "/etc/wsl.conf"], Some(WSL_CONF.as_bytes()))?,
        "WSL_CONFIG",
    )
}

fn ensure_packages() -> Result<()> {
    if run_wsl(&["/usr/bin/test", "-x", "/usr/bin/podman"], None)
        .is_ok_and(|output| output.status.success())
    {
        return Ok(());
    }
    checked(
        run_wsl(&["/usr/bin/apt-get", "update"], None)?,
        "WSL_APT_UPDATE",
    )?;
    checked(
        run_wsl(
            &[
                "/usr/bin/env",
                "DEBIAN_FRONTEND=noninteractive",
                "/usr/bin/apt-get",
                "install",
                "-y",
                "podman",
                "openssl",
                "curl",
                "iproute2",
                "ca-certificates",
            ],
            None,
        )?,
        "WSL_APT_INSTALL",
    )
}

fn install_staged_bundle(bootstrap: &Path) -> Result<()> {
    let bundle = bootstrap.join("gnx-linux-bundle.tar");
    if !bundle.is_file() {
        if run_wsl(&["/usr/bin/test", "-x", "/usr/local/bin/gnx"], None)
            .is_ok_and(|output| output.status.success())
        {
            return Ok(());
        }
        return Err(Error::Operation("LINUX_BUNDLE_MISSING"));
    }
    checked(
        run_wsl(
            &["/usr/bin/install", "-d", "-m", "700", "/run/gnx-bootstrap"],
            None,
        )?,
        "LINUX_BUNDLE_DIR",
    )?;
    let data = fs::read(&bundle).map_err(Error::ConfigRead)?;
    checked(
        run_wsl(
            &["/usr/bin/tar", "-xf", "-", "-C", "/run/gnx-bootstrap"],
            Some(&data),
        )?,
        "LINUX_BUNDLE_EXTRACT",
    )?;
    checked(
        run_wsl(
            &[
                "/bin/sh",
                "/run/gnx-bootstrap/install-linux.sh",
                "/run/gnx-bootstrap",
            ],
            None,
        )?,
        "LINUX_INSTALL",
    )?;
    let _ = run_wsl(&["/usr/bin/rm", "-rf", "/run/gnx-bootstrap"], None);
    let _ = fs::remove_file(bundle);
    let _ = fs::remove_file(bootstrap.join("ubuntu.rootfs.tar.gz"));
    Ok(())
}

fn run_wsl(args: &[&str], input: Option<&[u8]>) -> Result<Output> {
    let mut child = Command::new(WSL)
        .args(["-d", DISTRO_NAME, "--user", "root", "--exec"])
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(Error::Spawn)?;
    if let Some(data) = input {
        child
            .stdin
            .take()
            .ok_or(Error::Operation("WSL_STDIN"))?
            .write_all(data)
            .map_err(|_| Error::Operation("WSL_STDIN"))?;
    }
    child.wait_with_output().map_err(Error::Spawn)
}

fn checked(output: Output, operation: &'static str) -> Result<()> {
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::Operation(operation))
    }
}

pub fn data_path(name: &str) -> PathBuf {
    Path::new(DATA_ROOT).join(name)
}
