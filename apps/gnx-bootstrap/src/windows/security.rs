use std::fs::{self, File, OpenOptions};
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    SECURITY_ATTRIBUTES, SetFileSecurityW,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateDirectoryW, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
};
use winreg::RegKey;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};

use gnx_contracts::WINDOWS_SERVICE_SID;

const SHELL_FOLDERS: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Shell Folders";

pub(crate) fn program_data() -> Result<PathBuf, String> {
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(SHELL_FOLDERS, KEY_READ | KEY_WOW64_64KEY)
        .map_err(|error| format!("cannot open machine shell folders: {error}"))?;
    let path: String = key
        .get_value("Common AppData")
        .map_err(|error| format!("cannot read machine Common AppData: {error}"))?;
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return Err("machine Common AppData is not an absolute path".into());
    }
    verify_real_directory(&path)?;
    Ok(path)
}

pub(crate) fn secure_owned_tree(root: &Path, components: &[&str]) -> Result<PathBuf, String> {
    verify_real_directory(root)?;
    let mut current = root.to_path_buf();
    for component in components {
        if component.is_empty()
            || Path::new(component).components().count() != 1
            || Path::new(component)
                .file_name()
                .and_then(|name| name.to_str())
                != Some(component)
        {
            return Err("protected directory component is not a fixed name".into());
        }
        current.push(component);
        secure_directory(&current)?;
    }
    Ok(current)
}

pub(crate) fn apply_protected_acl(path: &Path) -> Result<(), String> {
    apply_protected_acl_inner(path, false)
}

fn apply_protected_acl_inner(path: &Path, directory: bool) -> Result<(), String> {
    let lock = lock_path(path)?;
    verify_real_metadata(
        path,
        &lock.metadata().map_err(|error| {
            format!("cannot inspect protected path {}: {error}", path.display())
        })?,
        directory,
    )?;
    let descriptor = SecurityDescriptor::new()?;
    let wide = wide_path(path)?;
    // Safety: the locked path cannot be renamed/deleted, the path is NUL-terminated and the
    // descriptor remains valid for the duration of SetFileSecurityW.
    if unsafe {
        SetFileSecurityW(
            wide.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor.0,
        )
    } == 0
    {
        return Err(last_error(&format!(
            "cannot apply protected ACL to {}",
            path.display()
        )));
    }
    verify_real_metadata(
        path,
        &lock.metadata().map_err(|error| {
            format!(
                "cannot re-inspect protected path {}: {error}",
                path.display()
            )
        })?,
        directory,
    )
}

pub(crate) fn verify_real_file(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(format!(
            "protected path is not a real file: {}",
            path.display()
        ));
    }
    Ok(())
}

fn secure_directory(path: &Path) -> Result<(), String> {
    let descriptor = SecurityDescriptor::new()?;
    let wide = wide_path(path)?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    // Safety: path and descriptor are valid, NUL-terminated values for this call.
    if unsafe { CreateDirectoryW(wide.as_ptr(), &attributes) } == 0 {
        let error = unsafe { GetLastError() };
        if error != ERROR_ALREADY_EXISTS {
            return Err(format!(
                "cannot create protected directory {} (Win32 {error})",
                path.display()
            ));
        }
    }
    apply_protected_acl_inner(path, true)
}

fn verify_real_directory(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect directory {}: {error}", path.display()))?;
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(format!("path is not a real directory: {}", path.display()));
    }
    Ok(())
}

fn verify_real_metadata(
    path: &Path,
    metadata: &fs::Metadata,
    directory: bool,
) -> Result<(), String> {
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || (directory && !metadata.is_dir())
        || (!directory && !metadata.is_file())
    {
        return Err(format!(
            "protected path has an invalid type: {}",
            path.display()
        ));
    }
    Ok(())
}

fn lock_path(path: &Path) -> Result<File, String> {
    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|error| format!("cannot lock protected path {}: {error}", path.display()))
}

fn wide_path(path: &Path) -> Result<Vec<u16>, String> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    if wide.len() > 32_768 {
        Err("protected path exceeds the Windows path limit".into())
    } else {
        Ok(wide)
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

fn last_error(context: &str) -> String {
    format!("{context} (Win32 {})", unsafe { GetLastError() })
}

struct SecurityDescriptor(PSECURITY_DESCRIPTOR);

impl SecurityDescriptor {
    fn new() -> Result<Self, String> {
        let sddl = wide(&format!(
            "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;GRGX;;;{WINDOWS_SERVICE_SID})"
        ));
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
            Err(last_error("cannot create installer security descriptor"))
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

#[cfg(test)]
mod tests {
    use std::os::windows::fs::symlink_dir;

    use super::*;

    #[test]
    #[ignore = "requires an elevated Windows token to create and remove the protected fixture"]
    fn rejects_a_reparse_point_in_an_owned_directory_component() {
        let program_data = program_data().expect("resolve ProgramData");
        let fixture_name = format!(
            "Quetzalcoatl.SecurityTest.{}.{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        );
        let fixture =
            secure_owned_tree(&program_data, &[&fixture_name]).expect("create protected fixture");
        let target = std::env::temp_dir().join(format!("{fixture_name}.target"));
        fs::create_dir(&target).expect("create reparse target");
        let reparse = fixture.join("cache");
        symlink_dir(&target, &reparse).expect("create directory reparse point");

        let error = secure_owned_tree(&fixture, &["cache"])
            .expect_err("reparse component must fail closed");
        assert!(error.contains("invalid type") || error.contains("reparse"));

        fs::remove_dir(&reparse).expect("remove reparse fixture");
        fs::remove_dir(&fixture).expect("remove protected fixture");
        fs::remove_dir(&target).expect("remove reparse target");
    }
}
