use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::path::PathBuf;

use windows_sys::Wdk::System::SystemServices::RtlGetVersion;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
use windows_sys::Win32::Security::{
    GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
};
use windows_sys::Win32::System::SystemInformation::{
    GetNativeSystemInfo, GetSystemDirectoryW, OSVERSIONINFOW, SYSTEM_INFO,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, IsProcessorFeaturePresent, OpenProcessToken, PF_VIRT_FIRMWARE_ENABLED,
};

pub fn windows_11_x64() -> Result<bool, String> {
    // Safety: the initialized structures and pointers have the exact Win32 API sizes.
    unsafe {
        let mut version: OSVERSIONINFOW = zeroed();
        version.dwOSVersionInfoSize = size_of::<OSVERSIONINFOW>() as u32;
        if RtlGetVersion(&mut version) != 0 {
            return Err("RtlGetVersion failed".into());
        }
        let mut system: SYSTEM_INFO = zeroed();
        GetNativeSystemInfo(&mut system);
        let architecture = *(&system as *const SYSTEM_INFO as *const u16);
        Ok(version.dwMajorVersion == 10 && version.dwBuildNumber >= 22000 && architecture == 9)
    }
}

pub fn is_elevated() -> Result<bool, String> {
    // Safety: the token is closed once and TOKEN_ELEVATION is a correctly sized output buffer.
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(last_error("OpenProcessToken"));
        }
        let mut elevation: TOKEN_ELEVATION = zeroed();
        let mut return_length = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut c_void,
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        );
        let error = if ok == 0 { Some(GetLastError()) } else { None };
        CloseHandle(token);
        if let Some(error) = error {
            return Err(format!(
                "GetTokenInformation failed with Win32 error {error}"
            ));
        }
        Ok(elevation.TokenIsElevated != 0)
    }
}

pub fn virtualization_available() -> Result<bool, String> {
    // Safety: IsProcessorFeaturePresent takes a documented feature identifier and CPUID leaf 1
    // is available on every x86_64 processor supported by this Windows-only binary.
    unsafe {
        Ok(IsProcessorFeaturePresent(PF_VIRT_FIRMWARE_ENABLED) != 0
            && std::arch::x86_64::__cpuid(1).ecx & (1 << 31) != 0)
    }
}

pub fn system32_file(name: &str) -> Result<PathBuf, String> {
    // Safety: the buffer is writable and its length is passed exactly to GetSystemDirectoryW.
    unsafe {
        let mut buffer = vec![0u16; 260];
        let mut written = GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) as usize;
        if written == 0 {
            return Err(last_error("GetSystemDirectoryW"));
        }
        if written >= buffer.len() {
            buffer.resize(written + 1, 0);
            written = GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) as usize;
            if written == 0 || written >= buffer.len() {
                return Err("GetSystemDirectoryW returned an invalid length".into());
            }
        }
        buffer.truncate(written);
        let directory = String::from_utf16(&buffer)
            .map_err(|_| "GetSystemDirectoryW returned invalid UTF-16".to_string())?;
        Ok(PathBuf::from(directory).join(name))
    }
}

fn last_error(operation: &str) -> String {
    // Safety: GetLastError has no parameters and is called immediately after a failing Win32 call.
    unsafe { format!("{operation} failed with Win32 error {}", GetLastError()) }
}
