use std::ffi::c_void;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;

use sha2::{Digest, Sha256};
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Security::Cryptography::{
    CERT_STORE_ADD_REPLACE_EXISTING, CERT_STORE_OPEN_EXISTING_FLAG, CERT_STORE_PROV_SYSTEM_W,
    CERT_SYSTEM_STORE_LOCAL_MACHINE, CertAddEncodedCertificateToStore, CertCloseStore,
    CertCreateCertificateContext, CertFreeCertificateContext, CertOpenStore, PKCS_7_ASN_ENCODING,
    X509_ASN_ENCODING,
};

const MIN_CERTIFICATE_BYTES: u64 = 256;
const MAX_CERTIFICATE_BYTES: u64 = 64 * 1024;

pub(crate) struct Spec {
    pub root_certificate: PathBuf,
    pub root_sha256: [u8; 32],
    pub publisher_certificate: PathBuf,
    pub publisher_sha256: [u8; 32],
}

pub(crate) fn install(spec: Spec) -> Result<String, String> {
    let root_parent = canonical_parent(&spec.root_certificate)?;
    let publisher_parent = canonical_parent(&spec.publisher_certificate)?;
    if root_parent != publisher_parent {
        return Err(
            "QA root and publisher certificates must share one bundle cache directory".into(),
        );
    }

    let root = read_certificate(&spec.root_certificate, &spec.root_sha256, "root")?;
    let publisher = read_certificate(
        &spec.publisher_certificate,
        &spec.publisher_sha256,
        "publisher",
    )?;
    validate_der_certificate(&root, "root")?;
    validate_der_certificate(&publisher, "publisher")?;

    add_to_machine_store("Root", &root)?;
    add_to_machine_store("TrustedPublisher", &publisher)?;
    Ok("pinned public QA root and publisher are trusted for this machine".into())
}

fn canonical_parent(path: &Path) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "QA certificate path has no parent directory".to_string())?;
    fs::canonicalize(parent)
        .map_err(|error| format!("cannot resolve QA certificate directory: {error}"))
}

fn read_certificate(
    path: &Path,
    expected_sha256: &[u8; 32],
    name: &str,
) -> Result<Vec<u8>, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("cannot inspect QA {name} certificate: {error}"))?;
    if !metadata.is_file()
        || metadata.len() < MIN_CERTIFICATE_BYTES
        || metadata.len() > MAX_CERTIFICATE_BYTES
    {
        return Err(format!(
            "QA {name} certificate size is outside the closed {MIN_CERTIFICATE_BYTES}..={MAX_CERTIFICATE_BYTES} byte range"
        ));
    }
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read QA {name} certificate: {error}"))?;
    let actual = Sha256::digest(&bytes);
    if actual[..] != expected_sha256[..] {
        return Err(format!("QA {name} certificate SHA-256 mismatch"));
    }
    Ok(bytes)
}

fn validate_der_certificate(bytes: &[u8], name: &str) -> Result<(), String> {
    let encoding = X509_ASN_ENCODING | PKCS_7_ASN_ENCODING;
    // Safety: bytes remains alive for the call and Crypt32 owns the returned context.
    let context =
        unsafe { CertCreateCertificateContext(encoding, bytes.as_ptr(), bytes.len() as u32) };
    if context.is_null() {
        return Err(last_error(&format!(
            "cannot decode QA {name} DER certificate"
        )));
    }
    // Safety: context was returned by CertCreateCertificateContext exactly once.
    unsafe { CertFreeCertificateContext(context) };
    Ok(())
}

fn add_to_machine_store(store_name: &str, bytes: &[u8]) -> Result<(), String> {
    let wide_name: Vec<u16> = std::ffi::OsStr::new(store_name)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let encoding = X509_ASN_ENCODING | PKCS_7_ASN_ENCODING;
    // Safety: the provider constant and NUL-terminated store name satisfy CertOpenStore.
    let store = unsafe {
        CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            encoding,
            0,
            CERT_SYSTEM_STORE_LOCAL_MACHINE | CERT_STORE_OPEN_EXISTING_FLAG,
            wide_name.as_ptr() as *const c_void,
        )
    };
    if store.is_null() {
        return Err(last_error(&format!(
            "cannot open LocalMachine\\{store_name}"
        )));
    }

    // Safety: store is open, bytes remains alive, and no output context is requested.
    let added = unsafe {
        CertAddEncodedCertificateToStore(
            store,
            encoding,
            bytes.as_ptr(),
            bytes.len() as u32,
            CERT_STORE_ADD_REPLACE_EXISTING,
            null_mut(),
        )
    };
    let error = if added == 0 {
        Some(unsafe { GetLastError() })
    } else {
        None
    };
    // Safety: store was returned by CertOpenStore and is closed exactly once.
    unsafe { CertCloseStore(store, 0) };
    if let Some(error) = error {
        return Err(format!(
            "cannot add public QA certificate to LocalMachine\\{store_name}: Win32 error {error}"
        ));
    }
    Ok(())
}

fn last_error(operation: &str) -> String {
    // Safety: GetLastError has no parameters and follows the failed Win32 call.
    unsafe { format!("{operation}: Win32 error {}", GetLastError()) }
}
