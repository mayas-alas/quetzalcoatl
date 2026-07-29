use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

pub struct StageRequest<'a> {
    pub dependency_id: &'a str,
    pub version: &'a str,
    pub file_name: &'a str,
    pub expected_size: u64,
    pub expected_sha256: &'a str,
}

#[derive(Debug)]
pub enum StageError {
    Missing(String),
    Invalid(String),
    Io(String),
}

impl StageError {
    pub fn message(&self) -> &str {
        match self {
            Self::Missing(message) | Self::Invalid(message) | Self::Io(message) => message,
        }
    }
}

pub fn installer_root() -> Result<PathBuf, StageError> {
    let program_data = env::var_os("ProgramData")
        .map(PathBuf::from)
        .ok_or_else(|| StageError::Io("ProgramData is unavailable".into()))?;
    if !program_data.is_absolute() {
        return Err(StageError::Io("ProgramData is not an absolute path".into()));
    }
    Ok(program_data.join("Quetzalcoatl").join("Installer"))
}

pub fn stage(request: &StageRequest<'_>) -> Result<PathBuf, StageError> {
    validate_fixed_name(request.file_name)?;
    let executable = env::current_exe()
        .map_err(|error| StageError::Io(format!("cannot resolve helper executable: {error}")))?;
    let source_root = executable
        .parent()
        .ok_or_else(|| StageError::Io("helper executable has no parent directory".into()))?;
    let source = source_root.join(request.file_name);
    validate_file(&source, request)?;

    let destination_root = installer_root()?
        .join("cache")
        .join(request.dependency_id)
        .join(request.version);
    fs::create_dir_all(&destination_root).map_err(|error| {
        StageError::Io(format!(
            "cannot create stable dependency cache {}: {error}",
            destination_root.display()
        ))
    })?;
    reject_symlink(&destination_root)?;

    let destination = destination_root.join(request.file_name);
    if destination.is_file() {
        if validate_file(&destination, request).is_ok() {
            return Ok(destination);
        }
        let invalid = destination.with_extension("msi.invalid");
        let _ = fs::remove_file(&invalid);
        fs::rename(&destination, &invalid).map_err(|error| {
            StageError::Io(format!(
                "cannot quarantine invalid staged payload {}: {error}",
                destination.display()
            ))
        })?;
    }

    let partial = destination.with_extension("msi.partial");
    let _ = fs::remove_file(&partial);
    copy_exclusive(&source, &partial)?;
    validate_file(&partial, request)?;
    fs::rename(&partial, &destination).map_err(|error| {
        StageError::Io(format!(
            "cannot activate staged payload {}: {error}",
            destination.display()
        ))
    })?;
    validate_file(&destination, request)?;
    Ok(destination)
}

fn validate_fixed_name(value: &str) -> Result<(), StageError> {
    let path = Path::new(value);
    if path.components().count() != 1
        || path.file_name().and_then(|name| name.to_str()) != Some(value)
    {
        return Err(StageError::Invalid(
            "dependency payload name is not a fixed file name".into(),
        ));
    }
    Ok(())
}

fn reject_symlink(path: &Path) -> Result<(), StageError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| StageError::Io(format!("cannot inspect {}: {error}", path.display())))?;
    if metadata.file_type().is_symlink() {
        return Err(StageError::Invalid(format!(
            "stable dependency cache is a symbolic link: {}",
            path.display()
        )));
    }
    Ok(())
}

fn copy_exclusive(source: &Path, destination: &Path) -> Result<(), StageError> {
    let mut input = File::open(source).map_err(|error| {
        StageError::Missing(format!(
            "cannot open bundled payload {}: {error}",
            source.display()
        ))
    })?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|error| {
            StageError::Io(format!(
                "cannot create staged payload {}: {error}",
                destination.display()
            ))
        })?;
    std::io::copy(&mut input, &mut output).map_err(|error| {
        StageError::Io(format!(
            "cannot copy payload to {}: {error}",
            destination.display()
        ))
    })?;
    output.flush().map_err(|error| {
        StageError::Io(format!("cannot flush {}: {error}", destination.display()))
    })?;
    output.sync_all().map_err(|error| {
        StageError::Io(format!("cannot sync {}: {error}", destination.display()))
    })?;
    Ok(())
}

fn validate_file(path: &Path, request: &StageRequest<'_>) -> Result<(), StageError> {
    let metadata = fs::metadata(path).map_err(|error| {
        let message = format!(
            "dependency payload is unavailable at {}: {error}",
            path.display()
        );
        if error.kind() == std::io::ErrorKind::NotFound {
            StageError::Missing(message)
        } else {
            StageError::Io(message)
        }
    })?;
    if !metadata.is_file() || metadata.len() != request.expected_size {
        return Err(StageError::Invalid(format!(
            "dependency payload size differs at {}",
            path.display()
        )));
    }
    let actual = sha256(path)?;
    if !actual.eq_ignore_ascii_case(request.expected_sha256) {
        return Err(StageError::Invalid(format!(
            "dependency payload SHA-256 differs at {}",
            path.display()
        )));
    }
    Ok(())
}

fn sha256(path: &Path) -> Result<String, StageError> {
    let mut file = File::open(path)
        .map_err(|error| StageError::Io(format!("cannot hash {}: {error}", path.display())))?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| StageError::Io(format!("cannot hash {}: {error}", path.display())))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
