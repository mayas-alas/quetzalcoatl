use super::{console, security};
use crate::protocol::invalid;
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
    process::{Child, Command},
    ptr::null_mut,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::{NamedPipeServer, ServerOptions},
};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::{
        Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1},
        SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::CreateDirectoryW,
    System::{SystemInformation::GetSystemDirectoryW, Threading::GetCurrentProcess},
};

const ALIAS: &str = "vm104";
const DESTINATION: &str = "gnx-incus-kini-lab";
const USER: &str = "debian";
const PIN: &str = "vm104.host.pub";
const RELAY: &str = "gnx-tailcat-relay.exe";
const KNOWN_HOSTS: &str = "known_hosts";
const EMBEDDED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/gnx-tailcat-relay.exe"));

pub fn run(args: &[String]) -> io::Result<bool> {
    let [selection] = args else {
        return Err(invalid("usa gnx connect list o gnx connect vm104"));
    };
    match selection.as_str() {
        "--licenses" => {
            io::stdout()
                .lock()
                .write_all(include_bytes!("../../tunnel/THIRD_PARTY_NOTICES.txt"))?;
            Ok(true)
        }
        "list" => {
            let state = match read_pin() {
                Ok(Some(_)) => "clave de servidor preparada",
                Ok(None) => "clave de servidor pendiente",
                Err(_) => "clave de servidor no válida o inaccesible",
            };
            println!("{ALIAS}\tVM 104\t{DESTINATION}\t{USER}\t{state}");
            println!("Perfil local; no consulta la disponibilidad de la VM.");
            Ok(true)
        }
        ALIAS => connect(),
        _ => Err(invalid("destino no registrado; usa gnx connect list")),
    }
}

fn local_profile() -> io::Result<PathBuf> {
    let path = PathBuf::from(
        std::env::var_os("LOCALAPPDATA").ok_or_else(|| invalid("perfil local no disponible"))?,
    );
    if !path.is_absolute() {
        return Err(invalid("perfil local inválido"));
    }
    Ok(path)
}

fn read_pin() -> io::Result<Option<String>> {
    let path = local_profile()?.join("GnX Core").join("connect").join(PIN);
    let file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let mut text = String::new();
    file.take(1025).read_to_string(&mut text)?;
    let fields: Vec<_> = text.split_ascii_whitespace().collect();
    // Accept only the canonical 51-byte OpenSSH Ed25519 blob (68 base64 bytes).
    let ["ssh-ed25519", blob] = fields.as_slice() else {
        return Err(invalid(
            "se requiere una sola clave pública SSH Ed25519 sin comentario",
        ));
    };
    if text.len() > 1024
        || blob.len() != 68
        || !blob.starts_with("AAAAC3NzaC1lZDI1NTE5AAAAI")
        || !blob
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'/'))
    {
        return Err(invalid("clave pública SSH Ed25519 inválida"));
    }
    Ok(Some(format!("{DESTINATION} ssh-ed25519 {blob}\n")))
}

fn ssh_client() -> io::Result<PathBuf> {
    let mut path = [0; 1024];
    let len = unsafe { GetSystemDirectoryW(path.as_mut_ptr(), path.len() as u32) } as usize;
    if len == 0 || len >= path.len() {
        return Err(io::Error::other(
            "no se encontró el directorio de sistema Windows",
        ));
    }
    let ssh = PathBuf::from(std::ffi::OsString::from_wide(&path[..len]))
        .join("OpenSSH")
        .join("ssh.exe");
    if !ssh.is_file() {
        return Err(io::Error::other("falta el cliente OpenSSH de Windows"));
    }
    Ok(ssh)
}

fn windows_arg(path: &Path) -> io::Result<&str> {
    path.to_str()
        .filter(|text| !text.chars().any(|c| c.is_control() || "\"%!".contains(c)))
        .ok_or_else(|| invalid("la ruta del perfil no es compatible con OpenSSH para Windows"))
}

fn connect() -> io::Result<bool> {
    if security::effective_admin().is_ok() {
        return Err(security::denied(
            "abre una terminal Windows sin elevar para conectar",
        ));
    }
    let pin = read_pin()?.ok_or_else(|| {
        invalid(
            "falta la clave pública del SSH de Tailcat para vm104; consulta docs/connect-vm104.md",
        )
    })?;
    let ssh = ssh_client()?;
    let profile = local_profile()?;
    windows_arg(&profile)?;
    // Keep the credential with its user in the CLI, outside the service pipe.
    eprintln!("Destino: VM 104 · {DESTINATION} · usuario {USER}");
    let invitation = console::read_masked("Invitación temporal de Tailcat: ")?;
    if invitation.len() < 3
        || !invitation.starts_with("tc")
        || !invitation
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    {
        return Err(invalid(
            "invitación Tailcat inválida; pega solo la dirección temporal",
        ));
    }
    let mut random = [0; 16];
    security::random(&mut random)?;
    let id: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
    let pipe = format!(r"\\.\pipe\GnX.Connect.{id}");
    let token = security::process_token(unsafe { GetCurrentProcess() })?;
    let sid = security::token_sid(token.0)?;
    if sid == "S-1-5-18" {
        return Err(security::denied(
            "la conexión requiere un usuario de Windows",
        ));
    }
    let access = UserAccess::new(&sid, "")?;
    let directory_access = UserAccess::new(&sid, "OICI")?;
    let session = Session::create(profile.join(format!("GnXConnect-{id}")), &directory_access)?;
    File::create_new(session.0.join(KNOWN_HOSTS))?.write_all(pin.as_bytes())?;
    let relay = session.0.join(RELAY);
    File::create_new(&relay)?.write_all(EMBEDDED)?;
    let known_hosts = session.0.join(KNOWN_HOSTS);
    let known_hosts = windows_arg(&known_hosts)?.replace('\\', "/");
    let proxy = format!("ProxyCommand=\"{}\" \"{pipe}\"", windows_arg(&relay)?);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let child = runtime.block_on(async {
        let mut listener = access.listen(&pipe)?;
        let mut child = Command::new(ssh)
            .args([
                "-F",
                "NUL",
                "-tt",
                "-o",
                "BatchMode=yes",
                "-o",
                "StrictHostKeyChecking=yes",
                "-o",
                "GlobalKnownHostsFile=NUL",
                "-o",
                &format!("UserKnownHostsFile=\"{known_hosts}\""),
                "-o",
                "HostKeyAlgorithms=ssh-ed25519",
                "-o",
                "UpdateHostKeys=no",
                "-o",
                "IdentityAgent=none",
                "-o",
                "PubkeyAuthentication=no",
                "-o",
                "PasswordAuthentication=no",
                "-o",
                "KbdInteractiveAuthentication=no",
                "-o",
                "ClearAllForwardings=yes",
                "-o",
                "EscapeChar=none",
                "-o",
                "ConnectTimeout=30",
                "-o",
                &proxy,
                &format!("{USER}@{DESTINATION}"),
            ])
            .spawn()?;
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            send_invitation(&mut listener, &mut child, invitation.as_bytes()),
        )
        .await;
        match result {
            Ok(Ok(())) => Ok(child),
            Ok(Err(error)) => {
                stop(&mut child);
                Err(error)
            }
            Err(_) => {
                stop(&mut child);
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "OpenSSH no abrió el adaptador local",
                ))
            }
        }
    });
    drop(invitation);
    let status = child?.wait()?;
    if status.success() {
        println!(
            "Sesión terminada. La invitación sigue vigente hasta que termine el listener de la VM."
        );
        Ok(true)
    } else {
        Err(io::Error::other(
            "no se completó la sesión; revisa el mensaje de OpenSSH",
        ))
    }
}

async fn send_invitation(
    listener: &mut NamedPipeServer,
    child: &mut Child,
    invite: &[u8],
) -> io::Result<()> {
    tokio::select! {
        result = listener.connect() => result?,
        result = async {
            loop {
                if child.try_wait()?.is_some() {
                    return Err(io::Error::other("OpenSSH cerró antes de abrir el adaptador local"));
                }
                tokio::time::sleep(Duration::from_millis(150)).await;
            }
        } => return result,
    }
    listener.write_u16_le(invite.len() as u16).await?;
    listener.write_all(invite).await?;
    if listener.read_u8().await? != 1 {
        return Err(io::Error::other("el adaptador no confirmó la invitación"));
    }
    Ok(())
}

fn stop(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

struct UserAccess(SECURITY_ATTRIBUTES);

impl UserAccess {
    fn new(sid: &str, inheritance: &str) -> io::Result<Self> {
        let sddl: Vec<_> = format!("D:P(A;{inheritance};GA;;;{sid})")
            .encode_utf16()
            .chain([0])
            .collect();
        let mut descriptor = null_mut();
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor,
            bInheritHandle: 0,
        }))
    }

    fn listen(&self, name: &str) -> io::Result<NamedPipeServer> {
        // The only allowed SID is the Windows user who entered the credential.
        unsafe {
            ServerOptions::new()
                .first_pipe_instance(true)
                .reject_remote_clients(true)
                .max_instances(1)
                .create_with_security_attributes_raw(
                    name,
                    std::ptr::from_ref(&self.0).cast_mut().cast(),
                )
        }
    }
}

impl Drop for UserAccess {
    fn drop(&mut self) {
        unsafe { LocalFree(self.0.lpSecurityDescriptor) };
    }
}

struct Session(PathBuf);

impl Session {
    fn create(path: PathBuf, access: &UserAccess) -> io::Result<Self> {
        let path_w: Vec<_> = path.as_os_str().encode_wide().chain([0]).collect();
        if unsafe { CreateDirectoryW(path_w.as_ptr(), &access.0) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(path))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // These artifacts contain only the embedded relay and public key.
        let _ = fs::remove_file(self.0.join(RELAY));
        let _ = fs::remove_file(self.0.join(KNOWN_HOSTS));
        let _ = fs::remove_dir(&self.0);
    }
}
