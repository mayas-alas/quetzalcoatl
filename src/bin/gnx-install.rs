//! Installer executable. The installation script is compiled into the signed/hash-verified artifact.
#[cfg(windows)]
fn main() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--version"] {
        println!("gnx-install {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.len() != 4 || args[0] != "--manifest-sha256" || args[2] != "--rootfs" {
        eprintln!("Run elevated: gnx-install.exe --manifest-sha256 HASH --rootfs FILE");
        std::process::exit(2);
    }
    if !is_elevated() {
        match elevate(&args) {
            Ok(code) => std::process::exit(code),
            Err(code) => {
                eprintln!("UAC_REQUIRED: approve the GNX installer elevation ({code})");
                std::process::exit(1);
            }
        }
    }
    let bundle = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let mut child = Command::new("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", "-"])
        .env("GNX_INSTALL_BUNDLE", &bundle)
        .env("GNX_INSTALL_MANIFEST", &args[1])
        .env("GNX_INSTALL_ROOTFS", &args[3])
        .stdin(Stdio::piped())
        .spawn()
        .expect("start installer");
    // Only public paths and digests cross through the environment. Credentials stay in SCM.
    let mut input = child.stdin.take().unwrap();
    input
        .write_all(include_bytes!("../../packaging/windows/setup.ps1"))
        .expect("installer input");
    input.write_all(b"\n").expect("installer terminator");
    drop(input);
    std::process::exit(child.wait().expect("installer result").code().unwrap_or(1));
}

#[cfg(windows)]
fn is_elevated() -> bool {
    unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
}

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

#[cfg(windows)]
fn elevate(args: &[String]) -> Result<i32, u32> {
    use std::mem::size_of;
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE},
        UI::{
            Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW},
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    };
    let exe = std::env::current_exe().map_err(|_| 1u32)?;
    let parameters = args
        .iter()
        .map(|a| format!("\"{}\"", a.replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(" ");
    let file = wide(&exe.to_string_lossy());
    let verb = wide("runas");
    let parameters = wide(&parameters);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        hwnd: std::ptr::null_mut(),
        lpVerb: verb.as_ptr(),
        lpFile: file.as_ptr(),
        lpParameters: parameters.as_ptr(),
        lpDirectory: std::ptr::null(),
        nShow: SW_SHOWNORMAL,
        hInstApp: std::ptr::null_mut(),
        lpIDList: std::ptr::null_mut(),
        lpClass: std::ptr::null(),
        hkeyClass: std::ptr::null_mut(),
        dwHotKey: 0,
        Anonymous: unsafe { std::mem::zeroed() },
        hProcess: std::ptr::null_mut(),
    };
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        return Err(unsafe { windows_sys::Win32::Foundation::GetLastError() });
    }
    if info.hProcess.is_null() {
        return Ok(0);
    }
    unsafe { WaitForSingleObject(info.hProcess, INFINITE) };
    let mut code = 1;
    let _ = unsafe { GetExitCodeProcess(info.hProcess, &mut code) };
    unsafe { CloseHandle(info.hProcess) };
    Ok(code as i32)
}
#[cfg(not(windows))]
fn main() {
    eprintln!("Use gnx-linux.run on Linux.");
    std::process::exit(2);
}
