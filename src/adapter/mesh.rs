use std::{
    ffi::OsString,
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{Error, Result, config::Artifact, port::Mesh};
use sha2::{Digest, Sha256};

pub struct NativeMesh;

impl NativeMesh {
    fn program(&self) -> PathBuf {
        PathBuf::from(r"C:\Program Files\NetBird\netbird.exe")
    }

    fn command(&self) -> Command {
        let mut command = Command::new(self.program());
        for name in ["NB_MANAGEMENT_URL", "NB_SETUP_KEY", "NB_SETUP_KEY_FILE"] {
            command.env_remove(name);
        }
        command
    }
}

impl Mesh for NativeMesh {
    fn installed_version(&self) -> Result<Option<String>> {
        let output = match self.command().arg("version").output() {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(Error::Spawn(error)),
        };
        if !output.status.success() {
            return Err(Error::External {
                operation: "CLIENT_VERSION",
                code: output.status.code().unwrap_or(-1),
            });
        }
        let text = String::from_utf8_lossy(&output.stdout);
        parse_version(&text).map(Some).ok_or(Error::ClientVersion)
    }

    fn install(&self, artifact: &Artifact) -> Result<()> {
        if !artifact.license.is_file() {
            return Err(Error::ReleaseEvidence);
        }
        verify_digest(&artifact.package, &artifact.sha256)?;
        let package = if artifact.package.is_absolute() {
            artifact.package.clone()
        } else {
            std::env::current_dir()
                .map_err(Error::PackageRead)?
                .join(&artifact.package)
        };
        let status = Command::new("msiexec.exe")
            .arg("/i")
            .arg(package)
            .args(["/quiet", "/norestart", "/L*v"])
            .arg(std::env::temp_dir().join("gnx-mesh-client-install.log"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(Error::Spawn)?;
        match status.code() {
            Some(0 | 1641 | 3010) => Ok(()),
            code => Err(Error::External {
                operation: "CLIENT_INSTALL",
                code: code.unwrap_or(-1),
            }),
        }
    }

    fn connect(&self, endpoint: &str, setup_key_file: Option<&Path>) -> Result<()> {
        let mut command = self.command();
        command
            .args(connect_args(endpoint, setup_key_file))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let status = command.status().map_err(Error::Spawn)?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::External {
                operation: "CLIENT_CONNECT",
                code: status.code().unwrap_or(-1),
            })
        }
    }

    fn ready(&self) -> Result<()> {
        let status = self
            .command()
            .args(["status", "--check", "startup"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(Error::Spawn)?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::External {
                operation: "CLIENT_READY",
                code: status.code().unwrap_or(-1),
            })
        }
    }
}

fn verify_digest(path: &Path, expected: &str) -> Result<()> {
    let file = File::open(path).map_err(Error::PackageRead)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer).map_err(Error::PackageRead)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual == expected {
        Ok(())
    } else {
        Err(Error::PackageDigest)
    }
}

fn parse_version(text: &str) -> Option<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+')))
        .map(|part| part.trim_start_matches('v'))
        .find(|part| {
            let core = part.split(['-', '+']).next().unwrap_or("");
            let pieces: Vec<_> = core.split('.').collect();
            pieces.len() >= 3
                && pieces
                    .iter()
                    .all(|piece| !piece.is_empty() && piece.bytes().all(|b| b.is_ascii_digit()))
        })
        .map(str::to_owned)
}

fn connect_args(endpoint: &str, setup_key_file: Option<&Path>) -> Vec<OsString> {
    let mut args = vec!["up".into(), "--management-url".into(), endpoint.into()];
    if let Some(path) = setup_key_file {
        args.push("--setup-key-file".into());
        args.push(path.as_os_str().into());
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_only_a_version_token() {
        assert_eq!(
            parse_version("netbird version v0.75.1\n"),
            Some("0.75.1".into())
        );
        assert_eq!(parse_version("unexpected"), None);
    }

    #[test]
    fn preserves_the_endpoint_and_only_passes_the_key_path() {
        let args = connect_args("https://mesh.gnx", Some(Path::new("secret.key")));
        assert_eq!(
            args,
            [
                "up",
                "--management-url",
                "https://mesh.gnx",
                "--setup-key-file",
                "secret.key"
            ]
            .map(OsString::from)
        );
    }
}
