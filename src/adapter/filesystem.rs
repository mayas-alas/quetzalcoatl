use crate::{
    config::Config,
    port::state::{StateStore, TransactionGuard},
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
pub struct Filesystem {
    pub root: PathBuf,
}
pub fn private_dir(p: &Path) -> Result<(), String> {
    if p.symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        return Err("STATE_SYMLINK_REFUSED".into());
    }
    fs::create_dir_all(p).map_err(|_| "STATE_CREATE_FAILED")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let m = fs::metadata(p).map_err(|_| "STATE_STAT_FAILED")?;
        if m.uid() != unsafe { libc::geteuid() } {
            return Err("STATE_OWNER_MISMATCH".into());
        }
        fs::set_permissions(p, fs::Permissions::from_mode(0o700))
            .map_err(|_| "STATE_PERMISSIONS_FAILED")?;
    }
    Ok(())
}
pub fn atomic_write(p: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    let parent = p.parent().ok_or("STATE_PATH_INVALID")?;
    let temp = p.with_extension("gnx-new");
    let mut o = OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(mode).custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(not(unix))]
    let _ = mode;
    if temp
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        return Err("STATE_SYMLINK_REFUSED".into());
    }
    let mut f = o.open(&temp).map_err(|_| "STATE_WRITE_FAILED")?;
    f.write_all(bytes).map_err(|_| "STATE_WRITE_FAILED")?;
    f.sync_all().map_err(|_| "STATE_SYNC_FAILED")?;
    drop(f);
    replace(&temp, p)?;
    sync_dir(parent)
}
fn replace(from: &Path, to: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let a: Vec<_> = from.as_os_str().encode_wide().chain(Some(0)).collect();
        let b: Vec<_> = to.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                a.as_ptr(),
                b.as_ptr(),
                windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                    | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err("STATE_PROMOTE_FAILED".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        fs::rename(from, to).map_err(|_| "STATE_PROMOTE_FAILED".into())
    }
}
fn sync_dir(p: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        fs::File::open(p)
            .and_then(|f| f.sync_all())
            .map_err(|_| "STATE_SYNC_FAILED")?;
    }
    let _ = p;
    Ok(())
}
impl StateStore for Filesystem {
    fn current(&self) -> Result<Option<String>, String> {
        Ok(self.previous()?.map(|c| c.revision()))
    }
    fn previous(&self) -> Result<Option<Config>, String> {
        match fs::read_to_string(self.root.join("last-valid.toml")) {
            Ok(s) => Ok(Some(Config::parse(&s)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err("STATE_READ_FAILED".into()),
        }
    }
    fn acquire(&self) -> Result<Box<dyn TransactionGuard>, String> {
        private_dir(&self.root)?;
        let mut o = OpenOptions::new();
        o.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            o.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        let f = o
            .open(self.root.join("apply.lock"))
            .map_err(|_| "LOCK_OPEN_FAILED")?;
        fs2::FileExt::try_lock_exclusive(&f).map_err(|_| "APPLY_BUSY")?;
        Ok(Box::new(f))
    }
    fn interrupted(&self) -> Result<bool, String> {
        Ok(self.root.join("transaction.json").exists())
    }
    fn stage(&self, c: &Config) -> Result<(), String> {
        atomic_write(
            &self.root.join("candidate.toml"),
            toml::to_string(c)
                .map_err(|_| "CONFIG_ENCODE_FAILED")?
                .as_bytes(),
            0o600,
        )?;
        self.phase("staged")
    }
    fn phase(&self, phase: &str) -> Result<(), String> {
        atomic_write(
            &self.root.join("transaction.json"),
            serde_json::to_string(&serde_json::json!({"schema":1,"phase":phase}))
                .unwrap()
                .as_bytes(),
            0o600,
        )
    }
    fn promote(&self) -> Result<(), String> {
        replace(
            &self.root.join("candidate.toml"),
            &self.root.join("last-valid.toml"),
        )?;
        sync_dir(&self.root)?;
        self.abort()
    }
    fn abort(&self) -> Result<(), String> {
        for name in ["candidate.toml", "transaction.json"] {
            match fs::remove_file(self.root.join(name)) {
                Ok(()) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(_) => return Err("STATE_CLEANUP_FAILED".into()),
            }
        }
        sync_dir(&self.root)
    }
}
