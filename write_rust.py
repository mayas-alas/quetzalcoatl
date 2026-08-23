f = open(r'C:\Users\mayas\quetzalcoatl\apps\gnx-service\src\infrastructure\podman.rs', 'w')
f.write('use std::env;' + '
')
f.write('use std::ffi::OsString;' + '
')
f.write('use std::fs::{self, File};' + '
')
f.write('use std::io::Read;' + '
')
f.write('use std::path::{Path, PathBuf};' + '
')
f.write('use std::ptr::null_mut;' + '
')
f.write('use std::thread;' + '
')
f.write('use std::time::Duration;' + '
')
f.write('' + '
')
f.write('use gnx_contracts::MachineProfile;' + '
')
f.write('use sha2::{Digest, Sha256};' + '
')
f.write('use windows_sys::Win32::Foundation::{' + '
')
f.write('    CloseHandle, GetLastError, ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_PIPE_BUSY,' + '
')
f.write('    INVALID_HANDLE_VALUE,' + '
')
f.write('};' + '
')
f.write('use windows_sys::Win32::Storage::FileSystem::{' + '
')
f.write('    CreateFileW, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,' + '
')
f.write('};' + '
')
f.write('use zeroize::Zeroizing;' + '
')

