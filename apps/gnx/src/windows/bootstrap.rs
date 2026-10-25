use super::{
    identity::Identity,
    security::{self, denied},
};
use crate::protocol::{ACCOUNT, DISTRO};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{ERROR_ALREADY_EXISTS, GetLastError, LocalFree},
    NetworkManagement::NetManagement::{
        NERR_UserNotFound, NetApiBufferFree, NetUserAdd, NetUserGetInfo, UF_DONT_EXPIRE_PASSWD,
        UF_NORMAL_ACCOUNT, UF_PASSWD_CANT_CHANGE, UF_SCRIPT, USER_INFO_1, USER_PRIV_USER,
    },
    Security::{
        Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1},
        Cryptography::{
            CRYPT_INTEGER_BLOB, CRYPTPROTECT_LOCAL_MACHINE, CRYPTPROTECT_UI_FORBIDDEN,
            CryptProtectData,
        },
        LookupAccountNameW, SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::CreateDirectoryW,
    System::Registry::{
        HKEY_USERS, KEY_READ, RRF_RT_REG_SZ, RegCloseKey, RegEnumKeyExW, RegGetValueW,
        RegOpenKeyExW,
    },
};
use zeroize::Zeroizing;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
const PRODUCT: &[u8] = b"GnXCore:7a34f3dc-ea24-4ddf-8adb-5817bf28bd21";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Account {
    version: u16,
    sid: String,
}

fn private_root() -> io::Result<PathBuf> {
    let root = security::state_dir()?;
    let mut descriptor = null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide("O:SYG:SYD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)").as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let attrs = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let created = unsafe { CreateDirectoryW(wide(&root.to_string_lossy()).as_ptr(), &attrs) };
    let error = unsafe { GetLastError() };
    unsafe { LocalFree(descriptor) };
    if created == 0 && error != ERROR_ALREADY_EXISTS {
        return Err(io::Error::from_raw_os_error(error as i32));
    }
    let _guard = security::private_directory()?;
    if created != 0 {
        write_new(&root.join("owner"), PRODUCT)?;
    }
    if security::read_private("owner", 128)? != PRODUCT {
        return Err(denied("estado existente no pertenece al producto nuevo"));
    }
    Ok(root)
}

fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn account_sid() -> io::Result<String> {
    let mut sid_len = 0;
    let mut domain_len = 0;
    let mut kind = 0;
    unsafe {
        LookupAccountNameW(
            null(),
            wide(ACCOUNT).as_ptr(),
            null_mut(),
            &mut sid_len,
            null_mut(),
            &mut domain_len,
            &mut kind,
        )
    };
    if sid_len == 0 || sid_len > 1024 || domain_len > 32768 {
        return Err(denied("SID dedicado no disponible"));
    }
    let mut sid = vec![0u64; (sid_len as usize).div_ceil(8)];
    let mut domain = vec![0u16; domain_len as usize];
    if unsafe {
        LookupAccountNameW(
            null(),
            wide(ACCOUNT).as_ptr(),
            sid.as_mut_ptr().cast(),
            &mut sid_len,
            domain.as_mut_ptr(),
            &mut domain_len,
            &mut kind,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    security::sid_text(sid.as_mut_ptr().cast())
}

fn ensure_account(root: &Path) -> io::Result<String> {
    let mut existing = null_mut();
    let status = unsafe { NetUserGetInfo(null(), wide(ACCOUNT).as_ptr(), 1, &mut existing) };
    if status == 0 {
        unsafe { NetApiBufferFree(existing.cast()) };
        let saved: Account = serde_json::from_slice(&security::read_private("account.json", 4096)?)
            .map_err(|_| denied("propiedad de cuenta inválida"))?;
        let actual = account_sid()?;
        if saved.version != 1 || actual != saved.sid {
            return Err(denied("cuenta existente ajena; no se adopta"));
        }
        return Ok(actual);
    }
    if status != NERR_UserNotFound {
        return Err(io::Error::other("no se pudo comprobar la cuenta dedicada"));
    }
    let mut random = Zeroizing::new([0u8; 32]);
    security::random(random.as_mut())?;
    let mut password = Zeroizing::new(String::from("GnX!"));
    use std::fmt::Write;
    for b in random.iter() {
        write!(&mut *password, "{b:02x}").map_err(io::Error::other)?;
    }
    let mut plain = Zeroizing::new(password.as_bytes().to_vec());
    let input = CRYPT_INTEGER_BLOB {
        cbData: plain.len() as u32,
        pbData: plain.as_mut_ptr(),
    };
    let mut output: CRYPT_INTEGER_BLOB = unsafe { std::mem::zeroed() };
    if unsafe {
        CryptProtectData(
            &input,
            null(),
            null(),
            null(),
            null(),
            CRYPTPROTECT_LOCAL_MACHINE | CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let blob = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) };
    let result = write_new(&root.join("account.dpapi"), blob);
    unsafe { LocalFree(output.pbData.cast()) };
    result?;
    let mut name = wide(ACCOUNT);
    let mut secret = Zeroizing::new(wide(&password));
    let info = USER_INFO_1 {
        usri1_name: name.as_mut_ptr(),
        usri1_password: secret.as_mut_ptr(),
        usri1_password_age: 0,
        usri1_priv: USER_PRIV_USER,
        usri1_home_dir: null_mut(),
        usri1_comment: null_mut(),
        usri1_flags: UF_NORMAL_ACCOUNT | UF_SCRIPT | UF_DONT_EXPIRE_PASSWD | UF_PASSWD_CANT_CHANGE,
        usri1_script_path: null_mut(),
    };
    if unsafe { NetUserAdd(null(), 1, (&info as *const USER_INFO_1).cast(), null_mut()) } != 0 {
        return Err(denied("no se pudo crear la cuenta dedicada"));
    }
    let sid = account_sid()?;
    write_new(
        &root.join("account.json"),
        &serde_json::to_vec(&Account {
            version: 1,
            sid: sid.clone(),
        })
        .map_err(io::Error::other)?,
    )?;
    Ok(sid)
}

fn registration(sid: &str, expected: &Path) -> io::Result<Option<String>> {
    let mut root = null_mut();
    if unsafe {
        RegOpenKeyExW(
            HKEY_USERS,
            wide(&format!(
                r"{sid}\Software\Microsoft\Windows\CurrentVersion\Lxss"
            ))
            .as_ptr(),
            0,
            KEY_READ,
            &mut root,
        )
    } != 0
    {
        return Ok(None);
    }
    let result = (|| {
        for index in 0..256 {
            let mut name = [0u16; 256];
            let mut len = name.len() as u32;
            let code = unsafe {
                RegEnumKeyExW(
                    root,
                    index,
                    name.as_mut_ptr(),
                    &mut len,
                    null(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                )
            };
            if code == 259 {
                break;
            }
            if code != 0 {
                return Err(io::Error::from_raw_os_error(code as i32));
            }
            let key_name = String::from_utf16_lossy(&name[..len as usize]);
            let read = |value: &str| -> io::Result<String> {
                let mut data = [0u16; 32768];
                let mut size = std::mem::size_of_val(&data) as u32;
                let code = unsafe {
                    RegGetValueW(
                        root,
                        wide(&key_name).as_ptr(),
                        wide(value).as_ptr(),
                        RRF_RT_REG_SZ,
                        null_mut(),
                        data.as_mut_ptr().cast(),
                        &mut size,
                    )
                };
                if code != 0 {
                    return Err(io::Error::from_raw_os_error(code as i32));
                }
                Ok(String::from_utf16_lossy(
                    &data[..data.iter().position(|&c| c == 0).unwrap_or(data.len())],
                ))
            };
            if read("DistributionName")? == DISTRO {
                let base = read("BasePath")?;
                let normalize = |s: &str| {
                    s.trim_start_matches(r"\\?\")
                        .trim_end_matches(['\\', '/'])
                        .to_ascii_lowercase()
                };
                if normalize(&base) != normalize(&expected.to_string_lossy()) {
                    return Err(denied("distro existente ajena; no se adopta"));
                }
                return Ok(Some(key_name));
            }
        }
        Ok(None)
    })();
    unsafe { RegCloseKey(root) };
    result
}

fn linux_path(path: &Path) -> io::Result<String> {
    let value = path.to_string_lossy().replace('\\', "/");
    if value.len() < 3 || value.as_bytes()[1] != b':' {
        return Err(io::Error::other("payload debe estar en unidad local"));
    }
    Ok(format!(
        "/mnt/{}/{}",
        value[..1].to_ascii_lowercase(),
        &value[3..]
    ))
}

pub fn prepare() -> io::Result<()> {
    if security::read_private("prepared", 16).is_ok_and(|b| b == b"1") {
        return Ok(());
    }
    security::require_system()?;
    let package = std::env::current_exe()?
        .parent()
        .ok_or_else(|| io::Error::other("ruta de instalación inválida"))?
        .join("payload");
    crate::package::verify(&package)?;
    let root = private_root()?;
    let sid = ensure_account(&root)?;
    let identity = Identity::for_setup(&sid)?;
    let wslconfig = identity.profile_dir().join(".wslconfig");
    if !wslconfig.exists() {
        write_new(
            &wslconfig,
            &fs::read(package.join("config/windows-wslconfig"))?,
        )?;
    }
    let destination = identity.profile_dir().join(r"AppData\Local\GnX Core\wsl");
    if registration(&sid, &destination)?.is_none() {
        let parent = destination
            .parent()
            .ok_or_else(|| io::Error::other("ruta WSL inválida"))?;
        fs::create_dir_all(parent)?;
        // This parent is inside the SID-verified, dedicated profile. No host WSL.
        let result = identity.run(
            &[
                "--import",
                DISTRO,
                &destination.to_string_lossy(),
                &package.join("rootfs.tar").to_string_lossy(),
                "--version",
                "2",
            ],
            Duration::from_secs(600),
        )?;
        if result.code != 0 {
            return Err(io::Error::other("importación WSL no confirmada"));
        }
    }
    let guid = registration(&sid, &destination)?
        .ok_or_else(|| io::Error::other("registro WSL no confirmado"))?;
    let payload = linux_path(&package)?;
    let result = identity.run(
        &[
            "--distribution",
            DISTRO,
            "--exec",
            "/usr/bin/install",
            "-m",
            "0644",
            &format!("{payload}/config/wsl.conf"),
            "/etc/wsl.conf",
        ],
        Duration::from_secs(60),
    )?;
    if result.code != 0 {
        return Err(io::Error::other("configuración WSL no confirmada"));
    }
    let result = identity.run(&["--terminate", DISTRO], Duration::from_secs(30))?;
    if result.code != 0 {
        return Err(io::Error::other("reinicio de WSL propia no confirmado"));
    }
    let result = identity.run(
        &[
            "--distribution",
            DISTRO,
            "--exec",
            "/usr/bin/bash",
            &format!("{payload}/scripts/install.run"),
            &payload,
        ],
        Duration::from_secs(600),
    )?;
    if result.code != 0 {
        return Err(io::Error::other(
            "preparación Linux no confirmada; consultar journalctl",
        ));
    }
    let ownership = serde_json::json!({"version":1,"sid":sid,"distribution_id":guid});
    let identity_file = root.join("identity.json");
    if !identity_file.exists() {
        write_new(
            &identity_file,
            &serde_json::to_vec(&ownership).map_err(io::Error::other)?,
        )?;
    }
    super::access::publish(&identity)?;
    if !root.join("intent").exists() {
        write_new(&root.join("intent"), b"running")?;
    }
    write_new(&root.join("prepared"), b"1")?;
    Ok(())
}
