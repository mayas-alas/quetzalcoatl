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
#[cfg(not(windows))]
fn main() {
    eprintln!("Use gnx-linux.run on Linux.");
    std::process::exit(2);
}
