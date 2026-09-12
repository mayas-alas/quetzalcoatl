use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const DOMAIN: &[u8] = b"GNX-RELEASE-MANIFEST-V1\0";

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DetachedSignature {
    schema: u32,
    algorithm: String,
    key_id: String,
    signature: String,
}

fn decode_exact<const N: usize>(value: &str) -> Result<[u8; N], String> {
    let decoded = hex::decode(value.trim()).map_err(|_| "RELEASE_SIGNATURE_INVALID")?;
    decoded
        .try_into()
        .map_err(|_| "RELEASE_SIGNATURE_INVALID".into())
}

fn signed_message(manifest: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(DOMAIN.len() + manifest.len());
    message.extend_from_slice(DOMAIN);
    message.extend_from_slice(manifest);
    message
}

pub fn key_id(public_key: &[u8; 32]) -> String {
    hex::encode(Sha256::digest(public_key))
}

pub fn public_key(secret_key_hex: &str) -> Result<String, String> {
    let secret = decode_exact::<32>(secret_key_hex).map_err(|_| "RELEASE_PRIVATE_KEY_INVALID")?;
    Ok(hex::encode(
        SigningKey::from_bytes(&secret).verifying_key().to_bytes(),
    ))
}

pub fn sign(manifest: &[u8], secret_key_hex: &str) -> Result<String, String> {
    let secret = decode_exact::<32>(secret_key_hex).map_err(|_| "RELEASE_PRIVATE_KEY_INVALID")?;
    let signing_key = SigningKey::from_bytes(&secret);
    let public = signing_key.verifying_key().to_bytes();
    let signature = signing_key.sign(&signed_message(manifest));
    serde_json::to_string(&DetachedSignature {
        schema: 1,
        algorithm: "Ed25519".into(),
        key_id: key_id(&public),
        signature: hex::encode(signature.to_bytes()),
    })
    .map_err(|_| "RELEASE_SIGNATURE_INVALID".into())
}

pub fn verify(
    manifest: &[u8],
    signature_json: &[u8],
    public_key_hex: &str,
) -> Result<String, String> {
    let document: DetachedSignature =
        serde_json::from_slice(signature_json).map_err(|_| "RELEASE_SIGNATURE_INVALID")?;
    if document.schema != 1 || document.algorithm != "Ed25519" {
        return Err("RELEASE_SIGNATURE_INVALID".into());
    }
    let public = decode_exact::<32>(public_key_hex)?;
    let expected_key_id = key_id(&public);
    if document.key_id != expected_key_id {
        return Err("RELEASE_SIGNER_UNTRUSTED".into());
    }
    let verifying_key =
        VerifyingKey::from_bytes(&public).map_err(|_| "RELEASE_SIGNER_UNTRUSTED")?;
    let signature_bytes = decode_exact::<64>(&document.signature)?;
    let signature = Signature::from_bytes(&signature_bytes);
    verifying_key
        .verify_strict(&signed_message(manifest), &signature)
        .map_err(|_| "RELEASE_SIGNATURE_INVALID")?;
    Ok(expected_key_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 8032 test key. It is intentionally used only inside unit tests.
    const TEST_SECRET: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
    const TEST_PUBLIC: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

    #[test]
    fn signed_manifest_verifies_and_reports_key_identity() {
        let manifest = br#"{"schema":1,"version":"test"}"#;
        let signature = sign(manifest, TEST_SECRET).unwrap();
        assert_eq!(public_key(TEST_SECRET).unwrap(), TEST_PUBLIC);
        assert_eq!(
            verify(manifest, signature.as_bytes(), TEST_PUBLIC).unwrap(),
            key_id(&decode_exact::<32>(TEST_PUBLIC).unwrap())
        );
    }

    #[test]
    fn tampering_wrong_key_and_malformed_signature_are_rejected() {
        let manifest = br#"{"schema":1}"#;
        let signature = sign(manifest, TEST_SECRET).unwrap();
        assert_eq!(
            verify(b"changed", signature.as_bytes(), TEST_PUBLIC).unwrap_err(),
            "RELEASE_SIGNATURE_INVALID"
        );
        let wrong_public = "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";
        assert_eq!(
            verify(manifest, signature.as_bytes(), wrong_public).unwrap_err(),
            "RELEASE_SIGNER_UNTRUSTED"
        );
        assert_eq!(
            verify(manifest, b"{}", TEST_PUBLIC).unwrap_err(),
            "RELEASE_SIGNATURE_INVALID"
        );
    }
}
