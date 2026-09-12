//! Installer executable. The installation script is compiled into the signed/hash-verified artifact.
#[cfg(windows)]
fn main() {
    use sha2::{Digest, Sha256};
    use std::io::Write;
    use std::process::{Command, Stdio};
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--version"] {
        println!("gnx-install {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let install = args.len() == 4 && args[0] == "--manifest-sha256" && args[2] == "--rootfs";
    let verify = args.len() == 3 && args[0] == "verify" && args[1] == "--manifest-sha256";
    let update = args.len() == 3 && args[0] == "update" && args[1] == "--manifest-sha256";
    let rollback = args.len() == 5
        && args[0] == "rollback"
        && args[1] == "--manifest-sha256"
        && args[3] == "--confirm"
        && args[4] == "ROLLBACK-GNX";
    let uninstall = args == ["uninstall", "--confirm", "REMOVE-GNX-AND-DATA"];
    let operation = if verify {
        "verify-release"
    } else if update {
        "update"
    } else if rollback {
        "rollback"
    } else {
        "install"
    };
    if !install && !verify && !update && !rollback && !uninstall {
        eprintln!("Install: gnx-install.exe --manifest-sha256 HASH --rootfs FILE");
        eprintln!("Verify: gnx-install.exe verify --manifest-sha256 HASH");
        eprintln!("Update: gnx-install.exe update --manifest-sha256 HASH");
        eprintln!(
            "Rollback: gnx-install.exe rollback --manifest-sha256 HASH --confirm ROLLBACK-GNX"
        );
        eprintln!("Uninstall: gnx-install.exe uninstall --confirm REMOVE-GNX-AND-DATA");
        std::process::exit(2);
    }
    let authenticated = if install || verify || update || rollback {
        let path = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let manifest = std::fs::read(path.join("manifest.json"))
            .unwrap_or_else(|_| installer_failure(operation, "RELEASE_MANIFEST_UNREADABLE"));
        let signature = std::fs::read(path.join("manifest.json.sig"))
            .unwrap_or_else(|_| installer_failure(operation, "RELEASE_SIGNATURE_MISSING"));
        let digest = hex::encode(Sha256::digest(&manifest));
        let expected_digest = if install { &args[1] } else { &args[2] };
        if !expected_digest.eq_ignore_ascii_case(&digest) {
            installer_failure(operation, "MANIFEST_AUTHENTICATION_FAILED");
        }
        let public = include_str!("../../packaging/release/trusted-release.pub").trim();
        let key_id = gnx::release_auth::verify(&manifest, &signature, public)
            .unwrap_or_else(|code| installer_failure(operation, &code));
        let document = validate_manifest_schema(&manifest)
            .unwrap_or_else(|code| installer_failure(operation, code));
        if document
            .get("signing_key_id")
            .and_then(|value| value.as_str())
            != Some(&key_id)
        {
            installer_failure(operation, "RELEASE_SIGNER_UNTRUSTED");
        }
        if install || update || rollback {
            let rootfs = install.then(|| std::path::Path::new(&args[3]));
            validate_release_artifacts(&path, &document, rootfs)
                .unwrap_or_else(|code| installer_failure(operation, code));
        }
        Some((path, key_id, digest))
    } else {
        None
    };
    if verify {
        let (_, key_id, manifest_sha256) = authenticated.unwrap();
        println!(
            "{}",
            serde_json::json!({"schema":1,"operation":"verify-release","state":"READY","code":"RELEASE_AUTHENTIC","key_id":key_id,"manifest_sha256":manifest_sha256,"next_action":null})
        );
        return;
    }
    if !is_elevated() {
        match elevate(&args) {
            Ok(code) => std::process::exit(code),
            Err(code) => {
                installer_failure(operation, &format!("UAC_REQUIRED_{code}"));
            }
        }
    }
    let mut command =
        Command::new("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe");
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", "-"])
        .stdin(Stdio::piped());
    if install || update || rollback {
        let (bundle, _, _) = authenticated.unwrap();
        command
            .env("GNX_INSTALL_BUNDLE", bundle)
            .env(
                "GNX_INSTALL_MANIFEST",
                if install { &args[1] } else { &args[2] },
            )
            .env("GNX_INSTALL_ACTION", operation);
        if install {
            command.env("GNX_INSTALL_ROOTFS", &args[3]);
        }
    } else {
        command.env("GNX_UNINSTALL_CONFIRM", &args[2]);
    }
    let mut child = command.spawn().expect("start installer");
    // Only public paths, digests and the destructive confirmation cross through the
    // environment. Credentials stay in SCM.
    let mut input = child.stdin.take().unwrap();
    let script: &[u8] = if install {
        include_bytes!("../../packaging/windows/setup.ps1")
    } else if update || rollback {
        include_bytes!("../../packaging/windows/update.ps1")
    } else {
        include_bytes!("../../packaging/windows/uninstall.ps1")
    };
    input.write_all(script).expect("installer input");
    input.write_all(b"\n").expect("installer terminator");
    drop(input);
    std::process::exit(child.wait().expect("installer result").code().unwrap_or(1));
}

#[cfg(windows)]
fn validate_manifest_schema(bytes: &[u8]) -> Result<serde_json::Value, &'static str> {
    let document: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "MANIFEST_SCHEMA_INVALID")?;
    let valid = document.get("schema").and_then(|v| v.as_u64()) == Some(1)
        && document
            .get("version")
            .and_then(|v| v.as_str())
            .is_some_and(|v| !v.is_empty())
        && document
            .get("release_serial")
            .and_then(|v| v.as_u64())
            .is_some_and(|v| v > 0)
        && document
            .get("signing_key_id")
            .and_then(|v| v.as_str())
            .is_some_and(valid_digest)
        && document.get("artifacts").is_some_and(|v| v.is_object())
        && document.get("rootfs").is_some_and(|v| v.is_object());
    valid.then_some(document).ok_or("MANIFEST_SCHEMA_INVALID")
}

#[cfg(windows)]
fn validate_release_artifacts(
    bundle: &std::path::Path,
    document: &serde_json::Value,
    rootfs: Option<&std::path::Path>,
) -> Result<(), &'static str> {
    use sha2::{Digest, Sha256};
    for name in [
        "gnx.exe",
        "gnx-service.exe",
        "gnx-install.exe",
        "gnx-linux-bundle.tar",
    ] {
        let expected = document
            .get("artifacts")
            .and_then(|v| v.get(name))
            .and_then(|v| v.as_str())
            .filter(|v| valid_digest(v))
            .ok_or("MANIFEST_SCHEMA_INVALID")?;
        let bytes = std::fs::read(bundle.join(name)).map_err(|_| "ARTIFACT_MISMATCH")?;
        if !expected.eq_ignore_ascii_case(&hex::encode(Sha256::digest(bytes))) {
            return Err("ARTIFACT_MISMATCH");
        }
    }
    if let Some(rootfs) = rootfs {
        let expected = document
            .get("rootfs")
            .and_then(|v| v.get("sha256"))
            .and_then(|v| v.as_str())
            .filter(|v| valid_digest(v))
            .ok_or("MANIFEST_SCHEMA_INVALID")?;
        let bytes = std::fs::read(rootfs).map_err(|_| "ROOTFS_MISMATCH")?;
        if !expected.eq_ignore_ascii_case(&hex::encode(Sha256::digest(bytes))) {
            return Err("ROOTFS_MISMATCH");
        }
    }
    Ok(())
}

#[cfg(windows)]
fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(windows)]
fn installer_failure(operation: &str, code: &str) -> ! {
    let next_action = match code {
        "MANIFEST_AUTHENTICATION_FAILED"
        | "RELEASE_SIGNATURE_INVALID"
        | "RELEASE_SIGNATURE_MISSING"
        | "RELEASE_SIGNER_UNTRUSTED" => {
            "Obtain the manifest and digest from the authenticated GNX release."
        }
        "MANIFEST_SCHEMA_INVALID" => "Use a supported GNX release manifest.",
        "ARTIFACT_MISMATCH" | "ROOTFS_MISMATCH" => {
            "Replace the release bundle with an exact authenticated copy."
        }
        _ => "Inspect the GNX release bundle and retry.",
    };
    println!(
        "{}",
        serde_json::json!({"schema":1,"operation":operation,"state":"FAILED","code":code,"next_action":next_action})
    );
    std::process::exit(1)
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
