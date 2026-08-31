use crate::error::GnxError;

pub fn random_hex(bytes: usize) -> Result<String, GnxError> {
    let mut random = vec![0_u8; bytes];
    getrandom::fill(&mut random).map_err(|error| {
        GnxError::new(
            "SECRET_RANDOM_FAILED",
            "secrets",
            "generate",
            error.to_string(),
            "No continúe sin una fuente criptográfica de aleatoriedad disponible.",
            true,
            17,
        )
    })?;
    let mut encoded = String::with_capacity(bytes * 2);
    for byte in random {
        use std::fmt::Write;
        write!(&mut encoded, "{byte:02x}").expect("escribir en String no falla");
    }
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_secret_has_requested_entropy_length() {
        let secret = random_hex(32).unwrap();
        assert_eq!(secret.len(), 64);
        assert!(secret.bytes().all(|value| value.is_ascii_hexdigit()));
    }
}
