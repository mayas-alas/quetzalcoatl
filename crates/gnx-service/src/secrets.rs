use std::env;
use std::ffi::c_void;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::mem::size_of;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

use gnx_protocol::InstallerConfiguration;
use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};
use windows_sys::Win32::Security::{
    DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    SECURITY_ATTRIBUTES, SetFileSecurityW,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateDirectoryW, FILE_ATTRIBUTE_REPARSE_POINT, MOVEFILE_REPLACE_EXISTING,
    MOVEFILE_WRITE_THROUGH, MoveFileExW,
};
use zeroize::Zeroize;

const SERVICE_SID: &str = "S-1-5-80-1414281857-1943412974-186110390-2486725240-2230548587";
const MANAGED_DIRECTORY: &str = "managed";
const BLOB_NAME: &str = "installer-inputs.bin";
const DPAPI_ENTROPY: &[u8] = b"Quetzalcoatl/installer-inputs/v1";

pub fn store(configuration: &InstallerConfiguration) -> Result<(), ConfigurationError> {
    validate(configuration)?;
    let directory = secrets_directory()?;
    secure_directory(&directory)?;

    let mut plaintext = serde_json::to_vec(configuration)
        .map_err(|_| ConfigurationError::storage("cannot encode installer configuration"))?;
    let encrypted = protect(&plaintext);
    plaintext.zeroize();
    let encrypted = encrypted?;

    let path = directory.join(BLOB_NAME);
    atomic_write(&path, &encrypted)?;

    let verified = load_from(&path)?;
    if &verified != configuration {
        return Err(ConfigurationError::storage(
            "DPAPI read-after-write verification failed",
        ));
    }
    Ok(())
}

fn load_from(path: &Path) -> Result<InstallerConfiguration, ConfigurationError> {
    let encrypted =
        fs::read(path).map_err(|error| ConfigurationError::io("cannot read DPAPI blob", &error))?;
    let mut plaintext = unprotect(&encrypted)?;
    let parsed = serde_json::from_slice(&plaintext)
        .map_err(|_| ConfigurationError::storage("DPAPI blob has invalid configuration data"));
    plaintext.zeroize();
    let configuration = parsed?;
    validate(&configuration)?;
    Ok(configuration)
}

fn validate(configuration: &InstallerConfiguration) -> Result<(), ConfigurationError> {
    validate_tailnet(&configuration.tailnet)?;
    validate_auth_key(&configuration.auth_key)?;
    validate_password(&configuration.pve_root_password)?;
    Ok(())
}

fn validate_tailnet(value: &str) -> Result<(), ConfigurationError> {
    if value.len() < 7 || value.len() > 253 || !value.ends_with(".ts.net") {
        return Err(ConfigurationError::invalid(
            "tailnet must be its lowercase DNS name ending in .ts.net",
        ));
    }
    if !value.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    }) {
        return Err(ConfigurationError::invalid(
            "tailnet must be a valid lowercase DNS name",
        ));
    }
    Ok(())
}

fn validate_auth_key(value: &str) -> Result<(), ConfigurationError> {
    if value.len() < 20
        || value.len() > 512
        || !value.starts_with("tskey-auth-")
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(ConfigurationError::invalid(
            "auth_key must be a Tailscale auth key without whitespace",
        ));
    }
    Ok(())
}

fn validate_password(value: &str) -> Result<(), ConfigurationError> {
    if value.len() < 12 || value.len() > 128 || value.chars().any(char::is_control) {
        return Err(ConfigurationError::invalid(
            "PVE root password must contain 12 to 128 characters and no control characters",
        ));
    }
    Ok(())
}

fn secrets_directory() -> Result<PathBuf, ConfigurationError> {
    let program_data = env::var_os("ProgramData")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| ConfigurationError::storage("ProgramData is unavailable"))?;
    let root = program_data.join("Quetzalcoatl");
    ensure_product_root(&root)?;
    let managed = root.join(MANAGED_DIRECTORY);
    secure_directory(&managed)?;
    Ok(managed.join("secrets"))
}

fn ensure_product_root(path: &Path) -> Result<(), ConfigurationError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => verify_real_directory(&metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let descriptor = SecurityDescriptor::new()?;
            let wide_path = wide_path(path)?;
            let attributes = SECURITY_ATTRIBUTES {
                nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: descriptor.0,
                bInheritHandle: 0,
            };
            // Safety: path and descriptor remain valid for the duration of this call.
            if unsafe { CreateDirectoryW(wide_path.as_ptr(), &attributes) } == 0 {
                let error = unsafe { GetLastError() };
                if error != ERROR_ALREADY_EXISTS {
                    return Err(ConfigurationError::win32(
                        "cannot create product data directory",
                        error,
                    ));
                }
            }
            let metadata = fs::symlink_metadata(path).map_err(|error| {
                ConfigurationError::io("cannot inspect product data directory", &error)
            })?;
            verify_real_directory(&metadata)
        }
        Err(error) => Err(ConfigurationError::io(
            "cannot inspect product data directory",
            &error,
        )),
    }
}

fn secure_directory(path: &Path) -> Result<(), ConfigurationError> {
    let descriptor = SecurityDescriptor::new()?;
    let wide_path = wide_path(path)?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    // Safety: path and descriptor are NUL-terminated/valid for the duration of the call.
    if unsafe { CreateDirectoryW(wide_path.as_ptr(), &attributes) } == 0 {
        let error = unsafe { GetLastError() };
        if error != ERROR_ALREADY_EXISTS {
            return Err(ConfigurationError::win32(
                "cannot create protected configuration directory",
                error,
            ));
        }
    }
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        ConfigurationError::io("cannot inspect protected configuration directory", &error)
    })?;
    verify_real_directory(&metadata)?;
    apply_acl(&wide_path, descriptor.0)
}

fn verify_real_directory(metadata: &fs::Metadata) -> Result<(), ConfigurationError> {
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(ConfigurationError::storage(
            "protected configuration path is not a real directory",
        ));
    }
    Ok(())
}

fn apply_acl(path: &[u16], descriptor: PSECURITY_DESCRIPTOR) -> Result<(), ConfigurationError> {
    // Safety: the path is NUL-terminated and descriptor is a valid self-relative descriptor.
    if unsafe {
        SetFileSecurityW(
            path.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        )
    } == 0
    {
        Err(ConfigurationError::win32(
            "cannot apply protected configuration ACL",
            unsafe { GetLastError() },
        ))
    } else {
        Ok(())
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), ConfigurationError> {
    let directory = path
        .parent()
        .ok_or_else(|| ConfigurationError::storage("DPAPI blob path has no parent"))?;
    secure_directory(directory)?;
    let temporary = directory.join(format!(".{BLOB_NAME}.tmp"));
    if temporary.exists() {
        fs::remove_file(&temporary).map_err(|error| {
            ConfigurationError::io("cannot remove stale DPAPI temporary", &error)
        })?;
    }

    let write_result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| ConfigurationError::io("cannot create DPAPI temporary", &error))?;
        file.write_all(contents)
            .map_err(|error| ConfigurationError::io("cannot write DPAPI temporary", &error))?;
        file.sync_all()
            .map_err(|error| ConfigurationError::io("cannot flush DPAPI temporary", &error))?;
        let descriptor = SecurityDescriptor::new()?;
        apply_acl(&wide_path(&temporary)?, descriptor.0)
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }

    let temporary_wide = wide_path(&temporary)?;
    let target_wide = wide_path(path)?;
    // Safety: both paths are NUL-terminated and refer to files in the same protected directory.
    if unsafe {
        MoveFileExW(
            temporary_wide.as_ptr(),
            target_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        let error = unsafe { GetLastError() };
        let _ = fs::remove_file(&temporary);
        return Err(ConfigurationError::win32("cannot commit DPAPI blob", error));
    }
    let descriptor = SecurityDescriptor::new()?;
    apply_acl(&target_wide, descriptor.0)
}

fn protect(plaintext: &[u8]) -> Result<Vec<u8>, ConfigurationError> {
    crypt(plaintext, true)
}

fn unprotect(ciphertext: &[u8]) -> Result<Vec<u8>, ConfigurationError> {
    crypt(ciphertext, false)
}

fn crypt(input: &[u8], protect_data: bool) -> Result<Vec<u8>, ConfigurationError> {
    let input_len = u32::try_from(input.len())
        .map_err(|_| ConfigurationError::storage("DPAPI input is too large"))?;
    let input_blob = CRYPT_INTEGER_BLOB {
        cbData: input_len,
        pbData: input.as_ptr().cast_mut(),
    };
    let entropy_blob = CRYPT_INTEGER_BLOB {
        cbData: DPAPI_ENTROPY.len() as u32,
        pbData: DPAPI_ENTROPY.as_ptr().cast_mut(),
    };
    let mut output_blob = CRYPT_INTEGER_BLOB::default();
    let ok = if protect_data {
        // Safety: blobs point to valid memory and output is released with LocalFree below.
        unsafe {
            CryptProtectData(
                &input_blob,
                null(),
                &entropy_blob,
                null(),
                null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output_blob,
            )
        }
    } else {
        // Safety: blobs point to valid memory and output is released with LocalFree below.
        unsafe {
            CryptUnprotectData(
                &input_blob,
                null_mut(),
                &entropy_blob,
                null(),
                null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output_blob,
            )
        }
    };
    if ok == 0 {
        return Err(ConfigurationError::win32(
            "DPAPI operation failed",
            unsafe { GetLastError() },
        ));
    }
    if output_blob.cbData > 0 && output_blob.pbData.is_null() {
        return Err(ConfigurationError::storage(
            "DPAPI returned an invalid output buffer",
        ));
    }
    // Safety: DPAPI returned exactly cbData initialized bytes.
    let output = unsafe {
        std::slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize).to_vec()
    };
    // Safety: CryptProtectData/CryptUnprotectData allocate pbData with LocalAlloc.
    unsafe { LocalFree(output_blob.pbData.cast::<c_void>()) };
    Ok(output)
}

fn wide_path(path: &Path) -> Result<Vec<u16>, ConfigurationError> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    if wide.len() > 32_768 {
        Err(ConfigurationError::storage(
            "protected configuration path is too long",
        ))
    } else {
        Ok(wide)
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

struct SecurityDescriptor(PSECURITY_DESCRIPTOR);

impl SecurityDescriptor {
    fn new() -> Result<Self, ConfigurationError> {
        let sddl = wide(&format!("D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;{SERVICE_SID})"));
        let mut descriptor: PSECURITY_DESCRIPTOR = null_mut();
        // Safety: sddl is NUL-terminated and descriptor receives LocalAlloc-owned memory.
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                null_mut(),
            )
        } == 0
        {
            Err(ConfigurationError::win32(
                "cannot create protected configuration ACL",
                unsafe { GetLastError() },
            ))
        } else {
            Ok(Self(descriptor))
        }
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        // Safety: ConvertStringSecurityDescriptor allocated this descriptor with LocalAlloc.
        unsafe { LocalFree(self.0) };
    }
}

#[derive(Debug)]
pub struct ConfigurationError {
    code: &'static str,
    message: String,
}

impl ConfigurationError {
    fn invalid(message: &'static str) -> Self {
        Self {
            code: "CONFIGURATION_INVALID",
            message: message.into(),
        }
    }

    fn storage(message: &'static str) -> Self {
        Self {
            code: "CONFIGURATION_STORAGE_FAILED",
            message: message.into(),
        }
    }

    fn io(operation: &'static str, error: &std::io::Error) -> Self {
        let os_code = error.raw_os_error().unwrap_or_default();
        Self::storage_with_detail(operation, os_code)
    }

    fn win32(operation: &'static str, error: u32) -> Self {
        Self::storage_with_detail(operation, error as i32)
    }

    fn storage_with_detail(operation: &'static str, error: i32) -> Self {
        Self {
            code: "CONFIGURATION_STORAGE_FAILED",
            message: format!("{operation} (OS {error})"),
        }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_configuration() -> InstallerConfiguration {
        InstallerConfiguration {
            tailnet: "tetra-balance.ts.net".into(),
            auth_key: "tskey-auth-k-example-not-a-real-key".into(),
            pve_root_password: "not-a-real-password".into(),
            install_garage: true,
            install_forgejo: true,
        }
    }

    #[test]
    fn configuration_validation_accepts_the_single_i1_input_shape() {
        assert!(validate(&valid_configuration()).is_ok());
    }

    #[test]
    fn configuration_validation_rejects_non_tailnet_dns_and_weak_password() {
        let mut configuration = valid_configuration();
        configuration.tailnet = "Example.COM".into();
        assert_eq!(
            validate(&configuration)
                .expect_err("invalid tailnet")
                .code(),
            "CONFIGURATION_INVALID"
        );
        configuration.tailnet = "tetra-balance.ts.net".into();
        configuration.pve_root_password = "short".into();
        assert_eq!(
            validate(&configuration).expect_err("weak password").code(),
            "CONFIGURATION_INVALID"
        );
    }

    #[test]
    fn dpapi_round_trip_uses_the_current_windows_identity() {
        let mut plaintext = b"not-a-real-secret".to_vec();
        let encrypted = protect(&plaintext).expect("protect with DPAPI");
        assert_ne!(encrypted, plaintext);
        let mut recovered = unprotect(&encrypted).expect("unprotect with DPAPI");
        assert_eq!(recovered, plaintext);
        plaintext.zeroize();
        recovered.zeroize();
    }
}
