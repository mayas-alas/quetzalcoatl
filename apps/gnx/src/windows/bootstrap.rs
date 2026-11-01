use super::{
    identity::Identity,
    security::{self, denied},
};
use crate::protocol::{ACCOUNT, DISTRO};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    os::windows::io::FromRawHandle,
    path::{Path, PathBuf},
    process::{Command, Stdio},
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
        Authentication::Identity::{
            LSA_HANDLE, LSA_OBJECT_ATTRIBUTES, LSA_UNICODE_STRING, LsaAddAccountRights, LsaClose,
            LsaNtStatusToWinError, LsaOpenPolicy, POLICY_CREATE_ACCOUNT, POLICY_LOOKUP_NAMES,
        },
        Authorization::{
            ConvertStringSecurityDescriptorToSecurityDescriptorW, ConvertStringSidToSidW,
            SDDL_REVISION_1,
        },
        Cryptography::{
            CRYPT_INTEGER_BLOB, CRYPTPROTECT_LOCAL_MACHINE, CRYPTPROTECT_UI_FORBIDDEN,
            CryptProtectData,
        },
        LookupAccountNameW, LookupAccountSidW, SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::CreateDirectoryW,
    System::Registry::{
        HKEY_LOCAL_MACHINE, HKEY_USERS, KEY_READ, KEY_SET_VALUE, KEY_WOW64_64KEY, REG_DWORD,
        REG_OPTION_NON_VOLATILE, RRF_RT_REG_SZ, RegCloseKey, RegCreateKeyExW, RegEnumKeyExW,
        RegGetValueW, RegOpenKeyExW, RegSetValueExW,
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

pub(super) fn private_root() -> io::Result<PathBuf> {
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

fn add_to_users_group(account_sid: &str) -> io::Result<()> {
    use windows_sys::Win32::NetworkManagement::NetManagement::{
        LOCALGROUP_MEMBERS_INFO_0, NetLocalGroupAddMembers,
    };
    let mut sid = null_mut();
    if unsafe { ConvertStringSidToSidW(wide("S-1-5-32-545").as_ptr(), &mut sid) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut name = [0u16; 256];
    let mut name_len = name.len() as u32;
    let mut domain = [0u16; 256];
    let mut domain_len = domain.len() as u32;
    let mut kind = 0;
    let found = unsafe {
        LookupAccountSidW(
            null(),
            sid,
            name.as_mut_ptr(),
            &mut name_len,
            domain.as_mut_ptr(),
            &mut domain_len,
            &mut kind,
        )
    };
    unsafe { LocalFree(sid.cast()) };
    if found == 0 {
        return Err(io::Error::last_os_error());
    }
    // Use the verified SID, not a relative name that SYSTEM may not resolve.
    let mut member = null_mut();
    if unsafe { ConvertStringSidToSidW(wide(account_sid).as_ptr(), &mut member) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let info = LOCALGROUP_MEMBERS_INFO_0 { lgrmi0_sid: member };
    let code = unsafe {
        NetLocalGroupAddMembers(
            null(),
            name.as_ptr(),
            0,
            (&info as *const LOCALGROUP_MEMBERS_INFO_0).cast(),
            1,
        )
    };
    unsafe { LocalFree(member) };
    if code != 0 && code != 1378 {
        return Err(io::Error::from_raw_os_error(code as i32));
    }
    Ok(())
}

pub(super) fn owned_account_sid() -> io::Result<String> {
    let saved: Account = serde_json::from_slice(&security::read_private("account.json", 4096)?)
        .map_err(|_| denied("propiedad de cuenta inválida"))?;
    let actual = account_sid()?;
    if saved.version != 1 || actual != saved.sid {
        return Err(denied("cuenta existente ajena; no se adopta"));
    }
    Ok(actual)
}

fn configure_service_account(sid: &str) -> io::Result<()> {
    // Only a newly created or SID-verified product account may reach this point.
    let mut account = null_mut();
    if unsafe { ConvertStringSidToSidW(wide(sid).as_ptr(), &mut account) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut attributes: LSA_OBJECT_ATTRIBUTES = unsafe { std::mem::zeroed() };
    attributes.Length = std::mem::size_of::<LSA_OBJECT_ATTRIBUTES>() as u32;
    let mut policy: LSA_HANDLE = 0;
    let status = unsafe {
        LsaOpenPolicy(
            null(),
            &attributes,
            (POLICY_LOOKUP_NAMES | POLICY_CREATE_ACCOUNT) as u32,
            &mut policy,
        )
    };
    if status != 0 {
        unsafe { LocalFree(account) };
        return Err(io::Error::from_raw_os_error(unsafe {
            LsaNtStatusToWinError(status) as i32
        }));
    }
    let mut names = [
        wide("SeServiceLogonRight"),
        wide("SeDenyInteractiveLogonRight"),
        wide("SeDenyRemoteInteractiveLogonRight"),
    ];
    let rights = names.each_mut().map(|name| LSA_UNICODE_STRING {
        Length: ((name.len() - 1) * 2) as u16,
        MaximumLength: (name.len() * 2) as u16,
        Buffer: name.as_mut_ptr(),
    });
    let status =
        unsafe { LsaAddAccountRights(policy, account, rights.as_ptr(), rights.len() as u32) };
    unsafe {
        LsaClose(policy);
        LocalFree(account);
    }
    if status != 0 {
        return Err(io::Error::from_raw_os_error(unsafe {
            LsaNtStatusToWinError(status) as i32
        }));
    }
    // Hiding the sign-in tile is presentation, not the security boundary above.
    let mut key = null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            wide(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon\SpecialAccounts\UserList")
                .as_ptr(),
            0,
            null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE | KEY_WOW64_64KEY,
            null(),
            &mut key,
            null_mut(),
        )
    };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    let hidden = 0u32.to_le_bytes();
    let status = unsafe {
        RegSetValueExW(
            key,
            wide(ACCOUNT).as_ptr(),
            0,
            REG_DWORD,
            hidden.as_ptr(),
            hidden.len() as u32,
        )
    };
    unsafe { RegCloseKey(key) };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    Ok(())
}

fn ensure_account(root: &Path) -> io::Result<String> {
    let mut existing = null_mut();
    let status = unsafe { NetUserGetInfo(null(), wide(ACCOUNT).as_ptr(), 1, &mut existing) };
    if status == 0 {
        unsafe { NetApiBufferFree(existing.cast()) };
        let actual = owned_account_sid()?;
        configure_service_account(&actual)?;
        add_to_users_group(&actual)?;
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
    let mut comment = wide("Cuenta de servicio GnX Core; sin inicio de sesión interactivo");
    let info = USER_INFO_1 {
        usri1_name: name.as_mut_ptr(),
        usri1_password: secret.as_mut_ptr(),
        usri1_password_age: 0,
        usri1_priv: USER_PRIV_USER,
        usri1_home_dir: null_mut(),
        usri1_comment: comment.as_mut_ptr(),
        // Windows local service accounts are still UF_NORMAL_ACCOUNT in SAM;
        // LSA logon rights, not an account-type flag, prevent human sign-in.
        usri1_flags: UF_NORMAL_ACCOUNT | UF_SCRIPT | UF_DONT_EXPIRE_PASSWD | UF_PASSWD_CANT_CHANGE,
        usri1_script_path: null_mut(),
    };
    if unsafe { NetUserAdd(null(), 1, (&info as *const USER_INFO_1).cast(), null_mut()) } != 0 {
        return Err(denied("no se pudo crear la cuenta dedicada"));
    }
    let sid = account_sid()?;
    add_to_users_group(&sid)?;
    write_new(
        &root.join("account.json"),
        &serde_json::to_vec(&Account {
            version: 1,
            sid: sid.clone(),
        })
        .map_err(io::Error::other)?,
    )?;
    configure_service_account(&sid)?;
    Ok(sid)
}

pub(super) fn registration(sid: &str, expected: &Path) -> io::Result<Option<String>> {
    let mut root = null_mut();
    let code = unsafe {
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
    };
    if code == 2 {
        return Ok(None);
    }
    if code != 0 {
        return Err(io::Error::from_raw_os_error(code as i32));
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
                return Ok(None);
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
        Err(denied(
            "inventario WSL excede el límite; ausencia no confirmada",
        ))
    })();
    unsafe { RegCloseKey(root) };
    result
}

pub(super) fn linux_path(path: &Path) -> io::Result<String> {
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
    security::require_system()?;
    let package = std::env::current_exe()?
        .parent()
        .ok_or_else(|| io::Error::other("ruta de instalación inválida"))?
        .join("payload");
    let prepared = match security::read_private("prepared", 16) {
        Ok(value) if value == b"1" => true,
        Ok(_) => return Err(denied("marcador de preparación inválido")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => return Err(error),
    };
    if prepared {
        // Reconcile ownership and apply a changed package only. Never recreate
        // a missing account/distro or replace its profile, CA or credentials.
        let _root = private_root()?;
        let sid = owned_account_sid()?;
        if super::identity::Ownership::read()?.sid != sid {
            return Err(denied(
                "la identidad preparada no coincide con la cuenta propia",
            ));
        }
        configure_service_account(&sid)?;
        if security::read_private("applied-version", 64)
            .is_ok_and(|value| value == env!("CARGO_PKG_VERSION").as_bytes())
        {
            return Ok(());
        }
        let files = crate::package::verify(&package)?;
        let identity = Identity::open(&super::identity::Ownership::read()?)?;
        apply(&identity, &package, "update", files)?;
        return fs::write(_root.join("applied-version"), env!("CARGO_PKG_VERSION"));
    }
    let files = crate::package::verify(&package)?;
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
    let imported = registration(&sid, &destination)?.is_none();
    if imported {
        match super::identity::Ownership::record() {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            _ => {
                return Err(denied(
                    "nodo declarado ausente; requiere recreación explícita, no otra importación",
                ));
            }
        }
        fs::create_dir_all(
            destination
                .parent()
                .ok_or_else(|| io::Error::other("ruta WSL inválida"))?,
        )?;
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
            let detail: String = result
                .text()
                .chars()
                .rev()
                .take(400)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            return Err(io::Error::other(format!(
                "importación WSL: exit {}; {detail}",
                result.code
            )));
        }
    }
    let guid = registration(&sid, &destination)?
        .ok_or_else(|| io::Error::other("registro WSL no confirmado"))?;
    // Bind the imported node before any later bootstrap step can fail.
    let ownership = super::identity::Ownership {
        version: 1,
        sid: sid.clone(),
        distribution_id: guid.clone(),
    };
    match super::identity::Ownership::record() {
        Ok(saved) if saved.sid == sid && saved.distribution_id == guid => {}
        Err(error) if imported && error.kind() == io::ErrorKind::NotFound => write_new(
            &root.join("identity.json"),
            &serde_json::to_vec(&ownership).map_err(io::Error::other)?,
        )?,
        _ => return Err(denied("identidad parcial divergente; no se sobrescribe")),
    }
    // Configuration is input data, not a Windows path that first needs DrvFS.
    let result = identity.run_payload(
        &[
            "--distribution",
            DISTRO,
            "--exec",
            "/usr/bin/install",
            "-m",
            "0644",
            "/dev/stdin",
            "/etc/wsl.conf",
        ],
        &fs::read(package.join("config/wsl.conf"))?,
        Duration::from_secs(60),
    )?;
    checked(result, "configuración WSL")?;
    let result = identity.run(&["--terminate", DISTRO], Duration::from_secs(30))?;
    if result.code != 0 {
        return Err(io::Error::other("reinicio de WSL propia no confirmado"));
    }
    apply(&identity, &package, "install", files)?;
    super::access::publish(&identity)?;
    if !root.join("intent").exists() {
        write_new(&root.join("intent"), b"running")?;
    }
    fs::write(root.join("applied-version"), env!("CARGO_PKG_VERSION"))?;
    write_new(&root.join("prepared"), b"1")?;
    Ok(())
}

fn checked(result: super::identity::Output, stage: &str) -> io::Result<()> {
    if result.code == 0 {
        return Ok(());
    }
    let detail: String = result
        .text()
        .chars()
        .rev()
        .take(350)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    Err(io::Error::other(format!(
        "{stage}: exit {}; {detail}",
        result.code
    )))
}

fn apply(
    identity: &Identity,
    package: &Path,
    mode: &str,
    mut files: Vec<String>,
) -> io::Result<()> {
    use sha2::{Digest, Sha256};
    use windows_sys::Win32::{
        Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation},
        System::Pipes::CreatePipe,
    };
    // Native tar -> inherited WSL stdin; no DrvFS, host archive or new transport service.
    let manifest = format!(
        "{:x}",
        Sha256::digest(fs::read(package.join("manifest.json"))?)
    );
    files.retain(|path| path != "rootfs.tar");
    files.push("manifest.json".into());
    if files.iter().any(|path| path.contains(['\r', '\n'])) {
        return Err(denied("nombre no transferible"));
    }
    let attrs = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    };
    let mut read = null_mut();
    let mut write = null_mut();
    if unsafe { CreatePipe(&mut read, &mut write, &attrs, 65536) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let read = security::Handle(read);
    let write = security::Handle(write);
    if unsafe { SetHandleInformation(read.0, HANDLE_FLAG_INHERIT, 0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let reader = unsafe { fs::File::from_raw_handle(read.0) };
    std::mem::forget(read);
    let writer = unsafe { fs::File::from_raw_handle(write.0) };
    std::mem::forget(write);
    let tar =
        PathBuf::from(std::env::var_os("SystemRoot").ok_or_else(|| denied("SystemRoot ausente"))?)
            .join("System32/tar.exe");
    let mut producer = Command::new(tar)
        .args(["--format=ustar", "-cf", "-", "-C"])
        .arg(package)
        .arg("--")
        .args(&files)
        .stdin(Stdio::null())
        .stdout(Stdio::from(writer))
        .stderr(Stdio::null())
        .spawn()?;
    let operations = r#"set -euo pipefail
umask 077
[[ "$1" =~ ^[a-f0-9]{64}$ && ( "$2" == install || "$2" == update ) ]]
if state=$(systemctl show -p ActiveState --value gnx-core-install.service); then :; else code=$?; [[ "$code" == 4 && "$state" == inactive ]] || exit "$code"; fi
case "$state" in active|activating|deactivating|reloading) echo 'Instalación Linux anterior activa; no repetir' >&2; exit 1;; esac
root=/var/lib/gnx-core
if [[ -e "$root" ]]; then [[ -d "$root" && ! -L "$root" && $(stat -c %u "$root") == 0 && $(stat -c %d "$root") == $(stat -c %d /) ]]; else install -d -m 0700 "$root"; fi
stage=$(mktemp -d "$root/payload.XXXXXXXX")
tar -x --ignore-zeros --no-same-owner --no-same-permissions -f - -C "$stage"
cd "$stage"
printf '%s  manifest.json\n' "$1" | sha256sum --check --status -
jq -er '.files[] | select(.path != "rootfs.tar") | "\(.sha256)  \(.path)"' manifest.json | sha256sum --check --status -
systemd-run --unit=gnx-core-install --wait --collect --property=Type=oneshot /usr/bin/bash "$stage/scripts/install.run" "$stage" "$2"
cd /
rm -r -- "$stage"
"#;
    let result = identity.run_pipe(
        &[
            "--distribution",
            DISTRO,
            "--exec",
            "/usr/bin/bash",
            "-c",
            operations,
            "gnx-bootstrap",
            &manifest,
            mode,
        ],
        &reader,
        Duration::from_secs(1200),
    );
    drop(reader);
    let status = producer.wait()?;
    checked(
        result?,
        "bootstrap Linux; consultar journal gnx-core-install",
    )?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "tar Windows: {status}; transferencia no confirmada"
        )));
    }
    Ok(())
}
