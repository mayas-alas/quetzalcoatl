use super::{identity::Identity, security};
use crate::protocol::DISTRO;
use std::{
    fs,
    io::{self, Write},
    net::{Ipv4Addr, TcpListener},
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn native(name: &str, args: &[&str]) -> io::Result<()> {
    let windows =
        std::env::var_os("SystemRoot").ok_or_else(|| io::Error::other("SystemRoot ausente"))?;
    let mut child = Command::new(PathBuf::from(windows).join("System32").join(name))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(io::Error::other("Windows no confirmó la publicación local"))
            };
        }
        if start.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "publicación local no confirmada",
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn hosts() -> io::Result<()> {
    let windows =
        std::env::var_os("SystemRoot").ok_or_else(|| io::Error::other("SystemRoot ausente"))?;
    let path = PathBuf::from(windows).join(r"System32\drivers\etc\hosts");
    let contents = fs::read_to_string(&path)?;
    let mut present = std::collections::HashSet::new();
    for line in contents.lines() {
        let mut parts = line.split('#').next().unwrap_or("").split_whitespace();
        let Some(ip) = parts.next() else {
            continue;
        };
        for name in parts {
            if ["app.gnx", "compute.gnx"].contains(&name) {
                if ip != "127.0.0.1" {
                    return Err(io::Error::other(
                        "nombre local existente en conflicto; no se modifica",
                    ));
                }
                present.insert(name);
            }
        }
    }
    if present.len() != 2 {
        let mut file = fs::OpenOptions::new().append(true).open(path)?;
        file.write_all(
            b"\r\n127.0.0.1 app.gnx compute.gnx # GnXCore:7a34f3dc-ea24-4ddf-8adb-5817bf28bd21\r\n",
        )?;
        file.sync_all()?;
    }
    Ok(())
}

pub fn publish(identity: &Identity) -> io::Result<()> {
    let root = security::state_dir()?;
    let output = identity.run(
        &[
            "--distribution",
            DISTRO,
            "--exec",
            "/usr/bin/cat",
            "/var/lib/gnx-core/tls/root.crt",
        ],
        Duration::from_secs(10),
    )?;
    let certificate = output.text();
    if output.code != 0
        || certificate.len() > 16384
        || !certificate.starts_with("-----BEGIN CERTIFICATE-----")
    {
        return Err(io::Error::other("CA pública no confirmada"));
    }
    let path = root.join("root.crt");
    if path.exists() {
        if security::read_private("root.crt", 16384)? != certificate.as_bytes() {
            return Err(io::Error::other(
                "la CA cambió; no se confía silenciosamente",
            ));
        }
    } else {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(certificate.as_bytes())?;
        file.sync_all()?;
    }
    native(
        "certutil.exe",
        &["-addstore", "-f", "Root", &path.to_string_lossy()],
    )?;
    let output = identity.run(
        &[
            "--distribution",
            DISTRO,
            "--exec",
            "/usr/bin/hostname",
            "-I",
        ],
        Duration::from_secs(10),
    )?;
    let ip = output
        .text()
        .split_whitespace()
        .find_map(|s| s.parse::<Ipv4Addr>().ok())
        .filter(|ip| ip.is_private())
        .ok_or_else(|| io::Error::other("IP NAT propia no confirmada"))?
        .to_string();
    if output.code != 0 {
        return Err(io::Error::other("IP WSL no confirmada"));
    }
    // Never overwrite a foreign portproxy slot. Compare native state with our
    // protected receipt before updating the WSL address.
    use windows_sys::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let mut value = [0u16; 256];
    let mut bytes = std::mem::size_of_val(&value) as u32;
    let code = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            wide(r"SYSTEM\CurrentControlSet\Services\PortProxy\v4tov4\tcp").as_ptr(),
            wide("127.0.0.1/443").as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            value.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    if code == 0 {
        let existing = String::from_utf16_lossy(
            &value[..value.iter().position(|&c| c == 0).unwrap_or(value.len())],
        );
        let saved = security::read_private("portproxy", 256)?;
        if saved != existing.as_bytes() {
            return Err(io::Error::other("portproxy ajeno; no se modifica"));
        }
    } else if code == 2 || code == 3 {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 443))
            .map_err(|_| io::Error::other("puerto local 443 ocupado"))?;
        drop(listener);
    } else {
        return Err(io::Error::from_raw_os_error(code as i32));
    }
    hosts()?;
    native(
        "netsh.exe",
        &[
            "interface",
            "portproxy",
            "add",
            "v4tov4",
            "listenaddress=127.0.0.1",
            "listenport=443",
            &format!("connectaddress={ip}"),
            "connectport=443",
        ],
    )?;
    fs::write(root.join("portproxy"), format!("{ip}/443"))?;
    Ok(())
}
