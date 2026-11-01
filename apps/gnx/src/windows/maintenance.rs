use super::{bootstrap, security};
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt},
};
use windows_sys::Win32::Storage::FileSystem::{
    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
};

// Shared by the pipe controller and the one-shot installer worker. It serializes
// native work across processes, including an MSI-started replacement service.
pub(super) fn lease() -> io::Result<File> {
    let root = bootstrap::private_root()?;
    let _parent = security::private_directory()?;
    match security::private_file("maintenance.lock") {
        Ok(file) => drop(file),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(0)
        .open(root.join("maintenance.lock"))
}

pub(super) fn write(name: &str, bytes: &[u8]) -> io::Result<()> {
    // Only fixed, source-code filenames are accepted by callers.
    let root = bootstrap::private_root()?;
    let _parent = security::private_directory()?;
    let temporary = root.join(format!("{name}.new"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    let from: Vec<_> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<_> = root
        .join(name)
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    if unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
