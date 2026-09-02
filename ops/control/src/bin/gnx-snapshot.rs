use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
    process::{Command, Stdio},
};

use age::{secrecy::ExposeSecret, x25519::Identity};
use sha2::{Digest, Sha256};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let result = match args.get(1).map(String::as_str) {
        Some("create") if args.len() == 6 => {
            create(&args[2], &args[3], Path::new(&args[4]), Path::new(&args[5]))
        }
        Some("check") if args.len() == 4 => {
            check(Path::new(&args[2]), Path::new(&args[3])).map(|_| ())
        }
        _ => Err("invalid snapshot arguments".into()),
    };
    if result.is_err() {
        eprintln!("FAILED CONTROL_SNAPSHOT");
        std::process::exit(1);
    }
    println!("READY control-snapshot");
}

fn key(path: &Path, allow_create: bool) -> Result<Identity> {
    if !path.exists() && allow_create {
        let identity = Identity::generate();
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(identity.to_string().expose_secret().as_bytes())?;
        file.sync_all()?;
        return Ok(identity);
    }
    fs::read_to_string(path)?
        .trim()
        .parse()
        .map_err(|_| "recovery identity unavailable".into())
}

fn transfer(mut source: impl Read, mut target: impl Write) -> Result<(u64, String)> {
    let mut digest = Sha256::new();
    let mut total = 0;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = source.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        target.write_all(&buffer[..count])?;
        digest.update(&buffer[..count]);
        total += count as u64;
    }
    Ok((total, format!("{:x}", digest.finalize())))
}

fn decrypt(source: impl Read, identity: &Identity) -> Result<(u64, String)> {
    let decryptor = age::Decryptor::new(source)?;
    let reader = decryptor.decrypt(std::iter::once(identity as &dyn age::Identity))?;
    transfer(reader, io::sink())
}

fn check(archive: &Path, identity: &Path) -> Result<(u64, String)> {
    decrypt(fs::File::open(archive)?, &key(identity, false)?)
}

fn create(distribution: &str, script: &str, archive: &Path, identity: &Path) -> Result<()> {
    if archive.exists() {
        return Err("refusing to replace an archive".into());
    }
    let identity_value = key(identity, true)?;
    let recipient = identity_value.to_public();
    let partial = archive.with_extension("age.partial");
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)?;
    let result = (|| -> Result<()> {
        let encryptor =
            age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))?;
        let mut writer = encryptor.wrap_output(file)?;
        let mut process = Command::new("wsl.exe")
            .args([
                "-d",
                distribution,
                "--user",
                "root",
                "--exec",
                "bash",
                script,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let source = process.stdout.take().ok_or("snapshot pipe unavailable")?;
        let copied = transfer(source, &mut writer);
        // Closing stdout on an error lets the snapshot trap clean up and restart.
        let status = process.wait()?;
        let expected = copied?;
        if !status.success() {
            return Err("consistent snapshot failed".into());
        }
        writer.finish()?.sync_all()?;
        let restored = decrypt(fs::File::open(&partial)?, &identity_value)?;
        if expected != restored {
            return Err("snapshot roundtrip mismatch".into());
        }
        fs::rename(&partial, archive)?;
        let (_, ciphertext) = transfer(fs::File::open(archive)?, io::sink())?;
        let report = serde_json::json!({
            "version": 1, "roundtrip_verified": true, "plaintext_bytes": expected.0,
            "archive_sha256": ciphertext, "plaintext_sha256": expected.1,
            "scope": "control-plane-state-and-ca", "live_restore_tested": false
        });
        let mut evidence = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(archive.with_extension("json"))?;
        evidence.write_all(&serde_json::to_vec_pretty(&report)?)?;
        evidence.sync_all()?;
        Ok(())
    })();
    if result.is_err() && partial.exists() {
        fs::remove_file(&partial)?;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (Identity, Vec<u8>) {
        let identity = Identity::generate();
        let encrypted = age::encrypt(&identity.to_public(), b"GNX snapshot test data").unwrap();
        (identity, encrypted)
    }

    #[test]
    fn checks_a_complete_roundtrip() {
        let (identity, encrypted) = sample();
        let original = transfer(&b"GNX snapshot test data"[..], io::sink()).unwrap();
        assert!(decrypt(&encrypted[..], &identity).unwrap() == original);
    }

    #[test]
    fn rejects_wrong_identity_and_modified_or_truncated_archives() {
        let (identity, mut encrypted) = sample();
        assert!(decrypt(&encrypted[..], &Identity::generate()).is_err());
        assert!(decrypt(&encrypted[..encrypted.len() - 1], &identity).is_err());
        let last = encrypted.len() - 1;
        encrypted[last] ^= 1;
        assert!(decrypt(&encrypted[..], &identity).is_err());
    }
}
