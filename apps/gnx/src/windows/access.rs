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
use windows_sys::Win32::{Foundation::CRYPT_E_NOT_FOUND, Security::Cryptography::*};

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
    let _directory = security::private_directory()?;
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
    trust(false)?;
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
    hosts()?;
    forward(&ip, 443, "portproxy")?;
    forward(&ip, 80, "portproxy-80")?;
    Ok(())
}

fn owned_forward(port: u16, receipt: &str) -> io::Result<Option<String>> {
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
            wide(&format!("127.0.0.1/{port}")).as_ptr(),
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
        let saved = security::read_private(receipt, 256)?;
        if saved != existing.as_bytes() {
            return Err(io::Error::other("portproxy ajeno; no se modifica"));
        }
        Ok(Some(existing))
    } else if code == 2 || code == 3 {
        Ok(None)
    } else {
        Err(io::Error::from_raw_os_error(code as i32))
    }
}

fn forward(ip: &str, port: u16, receipt: &str) -> io::Result<()> {
    let existing = owned_forward(port, receipt)?;
    if existing.as_deref() == Some(&format!("{ip}/{port}")) {
        return Ok(());
    }
    if existing.is_none() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))
            .map_err(|_| io::Error::other(format!("puerto local {port} ocupado")))?;
        drop(listener);
    }
    native(
        "netsh.exe",
        &[
            "interface",
            "portproxy",
            "add",
            "v4tov4",
            "listenaddress=127.0.0.1",
            &format!("listenport={port}"),
            &format!("connectaddress={ip}"),
            &format!("connectport={port}"),
        ],
    )?;
    fs::write(security::state_dir()?.join(receipt), format!("{ip}/{port}"))?;
    Ok(())
}

fn trust(remove: bool) -> io::Result<()> {
    use std::{
        os::windows::ffi::OsStrExt,
        ptr::{null, null_mut},
    };
    let path = security::state_dir()?.join("root.crt");
    let _receipt = security::private_file("root.crt")?;
    let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut context = null_mut();
    if unsafe {
        CryptQueryObject(
            CERT_QUERY_OBJECT_FILE,
            name.as_ptr().cast(),
            CERT_QUERY_CONTENT_FLAG_CERT,
            CERT_QUERY_FORMAT_FLAG_ALL,
            0,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            &mut context,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let certificate = context.cast::<CERT_CONTEXT>();
    let root: Vec<u16> = "Root".encode_utf16().chain(Some(0)).collect();
    let store = unsafe {
        CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            0,
            0,
            CERT_SYSTEM_STORE_LOCAL_MACHINE | CERT_STORE_OPEN_EXISTING_FLAG,
            root.as_ptr().cast(),
        )
    };
    let result = (|| {
        if store.is_null() {
            return Err(io::Error::last_os_error());
        }
        let existing = unsafe {
            CertFindCertificateInStore(
                store,
                X509_ASN_ENCODING,
                0,
                CERT_FIND_EXISTING,
                certificate.cast(),
                null(),
            )
        };
        if existing.is_null() {
            if unsafe { windows_sys::Win32::Foundation::GetLastError() } != CRYPT_E_NOT_FOUND as u32
            {
                return Err(io::Error::last_os_error());
            }
            if !remove
                && unsafe {
                    CertAddCertificateContextToStore(
                        store,
                        certificate,
                        CERT_STORE_ADD_NEW,
                        null_mut(),
                    )
                } == 0
            {
                return Err(io::Error::last_os_error());
            }
            return Ok(());
        }
        let same = unsafe {
            std::slice::from_raw_parts(
                (*existing).pbCertEncoded,
                (*existing).cbCertEncoded as usize,
            ) == std::slice::from_raw_parts(
                (*certificate).pbCertEncoded,
                (*certificate).cbCertEncoded as usize,
            )
        };
        let result = if !same {
            Err(io::Error::other(
                "certificado raíz existente distinto; no se modifica",
            ))
        } else if remove {
            // CertDeleteCertificateFromStore consumes this context, even on failure.
            let removed = unsafe { CertDeleteCertificateFromStore(existing) };
            return if removed != 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            };
        } else {
            Ok(())
        };
        unsafe { CertFreeCertificateContext(existing) };
        result
    })();
    unsafe {
        if !store.is_null() {
            CertCloseStore(store, 0);
        }
        CertFreeCertificateContext(certificate);
    }
    result
}

pub fn remove() -> io::Result<()> {
    let _directory = security::private_directory()?;
    // Validate ownership before the first removal; never delete by name alone.
    for (port, receipt) in [(443, "portproxy"), (80, "portproxy-80")] {
        owned_forward(port, receipt)?;
    }
    let windows =
        std::env::var_os("SystemRoot").ok_or_else(|| io::Error::other("SystemRoot ausente"))?;
    let path = PathBuf::from(windows).join(r"System32\drivers\etc\hosts");
    let contents = fs::read_to_string(&path)?;
    let marker = "# GnXCore:7a34f3dc-ea24-4ddf-8adb-5817bf28bd21";
    let owned = format!("127.0.0.1 app.gnx compute.gnx {marker}");
    if contents
        .lines()
        .any(|line| line.contains(marker) && line != owned)
    {
        return Err(io::Error::other(
            "entrada hosts propia modificada; no se retira",
        ));
    }
    for (port, receipt) in [(443, "portproxy"), (80, "portproxy-80")] {
        if owned_forward(port, receipt)?.is_some() {
            native(
                "netsh.exe",
                &[
                    "interface",
                    "portproxy",
                    "delete",
                    "v4tov4",
                    "listenaddress=127.0.0.1",
                    &format!("listenport={port}"),
                ],
            )?;
        }
    }
    let retained: String = contents
        .split_inclusive('\n')
        .filter(|line| line.trim_end_matches(['\r', '\n']) != owned)
        .collect();
    if retained != contents {
        fs::write(path, retained)?;
    }
    trust(true)
}
