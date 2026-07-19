use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use windows_sys::Win32::Security::Cryptography::{
    BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom,
};
use zeroize::{Zeroize, ZeroizeOnDrop};

const BLOB_NAME: &str = "service-secrets.bin";
const SCHEMA_VERSION: u8 = 1;
const DPAPI_ENTROPY: &[u8] = b"Quetzalcoatl/service-secrets/v1";

#[derive(Deserialize, PartialEq, Serialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
pub struct ServiceSecrets {
    pub schema_version: u8,
    pub garage: Option<GarageSecrets>,
    pub forgejo: Option<ForgejoSecrets>,
}

#[derive(Deserialize, PartialEq, Serialize, Zeroize)]
#[serde(deny_unknown_fields)]
pub struct GarageSecrets {
    pub rpc_secret: String,
    pub admin_token: String,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
}

#[derive(Deserialize, PartialEq, Serialize, Zeroize)]
#[serde(deny_unknown_fields)]
pub struct ForgejoSecrets {
    pub secret_key: String,
    pub internal_token: String,
    pub admin_password: String,
}

impl ServiceSecrets {
    fn generate(install_garage: bool, install_forgejo: bool) -> Result<Self, ServiceSecretsError> {
        Ok(Self {
            schema_version: SCHEMA_VERSION,
            garage: install_garage
                .then(|| {
                    Ok(GarageSecrets {
                        rpc_secret: random_hex(32)?,
                        admin_token: random_hex(32)?,
                        s3_access_key: None,
                        s3_secret_key: None,
                    })
                })
                .transpose()?,
            forgejo: install_forgejo
                .then(|| {
                    Ok(ForgejoSecrets {
                        secret_key: random_hex(32)?,
                        internal_token: random_hex(32)?,
                        admin_password: random_hex(24)?,
                    })
                })
                .transpose()?,
        })
    }
}

pub fn load_or_create(
    install_garage: bool,
    install_forgejo: bool,
) -> Result<ServiceSecrets, ServiceSecretsError> {
    match load_optional()? {
        Some(secrets) => {
            validate_selection(&secrets, install_garage, install_forgejo)?;
            Ok(secrets)
        }
        None => {
            let secrets = ServiceSecrets::generate(install_garage, install_forgejo)?;
            store(&secrets)?;
            Ok(secrets)
        }
    }
}

pub fn store(secrets: &ServiceSecrets) -> Result<(), ServiceSecretsError> {
    validate(secrets)?;
    let mut plaintext = serde_json::to_vec(secrets)
        .map_err(|_| ServiceSecretsError::new("cannot encode service secrets"))?;
    let encrypted = crate::secrets::protect_payload(&plaintext, DPAPI_ENTROPY)
        .map_err(ServiceSecretsError::from_storage);
    plaintext.zeroize();
    let encrypted = encrypted?;
    let path = blob_path()?;
    crate::secrets::atomic_write(&path, &encrypted).map_err(ServiceSecretsError::from_storage)?;
    let verified = load_from(&path)?;
    if &verified != secrets {
        return Err(ServiceSecretsError::new(
            "service secret read-after-write verification failed",
        ));
    }
    Ok(())
}

fn load_optional() -> Result<Option<ServiceSecrets>, ServiceSecretsError> {
    let path = blob_path()?;
    match fs::read(&path) {
        Ok(encrypted) => load_encrypted(&encrypted).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ServiceSecretsError::io(
            "cannot read service secret blob",
            &error,
        )),
    }
}

fn load_from(path: &Path) -> Result<ServiceSecrets, ServiceSecretsError> {
    let encrypted = fs::read(path)
        .map_err(|error| ServiceSecretsError::io("cannot read service secret blob", &error))?;
    load_encrypted(&encrypted)
}

fn load_encrypted(encrypted: &[u8]) -> Result<ServiceSecrets, ServiceSecretsError> {
    let mut plaintext = crate::secrets::unprotect_payload(encrypted, DPAPI_ENTROPY)
        .map_err(ServiceSecretsError::from_storage)?;
    let parsed = serde_json::from_slice(&plaintext)
        .map_err(|_| ServiceSecretsError::new("service secret blob has invalid data"));
    plaintext.zeroize();
    let secrets = parsed?;
    validate(&secrets)?;
    Ok(secrets)
}

fn validate_selection(
    secrets: &ServiceSecrets,
    install_garage: bool,
    install_forgejo: bool,
) -> Result<(), ServiceSecretsError> {
    if secrets.garage.is_some() != install_garage || secrets.forgejo.is_some() != install_forgejo {
        return Err(ServiceSecretsError::new(
            "service secret selection does not match persisted controller state",
        ));
    }
    Ok(())
}

fn validate(secrets: &ServiceSecrets) -> Result<(), ServiceSecretsError> {
    if secrets.schema_version != SCHEMA_VERSION {
        return Err(ServiceSecretsError::new(
            "unsupported service secret schema",
        ));
    }
    if let Some(garage) = secrets.garage.as_ref()
        && (!valid_hex(&garage.rpc_secret, 64)
            || !valid_hex(&garage.admin_token, 64)
            || garage
                .s3_access_key
                .as_ref()
                .is_some_and(|value| !valid_garage_access_key(value))
            || garage
                .s3_secret_key
                .as_ref()
                .is_some_and(|value| !valid_hex(value, 64))
            || garage.s3_access_key.is_some() != garage.s3_secret_key.is_some())
    {
        return Err(ServiceSecretsError::new("invalid Garage secret data"));
    }
    if let Some(forgejo) = secrets.forgejo.as_ref()
        && (!valid_hex(&forgejo.secret_key, 64)
            || !valid_hex(&forgejo.internal_token, 64)
            || !valid_hex(&forgejo.admin_password, 48))
    {
        return Err(ServiceSecretsError::new("invalid Forgejo secret data"));
    }
    Ok(())
}

fn valid_garage_access_key(value: &str) -> bool {
    value.len() == 26
        && value.starts_with("GK")
        && value[2..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn random_hex(byte_count: usize) -> Result<String, ServiceSecretsError> {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut bytes = vec![0_u8; byte_count];
    let status = unsafe {
        BCryptGenRandom(
            std::ptr::null_mut(),
            bytes.as_mut_ptr(),
            u32::try_from(bytes.len())
                .map_err(|_| ServiceSecretsError::new("CSPRNG request is too large"))?,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status < 0 {
        bytes.zeroize();
        return Err(ServiceSecretsError::new(format!(
            "Windows CSPRNG failed (NTSTATUS {status})"
        )));
    }
    let mut encoded = String::with_capacity(byte_count * 2);
    for byte in &bytes {
        encoded.push(HEX[usize::from(byte >> 4)] as char);
        encoded.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    bytes.zeroize();
    Ok(encoded)
}

fn blob_path() -> Result<PathBuf, ServiceSecretsError> {
    crate::secrets::product_root()
        .map(|root| root.join("secrets").join(BLOB_NAME))
        .map_err(ServiceSecretsError::from_storage)
}

#[derive(Debug)]
pub struct ServiceSecretsError {
    message: String,
}

impl ServiceSecretsError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn io(operation: &'static str, error: &std::io::Error) -> Self {
        Self::new(format!(
            "{operation} (OS {})",
            error.raw_os_error().unwrap_or_default()
        ))
    }

    fn from_storage(error: crate::secrets::ConfigurationError) -> Self {
        Self::new(error.message())
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_service_secrets_match_the_selected_i1_shape() {
        let secrets = ServiceSecrets::generate(true, true).expect("generate service secrets");
        assert!(validate(&secrets).is_ok());
        assert_eq!(
            secrets.garage.as_ref().expect("Garage").rpc_secret.len(),
            64
        );
        assert_eq!(
            secrets
                .forgejo
                .as_ref()
                .expect("Forgejo")
                .admin_password
                .len(),
            48
        );
        let json = serde_json::to_string(&secrets).expect("serialize service secrets");
        assert!(!json.contains("auth_key"));
        assert!(!json.contains("pve_root_password"));
    }

    #[test]
    fn service_secret_entropy_is_distinct_from_installer_inputs() {
        assert_ne!(DPAPI_ENTROPY, b"Quetzalcoatl/installer-inputs/v1");
    }

    #[test]
    fn garage_s3_credential_is_write_once() {
        let mut secrets = ServiceSecrets::generate(true, false).expect("generate service secrets");
        let garage = secrets.garage.as_mut().expect("Garage");
        garage.s3_access_key = Some("GK0123456789abcdef01234567".into());
        garage.s3_secret_key = Some("a".repeat(64));
        assert!(validate(&secrets).is_ok());

        let garage = secrets.garage.as_mut().expect("Garage");
        garage.s3_access_key = Some("invalid".into());
        assert!(validate(&secrets).is_err());
    }
}
