use crate::protocol::invalid;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    ffi::c_void,
    io::{self, Write},
    ptr::{null, null_mut},
    time::{Duration, Instant},
};
use windows_sys::Win32::{Networking::WinHttp::*, System::Registry::*};

pub(super) fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

pub(super) fn machine(key: &str, name: &str) -> io::Result<Option<String>> {
    let mut data = [0u16; 2048];
    let mut size = std::mem::size_of_val(&data) as u32;
    let code = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            wide(key).as_ptr(),
            wide(name).as_ptr(),
            RRF_RT_REG_SZ | RRF_RT_REG_MULTI_SZ | RRF_SUBKEY_WOW6464KEY,
            null_mut(),
            data.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if code == 2 {
        return Ok(None);
    }
    if code != 0 {
        return Err(io::Error::from_raw_os_error(code as i32));
    }
    let text = String::from_utf16(&data[..(size as usize / 2).min(data.len())])
        .map_err(|_| invalid("configuración de máquina inválida"))?;
    Ok(Some(text.trim_end_matches('\0').to_owned()))
}

pub(super) fn source() -> io::Result<(String, String)> {
    let key = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
    let repo =
        machine(key, "GNX_UPDATE_REPOSITORY")?.unwrap_or_else(|| "mayas-alas/quetzalcoatl".into());
    let asset = machine(key, "GNX_UPDATE_MSI_ASSET")?.unwrap_or_else(|| "GnxCore.msi".into());
    let parts: Vec<_> = repo.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || p.len() > 100
                || *p == "."
                || *p == ".."
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
        || asset.len() > 100
        || !asset.ends_with(".msi")
        || !asset
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        return Err(invalid(
            "origen inválido: usa owner/repo y un nombre MSI, sin URL ni credenciales",
        ));
    }
    Ok((repo, asset))
}

pub(super) fn version(value: &str) -> io::Result<[u32; 3]> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3
        || parts.iter().any(|p| {
            p.is_empty()
                || (p.len() > 1 && p.starts_with('0'))
                || !p.bytes().all(|b| b.is_ascii_digit())
        })
    {
        return Err(invalid("versión de producto inválida"));
    }
    let mut result = [0; 3];
    for (out, part) in result.iter_mut().zip(parts) {
        *out = part
            .parse()
            .map_err(|_| invalid("versión fuera de rango"))?;
    }
    if result[0] > 255 || result[1] > 255 || result[2] > 65535 {
        return Err(invalid("versión no representable en MSI"));
    }
    Ok(result)
}

struct Internet(*mut c_void);
impl Drop for Internet {
    fn drop(&mut self) {
        unsafe {
            WinHttpCloseHandle(self.0);
        }
    }
}
impl Internet {
    fn new(handle: *mut c_void) -> io::Result<Self> {
        if handle.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(handle))
        }
    }
    fn option(&self, option: u32, value: u32) -> io::Result<()> {
        if unsafe { WinHttpSetOption(self.0, option, (&value as *const u32).cast(), 4) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

// Redirects are followed manually: no auth, cookies, HTTP downgrade or arbitrary
// hosts. Signed GitHub CDN query strings stay in memory and never enter logs.
pub(super) fn transfer(
    url: &str,
    output: &mut impl Write,
    maximum: u64,
) -> io::Result<(u64, String)> {
    let session = Internet::new(unsafe {
        WinHttpOpen(
            wide("GnXCore-update/1").as_ptr(),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            null(),
            null(),
            0,
        )
    })?;
    if unsafe { WinHttpSetTimeouts(session.0, 15000, 15000, 30000, 30000) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut url = url.to_owned();
    let started = Instant::now();
    for _ in 0..6 {
        let (host, path) = url
            .strip_prefix("https://")
            .and_then(|s| s.split_once('/'))
            .ok_or_else(|| invalid("descarga requiere HTTPS"))?;
        if !matches!(
            host,
            "api.github.com"
                | "github.com"
                | "release-assets.githubusercontent.com"
                | "objects.githubusercontent.com"
        ) || url.len() > 32768
            || url.chars().any(char::is_control)
            || url.contains('#')
            || url.contains('\\')
        {
            return Err(invalid("redirección de descarga no autorizada"));
        }
        let connection =
            Internet::new(unsafe { WinHttpConnect(session.0, wide(host).as_ptr(), 443, 0) })?;
        let request = Internet::new(unsafe {
            WinHttpOpenRequest(
                connection.0,
                wide("GET").as_ptr(),
                wide(&format!("/{path}")).as_ptr(),
                null(),
                null(),
                null(),
                WINHTTP_FLAG_SECURE,
            )
        })?;
        request.option(
            WINHTTP_OPTION_DISABLE_FEATURE,
            WINHTTP_DISABLE_REDIRECTS | WINHTTP_DISABLE_COOKIES,
        )?;
        request.option(WINHTTP_OPTION_ENABLE_FEATURE, WINHTTP_ENABLE_SSL_REVOCATION)?;
        request.option(
            WINHTTP_OPTION_AUTOLOGON_POLICY,
            WINHTTP_AUTOLOGON_SECURITY_LEVEL_HIGH,
        )?;
        let headers =
            wide("Accept: application/vnd.github+json\r\nX-GitHub-Api-Version: 2022-11-28\r\n");
        if unsafe { WinHttpSendRequest(request.0, headers.as_ptr(), u32::MAX, null(), 0, 0, 0) }
            == 0
            || unsafe { WinHttpReceiveResponse(request.0, null_mut()) } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let mut status = 0u32;
        let mut size = 4;
        if unsafe {
            WinHttpQueryHeaders(
                request.0,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                null(),
                (&mut status as *mut u32).cast(),
                &mut size,
                null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if matches!(status, 301 | 302 | 303 | 307 | 308) {
            let mut location = [0u16; 16384];
            let mut size = std::mem::size_of_val(&location) as u32;
            if unsafe {
                WinHttpQueryHeaders(
                    request.0,
                    WINHTTP_QUERY_LOCATION,
                    null(),
                    location.as_mut_ptr().cast(),
                    &mut size,
                    null_mut(),
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            url = String::from_utf16(
                &location[..location
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(location.len())],
            )
            .map_err(|_| invalid("redirección inválida"))?;
            continue;
        }
        if status != 200 {
            return Err(io::Error::other(match status {
                401 | 403 | 404 => {
                    "origen privado, inexistente o no accesible públicamente; no se instala ni se solicita autenticación"
                }
                429 => "GitHub limita las consultas; esperar antes de volver a comprobar",
                _ => "GitHub no confirmó la descarga; no se instala nada",
            }));
        }
        let mut total = 0u64;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            if started.elapsed() > Duration::from_secs(1800) {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "descarga agotó su plazo; instalador no ejecutado",
                ));
            }
            let mut read = 0;
            if unsafe {
                WinHttpReadData(
                    request.0,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32,
                    &mut read,
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            if read == 0 {
                break;
            }
            total += u64::from(read);
            if total > maximum {
                return Err(invalid("descarga excede el tamaño permitido"));
            }
            output.write_all(&buffer[..read as usize])?;
            hash.update(&buffer[..read as usize]);
        }
        return Ok((total, format!("{:x}", hash.finalize())));
    }
    Err(invalid("demasiadas redirecciones; no se instala"))
}

fn bytes(url: &str, maximum: u64) -> io::Result<Vec<u8>> {
    let mut result = Vec::new();
    transfer(url, &mut result, maximum)?;
    Ok(result)
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Asset {
    pub name: String,
    pub size: u64,
    pub digest: Option<String>,
    pub browser_download_url: String,
    pub state: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Release {
    pub tag_name: String,
    pub draft: bool,
    pub assets: Vec<Asset>,
}

impl Release {
    pub fn number(&self) -> io::Result<[u32; 3]> {
        version(
            self.tag_name
                .strip_prefix('v')
                .ok_or_else(|| invalid("tag de producto inválido"))?,
        )
    }
    pub fn asset(&self, name: &str, repo: &str) -> io::Result<&Asset> {
        let values: Vec<_> = self.assets.iter().filter(|a| a.name == name).collect();
        if values.len() != 1 {
            return Err(invalid("release sin instalador o hashes únicos"));
        }
        let asset = values[0];
        let expected = format!(
            "https://github.com/{repo}/releases/download/{}/{name}",
            self.tag_name
        );
        if asset.state != "uploaded"
            || asset.size == 0
            || asset.size > 8 * 1024 * 1024 * 1024
            || asset.browser_download_url != expected
        {
            return Err(invalid("asset de release inválido"));
        }
        Ok(asset)
    }
    pub fn hashes(
        &self,
        repo: &str,
        msi: &str,
    ) -> io::Result<std::collections::BTreeMap<String, String>> {
        let asset = self.asset("SHA256SUMS.txt", repo)?;
        let mut contents = Vec::new();
        let (length, digest) = transfer(&asset.browser_download_url, &mut contents, 4096)?;
        if length != asset.size
            || asset.digest.as_deref() != Some(format!("sha256:{digest}").as_str())
        {
            return Err(invalid("archivo de hashes difiere del asset publicado"));
        }
        let text = String::from_utf8(contents).map_err(|_| invalid("hashes no son UTF-8"))?;
        let mut hashes: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        for line in text.lines() {
            let (hash, name) = line
                .split_once("  ")
                .ok_or_else(|| invalid("formato de hashes inválido"))?;
            if hash.len() != 64
                || !hash
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || ![msi, "GnxCoreSetup.exe"].contains(&name)
                || hashes.insert(name.into(), hash.into()).is_some()
            {
                return Err(invalid("hashes de release inválidos"));
            }
        }
        if hashes.len() != 2 {
            return Err(invalid("faltan hashes de los dos instaladores"));
        }
        for (name, hash) in &hashes {
            if self.asset(name, repo)?.digest.as_deref() != Some(format!("sha256:{hash}").as_str())
            {
                return Err(invalid("digest GitHub difiere de SHA256SUMS"));
            }
        }
        Ok(hashes)
    }
}

pub(super) fn latest(repo: &str) -> io::Result<Release> {
    let metadata: serde_json::Value = serde_json::from_slice(&bytes(
        &format!("https://api.github.com/repos/{repo}"),
        1024 * 1024,
    )?)
    .map_err(|_| invalid("metadatos de origen inválidos"))?;
    if metadata["private"].as_bool() != Some(false)
        || !metadata["full_name"]
            .as_str()
            .is_some_and(|s| s.eq_ignore_ascii_case(repo))
    {
        return Err(invalid(
            "el origen debe ser el repositorio público configurado",
        ));
    }
    let mut selected: Option<Release> = None;
    for page in 1..=4 {
        let releases: Vec<Release> = serde_json::from_slice(&bytes(
            &format!("https://api.github.com/repos/{repo}/releases?per_page=100&page={page}"),
            4 * 1024 * 1024,
        )?)
        .map_err(|_| invalid("inventario GitHub inválido"))?;
        let finished = releases.len() < 100;
        for release in releases {
            if release.draft || release.number().is_err() {
                continue;
            }
            if selected
                .as_ref()
                .is_none_or(|old| release.number().ok() > old.number().ok())
            {
                selected = Some(release);
            }
        }
        if finished {
            return selected.ok_or_else(|| {
                invalid("sin releases públicas de producto, incluidas pre-releases")
            });
        }
    }
    Err(invalid(
        "inventario de releases excede el límite; selección no confirmada",
    ))
}

// This is discovery only. It must not claim installation, create an updater,
// or fall back to a different repository when the configured one is unavailable.
pub(super) fn check() -> io::Result<String> {
    let (repository, msi) = source()?;
    let release = latest(&repository)?;
    let available = release.number()?;
    let current = version(env!("CARGO_PKG_VERSION"))?;
    release.hashes(&repository, &msi)?;
    let state = match available.cmp(&current) {
        std::cmp::Ordering::Greater => "versión más reciente disponible",
        std::cmp::Ordering::Equal => "la CLI coincide con la versión publicada",
        std::cmp::Ordering::Less => "la CLI es posterior; no se propone downgrade",
    };
    Ok(format!(
        "Origen público: {repository}; release {}: {state}. Metadatos y hashes de los dos instaladores coherentes; bytes del MSI/EXE, instalación y salud no comprobados.",
        release.tag_name,
    ))
}
