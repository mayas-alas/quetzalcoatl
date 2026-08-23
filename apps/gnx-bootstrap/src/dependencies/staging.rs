use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
};

use crate::windows::security;

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

pub struct StagedArtifact {
    path: PathBuf,
    _lock: File,
}

impl StagedArtifact {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl StageError {
    pub fn message(&self) -> &str {
        match self {
            Self::Missing(message) | Self::Invalid(message) | Self::Io(message) => message,
        }
    }
}

pub fn installer_root() -> Result<PathBuf, StageError> {
    let program_data = security::program_data().map_err(StageError::Io)?;
    security::secure_owned_tree(&program_data, &["Quetzalcoatl", "Installer"])
        .map_err(StageError::Io)
}

pub fn stage(request: &StageRequest<'_>) -> Result<StagedArtifact, StageError> {
    validate_fixed_name(request.file_name)?;
    let executable = env::current_exe()
        .map_err(|error| StageError::Io(format!("cannot resolve helper executable: {error}")))?;
    let source_root = executable
        .parent()
        .ok_or_else(|| StageError::Io("helper executable has no parent directory".into()))?;
    let source = source_root.join(request.file_name);
    validate_file(&source, request)?;

    validate_fixed_name(request.dependency_id)?;
    validate_fixed_name(request.version)?;
    let installer_root = installer_root()?;
    let destination_root = security::secure_owned_tree(
        &installer_root,
        &["cache", request.dependency_id, request.version],
    )
    .map_err(StageError::Io)?;

    let destination = destination_root.join(request.file_name);
    if destination.is_file() {
        if let Ok(locked) = open_validated_locked(&destination, request) {
            return Ok(locked);
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
    security::apply_protected_acl(&destination).map_err(StageError::Io)?;
    open_validated_locked(&destination, request)
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
    security::verify_real_file(path).map_err(StageError::Invalid)?;
    let mut file = File::open(path).map_err(|error| {
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
    validate_open_file(path, &mut file, request)
}

fn open_validated_locked(
    path: &Path,
    request: &StageRequest<'_>,
) -> Result<StagedArtifact, StageError> {
    let mut file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|error| {
            StageError::Io(format!(
                "cannot lock staged dependency {}: {error}",
                path.display()
            ))
        })?;
    validate_open_file(path, &mut file, request)?;
    Ok(StagedArtifact {
        path: path.to_path_buf(),
        _lock: file,
    })
}

fn validate_open_file(
    path: &Path,
    file: &mut File,
    request: &StageRequest<'_>,
) -> Result<(), StageError> {
    let metadata = file.metadata().map_err(|error| {
        StageError::Io(format!(
            "cannot inspect dependency payload {}: {error}",
            path.display()
        ))
    })?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(StageError::Invalid(format!(
            "dependency payload is a reparse point at {}",
            path.display()
        )));
    }
    if !metadata.is_file() || metadata.len() != request.expected_size {
        return Err(StageError::Invalid(format!(
            "dependency payload size differs at {}",
            path.display()
        )));
    }
    file.seek(SeekFrom::Start(0)).map_err(|error| {
        StageError::Io(format!(
            "cannot seek dependency payload {}: {error}",
            path.display()
        ))
    })?;
    let actual = sha256(file, path)?;
    if !actual.eq_ignore_ascii_case(request.expected_sha256) {
        return Err(StageError::Invalid(format!(
            "dependency payload SHA-256 differs at {}",
            path.display()
        )));
    }
    Ok(())
}

fn sha256(file: &mut File, path: &Path) -> Result<String, StageError> {
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
