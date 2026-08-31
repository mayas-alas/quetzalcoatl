use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_LOCAL_MACHINE, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData,
    CryptUnprotectData,
};

use crate::error::GnxError;
use crate::process::CommandSpec;

const ENTROPY: &[u8] = b"QuetzalcoatlNext/mesh-preauth/v1";
const PENDING_FILE: &str = "mesh-auth.pending.dpapi";

pub fn submit_mesh_auth(secret: &mut [u8]) -> Result<(), GnxError> {
    validate(secret)?;
    let result = if crate::host::windows::install::is_elevated() {
        stage_machine_secret(secret)
    } else {
        let sealed = protect(secret, false)?;
        let temporary = std::env::temp_dir().join(format!(
            "gnx-mesh-auth-{}.dpapi",
            crate::secrets::random_hex(16)?
        ));
        let write = crate::state::atomic_write(&temporary, &sealed);
        let outcome = match write {
            Ok(()) => {
                let parameters = format!(
                    "__mesh-auth --elevated --sealed \"{}\"",
                    temporary.display()
                );
                let code = crate::host::windows::install::elevate(
                    &parameters,
                    "entregar de forma segura la credencial efímera de Headscale",
                )?;
                if code == 0 {
                    Ok(())
                } else {
                    Err(GnxError::new(
                        "MESH_AUTH_ELEVATED_CHILD_FAILED",
                        "mesh",
                        "auth_stage",
                        format!("El proceso elevado terminó con código {code}."),
                        "Vuelva a ejecutar gnx init --mesh-auth-stdin y acepte UAC.",
                        true,
                        16,
                    ))
                }
            }
            Err(error) => Err(error),
        };
        let _ = std::fs::remove_file(&temporary);
        outcome
    };
    secret.fill(0);
    result
}

pub fn complete_mesh_auth(elevated: bool, sealed: &Path) -> Result<(), GnxError> {
    if !elevated || !crate::host::windows::install::is_elevated() {
        return Err(GnxError::new(
            "HOST_ELEVATION_REQUIRED",
            "mesh",
            "auth_stage",
            "La entrega de la credencial no recibió un token elevado.",
            "Ejecute gnx init --mesh-auth-stdin y acepte UAC.",
            false,
            9,
        ));
    }
    let ciphertext = std::fs::read(sealed)
        .map_err(|error| GnxError::io("mesh_auth_sealed_read", error.to_string()))?;
    let mut secret = unprotect(&ciphertext)?;
    validate(&secret)?;
    let result = stage_machine_secret(&secret);
    secret.fill(0);
    result
}

pub fn load_pending_mesh_auth() -> Result<Option<Vec<u8>>, GnxError> {
    let path = pending_path();
    let ciphertext = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(GnxError::io("mesh_auth_pending_read", error.to_string()));
        }
    };
    let secret = unprotect(&ciphertext)?;
    validate(&secret)?;
    Ok(Some(secret))
}

pub fn discard_pending_mesh_auth() -> Result<(), GnxError> {
    match std::fs::remove_file(pending_path()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(GnxError::io("mesh_auth_pending_remove", error.to_string())),
    }
}

fn stage_machine_secret(secret: &[u8]) -> Result<(), GnxError> {
    let path = pending_path();
    let ciphertext = protect(secret, true)?;
    crate::state::atomic_write(&path, &ciphertext)?;
    let runtime_grant = format!(r"{}:R", crate::host::windows::account::RUNTIME_ACCOUNT_NAME);
    CommandSpec::new(r"C:\Windows\System32\icacls.exe")
        .arg(&path)
        .args([
            "/inheritance:r",
            "/grant:r",
            r"*S-1-5-18:F",
            r"*S-1-5-32-544:F",
            &runtime_grant,
        ])
        .run_checked("mesh_auth_acl")?;
    crate::logs::event(
        "info",
        "mesh",
        "auth_stage",
        "Credencial efímera cifrada y entregada a la identidad dedicada",
    );
    crate::host::windows::service::stop()?;
    crate::host::windows::service::start()?;
    Ok(())
}

fn pending_path() -> PathBuf {
    crate::config::data_root().join(PENDING_FILE)
}

fn validate(secret: &[u8]) -> Result<(), GnxError> {
    if !(16..=1024).contains(&secret.len())
        || secret
            .iter()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return Err(GnxError::new(
            "MESH_AUTH_INVALID",
            "mesh",
            "auth_validate",
            "La pre-auth key debe tener entre 16 y 1024 bytes, sin espacios ni saltos de línea internos.",
            "Genere una pre-auth key reutilizable y etiquetada en Headscale y entréguela sólo por stdin.",
            false,
            16,
        ));
    }
    Ok(())
}

fn protect(secret: &[u8], machine_scope: bool) -> Result<Vec<u8>, GnxError> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: secret.len() as u32,
        pbData: secret.as_ptr() as *mut u8,
    };
    let entropy = CRYPT_INTEGER_BLOB {
        cbData: ENTROPY.len() as u32,
        pbData: ENTROPY.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    let flags = CRYPTPROTECT_UI_FORBIDDEN
        | if machine_scope {
            CRYPTPROTECT_LOCAL_MACHINE
        } else {
            0
        };
    // SAFETY: input and entropy point to live buffers; DPAPI allocates output with LocalAlloc.
    let protected =
        unsafe { CryptProtectData(&input, null(), &entropy, null(), null(), flags, &mut output) };
    if protected == 0 {
        return Err(GnxError::io(
            "mesh_auth_protect",
            std::io::Error::last_os_error().to_string(),
        ));
    }
    // SAFETY: DPAPI returned a buffer of cbData bytes.
    let result =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    // SAFETY: output was allocated by DPAPI and is released exactly once.
    unsafe { LocalFree(output.pbData.cast()) };
    Ok(result)
}

fn unprotect(ciphertext: &[u8]) -> Result<Vec<u8>, GnxError> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: ciphertext.len() as u32,
        pbData: ciphertext.as_ptr() as *mut u8,
    };
    let entropy = CRYPT_INTEGER_BLOB {
        cbData: ENTROPY.len() as u32,
        pbData: ENTROPY.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    let mut description = null_mut();
    // SAFETY: input and entropy point to live buffers; DPAPI allocates output and description.
    let unprotected = unsafe {
        CryptUnprotectData(
            &input,
            &mut description,
            &entropy,
            null(),
            null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if unprotected == 0 {
        return Err(GnxError::io(
            "mesh_auth_unprotect",
            std::io::Error::last_os_error().to_string(),
        ));
    }
    // SAFETY: DPAPI returned a buffer of cbData bytes.
    let result =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    // SAFETY: both optional buffers were allocated by DPAPI and are released once.
    unsafe {
        LocalFree(output.pbData.cast());
        if !description.is_null() {
            LocalFree(description.cast());
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpapi_round_trip_uses_explicit_entropy() {
        let secret = b"0123456789abcdef0123456789abcdef";
        let protected = protect(secret, false).unwrap();
        assert_ne!(protected, secret);
        assert_eq!(unprotect(&protected).unwrap(), secret);
    }

    #[test]
    fn auth_validation_rejects_multiline_input() {
        assert!(validate(b"0123456789abcdef\nsecond").is_err());
    }
}
