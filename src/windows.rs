use crate::protocol::*;
pub mod installer;
use std::os::windows::process::CommandExt;
use crate::{CLI_EXE, PRODUCT, SETUP_EXE};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{ffi::c_void, fs, io::{Read, Seek}, mem::{size_of, zeroed}, path::{Path, PathBuf}, process::{Command, Stdio}, ptr::{null, null_mut}, sync::{atomic::{AtomicBool, Ordering}, Mutex}, thread, time::{Duration, Instant}};
use windows_sys::Win32::{Foundation::*, Security::{*, Authorization::*, Authentication::Identity::*, Cryptography::*}, NetworkManagement::NetManagement::*, Storage::FileSystem::*, System::{Pipes::*, Services::*, Threading::*}};

// Private installation identifiers; changing them requires a migration.
const SERVICE: &str = "QuetzalcoatlGNX";
const SERVICE_DISPLAY: &str = "Quetzalcoatl GNX - Consultas WSL";
const ACCOUNT: &str = "svc_quetzalcoatl_gnx";
const DISTRO: &str = "quetzalcoatl-gnx";
const LINUX_USER: &str = "quetzalcoatl-gnx";
const PIPE: &str = r"\\.\pipe\quetzalcoatl-gnx-control-v1";
const BASE: &str = r"C:\ProgramData\QuetzalcoatlGNX";
const MAX_MESSAGE: usize = 8192;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config { client_sid: String, account_sid: String, #[serde(default)] rootfs_sha256: String }

fn valid_sid(s: &str) -> bool {
    crate::installer::valid_client_sid(s)
}

fn bootstrap() -> String {
    include_str!("bootstrap.sh").replace("@LINUX_USER@", LINUX_USER).replace('\r', "")
}

static STOP: AtomicBool = AtomicBool::new(false);
static REPORT: Mutex<Option<Report>> = Mutex::new(None);
static mut STATUS_HANDLE: SERVICE_STATUS_HANDLE = null_mut();
const CLIENT_PIPE_ACCESS: u32 = FILE_READ_DATA | FILE_WRITE_DATA | FILE_READ_ATTRIBUTES | FILE_WRITE_ATTRIBUTES | SYNCHRONIZE;
struct Secret(Vec<u16>);
impl Drop for Secret { fn drop(&mut self) { for ch in &mut self.0 { unsafe { std::ptr::write_volatile(ch, 0); } } std::sync::atomic::compiler_fence(Ordering::SeqCst); } }
fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() }
fn err(context: &str) -> String { format!("{context}: {}", std::io::Error::last_os_error()) }
fn base() -> PathBuf { PathBuf::from(BASE) }
fn config() -> Result<Config, String> { serde_json::from_slice(&fs::read(base().join("config.json")).map_err(|_| "broker_unavailable: installation not found".to_string())?).map_err(|e| e.to_string()) }

struct Handle(HANDLE);
impl Drop for Handle { fn drop(&mut self) { unsafe { if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE { CloseHandle(self.0); } } } }

struct ServiceHandle(SC_HANDLE);
impl Drop for ServiceHandle { fn drop(&mut self) { unsafe { if !self.0.is_null() { CloseServiceHandle(self.0); } } } }

unsafe fn sid_string(sid: PSID) -> Result<String, String> {
    let mut text = null_mut();
    if ConvertSidToStringSidW(sid, &mut text) == 0 { return Err(err("SID")); }
    let mut len = 0; while *text.add(len) != 0 { len += 1; }
    let result = String::from_utf16_lossy(std::slice::from_raw_parts(text, len));
    LocalFree(text.cast()); Ok(result)
}

fn token_sid(token: HANDLE) -> Result<String, String> { unsafe {
    let mut n = 0;
    GetTokenInformation(token, TokenUser, null_mut(), 0, &mut n);
    if n == 0 { return Err(err("token size")); }
    let mut buf = vec![0usize; (n as usize).div_ceil(size_of::<usize>())];
    if GetTokenInformation(token, TokenUser, buf.as_mut_ptr().cast(), n, &mut n) == 0 { return Err(err("token identity")); }
    let user = &*(buf.as_ptr() as *const TOKEN_USER);
    sid_string(user.User.Sid)
} }
fn current_sid() -> Result<String, String> { unsafe {
    let mut token = null_mut();
    if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 { return Err(err("identity")); }
    let token = Handle(token); token_sid(token.0)
} }
fn elevated() -> bool { unsafe {
    let mut token = null_mut(); if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 { return false; }
    let token = Handle(token); let mut elevation: TOKEN_ELEVATION = zeroed(); let mut size = 0;
    GetTokenInformation(token.0, TokenElevation, (&mut elevation as *mut TOKEN_ELEVATION).cast(), size_of::<TOKEN_ELEVATION>() as u32, &mut size) != 0 && elevation.TokenIsElevated != 0
} }

pub fn query(op: Operation) -> Result<Report, String> { unsafe {
    let cfg = config()?;
    if current_sid()? != cfg.client_sid { return Err("access_denied: caller is not authorized".into()); }
    let path = wide(PIPE);
    if WaitNamedPipeW(path.as_ptr(), 1500) == 0 { return Err("broker_unavailable: pipe not available within 1500ms".into()); }
    let handle = Handle(CreateFileW(path.as_ptr(), CLIENT_PIPE_ACCESS, 0, null(), OPEN_EXISTING, SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION, null_mut()));
    if handle.0 == INVALID_HANDLE_VALUE { return Err(err("access_denied")); }
    let mut pid = 0;
    if GetNamedPipeServerProcessId(handle.0, &mut pid) == 0 { return Err(err("server identity")); }
    // Compare the pipe endpoint with the process registered by the service manager.
    let manager = ServiceHandle(OpenSCManagerW(null(), null(), SC_MANAGER_CONNECT));
    if manager.0.is_null() { return Err(err("service manager")); }
    let service = ServiceHandle(OpenServiceW(manager.0, wide(SERVICE).as_ptr(), SERVICE_QUERY_STATUS));
    if service.0.is_null() { return Err(err("service identity")); }
    let mut status: SERVICE_STATUS_PROCESS = zeroed(); let mut needed = 0;
    let ok = QueryServiceStatusEx(service.0, SC_STATUS_PROCESS_INFO, (&mut status as *mut SERVICE_STATUS_PROCESS).cast(), size_of::<SERVICE_STATUS_PROCESS>() as u32, &mut needed);
    if ok == 0 || status.dwProcessId != pid || status.dwCurrentState != SERVICE_RUNNING { return Err("untrusted_server: endpoint does not match running service".into()); }
    let mode = PIPE_READMODE_MESSAGE | PIPE_NOWAIT;
    if SetNamedPipeHandleState(handle.0, &mode, null(), null()) == 0 { return Err(err("pipe mode")); }
    let request = op.as_bytes();
    let mut written = 0;
    if WriteFile(handle.0, request.as_ptr(), request.len() as u32, &mut written, null_mut()) == 0 { return Err(err("request")); }
    let data = read_message(handle.0, Duration::from_secs(2))?;
    let mut report: Report = serde_json::from_slice(&data).map_err(|_| "protocol_mismatch: invalid response".to_string())?;
    report.validate_at(unix_now())?;
    Ok(report)
} }

unsafe fn read_message(pipe: HANDLE, timeout: Duration) -> Result<Vec<u8>, String> {
    let start = Instant::now(); let mut buf = vec![0u8; MAX_MESSAGE];
    loop {
        let mut read = 0;
        if ReadFile(pipe, buf.as_mut_ptr(), buf.len() as u32, &mut read, null_mut()) != 0 && read > 0 { buf.truncate(read as usize); return Ok(buf); }
        let code = GetLastError();
        if code != ERROR_NO_DATA && code != ERROR_PIPE_LISTENING && code != ERROR_SUCCESS { return Err(format!("pipe read failed: {code}")); }
        if start.elapsed() > timeout || STOP.load(Ordering::Relaxed) { return Err("pipe timeout".into()); }
        thread::sleep(Duration::from_millis(15));
    }
}

pub fn service_dispatch() -> Result<(), String> { unsafe {
    let mut name = wide(SERVICE);
    let table = [SERVICE_TABLE_ENTRYW { lpServiceName: name.as_mut_ptr(), lpServiceProc: Some(service_main) }, SERVICE_TABLE_ENTRYW { lpServiceName: null_mut(), lpServiceProc: None }];
    if StartServiceCtrlDispatcherW(table.as_ptr()) == 0 { return Err(err("service dispatcher")); } Ok(())
} }
unsafe extern "system" fn control(code: u32, _: u32, _: *mut c_void, _: *mut c_void) -> u32 { if code == SERVICE_CONTROL_STOP || code == SERVICE_CONTROL_SHUTDOWN { STOP.store(true, Ordering::Relaxed); } 0 }
unsafe fn set_status(state: u32, exit: u32) {
    let s = SERVICE_STATUS { dwServiceType: SERVICE_WIN32_OWN_PROCESS, dwCurrentState: state, dwControlsAccepted: if state == SERVICE_RUNNING { SERVICE_ACCEPT_STOP | SERVICE_ACCEPT_SHUTDOWN } else { 0 }, dwWin32ExitCode: exit, dwServiceSpecificExitCode: 0, dwCheckPoint: 0, dwWaitHint: 0 };
    SetServiceStatus(STATUS_HANDLE, &s);
}
unsafe extern "system" fn service_main(_: u32, _: *mut *mut u16) {
    STATUS_HANDLE = RegisterServiceCtrlHandlerExW(wide(SERVICE).as_ptr(), Some(control), null());
    if STATUS_HANDLE.is_null() { return; }
    set_status(SERVICE_START_PENDING, 0);
    let result = config().and_then(|cfg| {
        if current_sid()? != cfg.account_sid || elevated() { return Err("service identity must be dedicated and non-admin".into()); }
        set_status(SERVICE_RUNNING, 0);
        thread::spawn(supervise);
        serve(&cfg)
    });
    STOP.store(true, Ordering::Relaxed);
    set_status(SERVICE_STOPPED, if result.is_ok() { 0 } else { 1 });
}

fn serve(cfg: &Config) -> Result<(), String> { unsafe {
    if !valid_sid(&cfg.client_sid) || !valid_sid(&cfg.account_sid) || cfg.client_sid == cfg.account_sid { return Err("invalid or overlapping identities".into()); }
    let acl = wide(&format!("D:P(A;;GA;;;SY)(A;;GA;;;{})(A;;0x{CLIENT_PIPE_ACCESS:08x};;;{})", cfg.account_sid, cfg.client_sid));
    let mut sd = null_mut();
    if ConvertStringSecurityDescriptorToSecurityDescriptorW(acl.as_ptr(), 1, &mut sd, null_mut()) == 0 { return Err(err("pipe ACL")); }
    let sa = SECURITY_ATTRIBUTES { nLength: size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor: sd, bInheritHandle: 0 };
    let pipe = Handle(CreateNamedPipeW(wide(PIPE).as_ptr(), PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS, 1, MAX_MESSAGE as u32, MAX_MESSAGE as u32, 1000, &sa));
    LocalFree(sd);
    if pipe.0 == INVALID_HANDLE_VALUE { return Err(err("pipe creation")); }
    while !STOP.load(Ordering::Relaxed) {
        if ConnectNamedPipe(pipe.0, null_mut()) == 0 && GetLastError() != ERROR_PIPE_CONNECTED { thread::sleep(Duration::from_millis(30)); continue; }
        let response = (|| -> Result<Vec<u8>, String> {
            let request = read_message(pipe.0, Duration::from_secs(1))?;
            if ImpersonateNamedPipeClient(pipe.0) == 0 { return Err(err("client authentication")); }
            let mut token = null_mut();
            let identity = if OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, 1, &mut token) != 0 { let t = Handle(token); token_sid(t.0) } else { Err(err("client token")) };
            if RevertToSelf() == 0 { std::process::abort(); }
            if identity? != cfg.client_sid { return Err("access_denied".into()); }
            if Operation::from_bytes(&request).is_none() { return Err("unsupported operation".into()); }
            let mut report = REPORT.lock().map_err(|_| "state lock".to_string())?.clone().unwrap_or_else(|| Report::new("unknown", "supervisor initializing", false));
            if matches!(report.state.as_str(), "downloading" | "configuring" | "verifying") { report.observed_unix = unix_now(); }
            serde_json::to_vec(&report).map_err(|e| e.to_string())
        })();
        if let Ok(data) = response { let mut n = 0; WriteFile(pipe.0, data.as_ptr(), data.len() as u32, &mut n, null_mut()); thread::sleep(Duration::from_millis(50)); }
        DisconnectNamedPipe(pipe.0);
    }
    Ok(())
} }

fn run(exe: &Path, args: &[&str], seconds: u64) -> Result<(), String> {
    let log_path = base().join("private").join("runtime.log");
    let output = || -> Stdio { fs::OpenOptions::new().create(true).append(true).open(&log_path).map(Stdio::from).unwrap_or_else(|_| Stdio::null()) };
    let mut child = Command::new(exe).creation_flags(CREATE_NO_WINDOW).args(args).stdin(Stdio::null()).stdout(output()).stderr(output()).spawn().map_err(|e| e.to_string())?;
    let started = Instant::now();
    loop {
        match child.try_wait().map_err(|e| e.to_string())? { Some(s) if s.success() => return Ok(()), Some(s) => return Err(format!("{} exited with {s}", exe.display())), None => {} }
        if started.elapsed() > Duration::from_secs(seconds) || STOP.load(Ordering::Relaxed) { let _ = child.kill(); let _ = child.wait(); return Err(format!("{} timed out", exe.display())); }
        thread::sleep(Duration::from_millis(100));
    }
}
fn wsl() -> PathBuf { PathBuf::from(r"C:\Windows\System32\wsl.exe") }
fn publish_report(report: Report) {
    if let Ok(bytes) = serde_json::to_vec(&report) {
        let _ = installer::atomic_write(&base().join("private/runtime-status.json"), &bytes);
    }
    *REPORT.lock().unwrap() = Some(report);
}

fn supervise() {
    let result = initialize_runtime();
    if let Err(e) = result { publish_report(Report::new("degraded", &e, false)); return; }
    while !STOP.load(Ordering::Relaxed) {
        // Supervisor, not a user query, is responsible for keeping the runtime active.
        let probe = format!("set -o pipefail; test \"$(cat /proc/1/comm)\" = systemd && test -f /sys/fs/cgroup/cgroup.controllers && test -x /usr/lib/systemd/system-generators/podman-system-generator && runuser -u {LINUX_USER} -- env XDG_RUNTIME_DIR=/run/user/$(id -u {LINUX_USER}) podman info --format '{{{{.Host.CgroupsVersion}}}}' | grep -qx v2");
        let result = run(&wsl(), &["-d", DISTRO, "-u", "root", "--", "bash", "-lc", &probe], 20);
        publish_report(match result { Ok(()) => Report::new("ready", "systemd, cgroups v2 and rootless Podman verified; no application Quadlet deployed", true), Err(e) => Report::new("degraded", &e, false) });
        for _ in 0..100 { if STOP.load(Ordering::Relaxed) { break; } thread::sleep(Duration::from_millis(100)); }
    }
}
fn initialize_runtime() -> Result<(), String> {
    let data = base().join("private");
    if !data.join("initialized").exists() {
        let cfg = config()?;
        let recovering = data.join("download-started").exists();
        if data.join("distro").exists() && !recovering { return Err("Unowned partial distro directory; administrator inspection required.".into()); }
        if recovering {
            // Never delete/reimport a distro on retry. Only continue a usable owned Ubuntu.
            run(&wsl(), &["-d", DISTRO, "-u", "root", "--", "bash", "-lc", ". /etc/os-release; test \"$ID\" = ubuntu && test \"$VERSION_ID\" = 24.04"], 60)
                .map_err(|e| format!("Partial download cannot be resumed safely; inspect runtime.log: {e}"))?;
        } else if cfg.rootfs_sha256.is_empty() {
            publish_report(Report::new("downloading", "WSL is downloading Ubuntu 24.04 under the dedicated account.", false));
            // A durable marker prevents retrying a possibly partial registration blindly.
            let marker = data.join("download-started");
            fs::write(&marker, b"1").map_err(|e| e.to_string())?;
            run(&wsl(), &["--install", "Ubuntu-24.04", "--web-download", "--no-launch", "--name", DISTRO, "--location", data.join("distro").to_str().unwrap()], 1800)?;
        } else {
            verify_sha256(&mut fs::File::open(data.join("rootfs.tar")).map_err(|e| e.to_string())?, &cfg.rootfs_sha256)?;
            run(&wsl(), &["--import", DISTRO, data.join("distro").to_str().unwrap(), data.join("rootfs.tar").to_str().unwrap(), "--version", "2"], 300)?;
        }
        publish_report(Report::new("configuring", "Configuring Ubuntu, systemd and rootless Podman.", false));
        run(&wsl(), &["-d", DISTRO, "-u", "root", "--", "bash", "-lc", &bootstrap()], 1200)?;
        run(&wsl(), &["--terminate", DISTRO], 30)?;
        fs::write(data.join("initialized"), b"1").map_err(|e| e.to_string())?;
    }
    publish_report(Report::new("verifying", "Verifying the dedicated runtime.", false));
    Ok(())
}

fn account_sid(name: &str) -> Result<String, String> { unsafe {
    let mut n = 0; let mut dn = 0; let mut kind = 0;
    LookupAccountNameW(null(), wide(name).as_ptr(), null_mut(), &mut n, null_mut(), &mut dn, &mut kind);
    if n == 0 { return Err(err("lookup account")); }
    let mut sid = vec![0u8; n as usize]; let mut domain = vec![0u16; dn as usize];
    if LookupAccountNameW(null(), wide(name).as_ptr(), sid.as_mut_ptr().cast(), &mut n, domain.as_mut_ptr(), &mut dn, &mut kind) == 0 { return Err(err("lookup SID")); }
    sid_string(sid.as_mut_ptr().cast())
} }
unsafe fn account_rights(sid: &str) -> Result<(), String> {
    let mut raw = null_mut(); if ConvertStringSidToSidW(wide(sid).as_ptr(), &mut raw) == 0 { return Err(err("account rights SID")); }
    let mut attrs: LSA_OBJECT_ATTRIBUTES = zeroed(); attrs.Length = size_of::<LSA_OBJECT_ATTRIBUTES>() as u32;
    let mut policy = 0isize; let result = LsaOpenPolicy(null(), &attrs, (POLICY_LOOKUP_NAMES | POLICY_CREATE_ACCOUNT) as u32, &mut policy);
    if result != 0 { LocalFree(raw); return Err(format!("LSA open: {}", LsaNtStatusToWinError(result))); }
    for right in ["SeServiceLogonRight", "SeDenyInteractiveLogonRight", "SeDenyRemoteInteractiveLogonRight"] {
        let mut name = wide(right); let value = LSA_UNICODE_STRING { Length: ((name.len()-1)*2) as u16, MaximumLength: (name.len()*2) as u16, Buffer: name.as_mut_ptr() };
        let result = LsaAddAccountRights(policy, raw, &value, 1);
        if result != 0 { LsaClose(policy); LocalFree(raw); return Err(format!("account right: {}", LsaNtStatusToWinError(result))); }
    }
    LsaClose(policy); LocalFree(raw); Ok(())
}

fn verify_sha256(input: &mut impl Read, expected: &str) -> Result<(), String> {
    let mut hash = Sha256::new(); let mut buffer = [0u8; 65536];
    loop { let n = input.read(&mut buffer).map_err(|e| e.to_string())?; if n == 0 { break; } hash.update(&buffer[..n]); }
    if format!("{:x}", hash.finalize()) != expected.to_ascii_lowercase() { return Err("Rootfs SHA256 mismatch; refusing to import.".into()); }
    Ok(())
}

pub fn setup(args: &[String]) -> Result<(), String> {
    if args == ["preflight"] {
        println!("{}", serde_json::json!({"product":PRODUCT, "service":SERVICE, "account":ACCOUNT, "distribution":DISTRO, "install_path":BASE, "platform":"windows", "architecture":std::env::consts::ARCH, "elevated":elevated(), "current_sid":current_sid()?, "wsl_launcher_present":wsl().exists(), "installation_present":base().exists(), "dedicated_runtime_verified":false, "changes_made":false})); return Ok(());
    }
    // Keep the verified offline path available, but default to WSL-managed download.
    let normalized;
    let args = if args.len() == 2 && args[0] == "install" {
        normalized = vec![args[0].clone(), String::new(), String::new(), args[1].clone()];
        &normalized[..]
    } else { args };
    if args.len() != 4 || args[0] != "install" { return Err(format!("Usage: {SETUP_EXE} install <client-SID> OR install <rootfs.tar> <sha256> <client-SID>")); }
    let online = args[1].is_empty() && args[2].is_empty();
    if !elevated() { return Err("Open an elevated console to install; check/status never require elevation.".into()); }
    if !valid_sid(&args[3]) { return Err("Expected a local/domain user SID, not a group or system identity.".into()); }
    let _engine_lock = installer::lock_engine(&args[3])?;
    if !online && (args[2].len() != 64 || !args[2].bytes().all(|b| b.is_ascii_hexdigit())) { return Err("SHA256 must contain exactly 64 hexadecimal characters.".into()); }
    if base().exists() || account_sid(ACCOUNT).is_ok() { return Err("Existing installation or account detected; refusing to overwrite. Administrator inspection required.".into()); }
    let mut input = if online { None } else {
        let mut file = fs::File::open(fs::canonicalize(&args[1]).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        verify_sha256(&mut file, &args[2])?;
        Some(file)
    };
    let companion = std::env::current_exe().map_err(|e| e.to_string())?.with_file_name(CLI_EXE);
    if !companion.is_file() { return Err(format!("Place {CLI_EXE} next to {SETUP_EXE}.")); }
    // Enable host features without rebooting the user's machine automatically.
    let features = Command::new(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe")
        .creation_flags(CREATE_NO_WINDOW).env("PSModulePath", r"C:\Windows\System32\WindowsPowerShell\v1.0\Modules").args(["-NoProfile", "-NonInteractive", "-Command", r"$ErrorActionPreference='Stop'; $restart=(Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending'); foreach($name in @('Microsoft-Windows-Subsystem-Linux','VirtualMachinePlatform')) { $f=Get-WindowsOptionalFeature -Online -FeatureName $name; if($f.State -eq 'EnablePending') { $restart=$true } elseif($f.State -ne 'Enabled') { $r=Enable-WindowsOptionalFeature -Online -FeatureName $name -All -NoRestart; if($r.RestartNeeded) { $restart=$true } } }; if($restart) { exit 3010 }"])
        .status().map_err(|e| e.to_string())?;
    if features.code() == Some(3010) { println!("Restart Windows and run the same install command again. No dedicated account created yet."); std::process::exit(3010); }
    if !features.success() { return Err(format!("Windows feature preparation failed: {features}")); }
    // Host prerequisites happen before creating a dedicated identity.
    let mut child = Command::new(&wsl()).creation_flags(CREATE_NO_WINDOW).args(["--install", "--no-distribution", "--web-download", "--no-launch"]).spawn().map_err(|e| e.to_string())?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? { break status; }
        if start.elapsed() > Duration::from_secs(600) {
            let _ = child.kill(); let _ = child.wait();
            return Err("WSL host preparation timed out. Inspect Windows prerequisites before retrying.".into());
        }
        thread::sleep(Duration::from_millis(200));
    };
    if status.code() == Some(3010) { println!("Restart Windows and run the same install command again."); std::process::exit(3010); }
    if !status.success() { return Err(format!("WSL preparation failed ({status}); restart if requested and retry.")); }
    if online { installer::provisioning_progress(&args[3])?; }
    fs::create_dir(base()).map_err(|e| e.to_string())?;
    // Protect the newly created directory before putting executable content in it.
    run(Path::new(r"C:\Windows\System32\icacls.exe"), &[BASE, "/inheritance:r", "/grant:r", "*S-1-5-18:(OI)(CI)F", "*S-1-5-32-544:(OI)(CI)F", &format!("*{}:(OI)(CI)RX", args[3])], 15)?;
    unsafe {
        let mut random = [0u8; 32]; if BCryptGenRandom(null_mut(), random.as_mut_ptr(), 32, BCRYPT_USE_SYSTEM_PREFERRED_RNG) != 0 { return Err("secure random generation failed".into()); }
        let mut password_w = Secret(Vec::with_capacity(69)); password_w.0.extend("Aa1!".encode_utf16());
        const HEX: &[u8] = b"0123456789abcdef";
        for byte in &random { password_w.0.push(HEX[(byte >> 4) as usize] as u16); password_w.0.push(HEX[(byte & 15) as usize] as u16); }
        password_w.0.push(0);
        for byte in &mut random { std::ptr::write_volatile(byte, 0); }
        let mut user = wide(ACCOUNT);
        let info = USER_INFO_1 { usri1_name: user.as_mut_ptr(), usri1_password: password_w.0.as_mut_ptr(), usri1_password_age: 0, usri1_priv: USER_PRIV_USER, usri1_home_dir: null_mut(), usri1_comment: null_mut(), usri1_flags: UF_SCRIPT | UF_DONT_EXPIRE_PASSWD | UF_PASSWD_CANT_CHANGE, usri1_script_path: null_mut() };
        let mut parm = 0; let code = NetUserAdd(null(), 1, (&info as *const USER_INFO_1).cast(), &mut parm);
        if code != 0 { return Err(format!("Dedicated account creation failed: {code}, parameter {parm}")); }
        let sid = account_sid(ACCOUNT)?; account_rights(&sid)?;
        run(Path::new(r"C:\Windows\System32\icacls.exe"), &[BASE, "/grant", &format!("*{sid}:(OI)(CI)RX")], 15)?;
        let private = base().join("private"); fs::create_dir(&private).map_err(|e| e.to_string())?;
        run(Path::new(r"C:\Windows\System32\icacls.exe"), &[private.to_str().unwrap(), "/inheritance:r", "/grant:r", "*S-1-5-18:(OI)(CI)F", "*S-1-5-32-544:(OI)(CI)F", &format!("*{sid}:(OI)(CI)F")], 15)?;
        if let Some(input) = input.as_mut() {
        input.rewind().map_err(|e| e.to_string())?;
        let mut staged = fs::OpenOptions::new().read(true).write(true).create_new(true).open(private.join("rootfs.tar")).map_err(|e| e.to_string())?;
        std::io::copy(input, &mut staged).map_err(|e| e.to_string())?;
        staged.sync_all().map_err(|e| e.to_string())?;
        staged.rewind().map_err(|e| e.to_string())?;
        verify_sha256(&mut staged, &args[2])?;
        }
        fs::copy(companion, base().join(CLI_EXE)).map_err(|e| e.to_string())?;
        let cfg = Config { client_sid: args[3].clone(), account_sid: sid, rootfs_sha256: args[2].to_ascii_lowercase() };
        fs::write(base().join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).map_err(|e| e.to_string())?;
        let scm = ServiceHandle(OpenSCManagerW(null(), null(), SC_MANAGER_CREATE_SERVICE));
        if scm.0.is_null() { return Err(err("service manager")); }
        let binary = wide(&format!("\"{}\" --service", base().join(CLI_EXE).display()));
        let service = ServiceHandle(CreateServiceW(scm.0, wide(SERVICE).as_ptr(), wide(SERVICE_DISPLAY).as_ptr(), SERVICE_START | SERVICE_QUERY_STATUS, SERVICE_WIN32_OWN_PROCESS, SERVICE_AUTO_START, SERVICE_ERROR_NORMAL, binary.as_ptr(), null(), null_mut(), null(), wide(&format!(".\\{ACCOUNT}")).as_ptr(), password_w.0.as_ptr()));
        drop(password_w);
        if service.0.is_null() { return Err(err("service creation (account retained for recovery)")); }
        let started = StartServiceW(service.0, 0, null());
        if started == 0 { return Err(err("service start (installation retained for recovery)")); }
    }
    println!("{PRODUCT} service installed. Initialization is asynchronous; use {CLI_EXE} check. Installation does not yet mean runtime readiness."); Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn installation_identity_matches_bootstrap() {
        assert!(ACCOUNT.len() <= 20);
        assert!(ACCOUNT.bytes().all(|c| c.is_ascii_lowercase() || c == b'_'));
        let script = bootstrap();
        assert!(!script.contains("@LINUX_USER@") && !script.contains('\r'));
        for required in [format!("default={LINUX_USER}"), format!("/home/{LINUX_USER}/.config/containers/systemd"), format!("/var/lib/systemd/linger/{LINUX_USER}")] {
            assert!(script.contains(&required));
        }
    }
    #[test] fn sid_cannot_inject_acl() {
        assert!(valid_sid("S-1-5-21-1-2-3-1001"));
        for sid in ["S-1-5-18", "S-1-5-21-1-2-3-1001)(A;;GA;;;WD)", "S-1-5-21-1-2-3-", "S-1-5-21-1-2-3-1001-extra", "S-1-5-21-4294967296-2-3-1001"] {
            assert!(!valid_sid(sid));
        }
    }
    #[test] fn pipe_rights_allow_mode_changes_without_server_creation() {
        assert_eq!(CLIENT_PIPE_ACCESS & FILE_WRITE_ATTRIBUTES, FILE_WRITE_ATTRIBUTES);
        assert_eq!(CLIENT_PIPE_ACCESS & FILE_CREATE_PIPE_INSTANCE, 0);
        assert_eq!(CLIENT_PIPE_ACCESS, 0x00100183);
    }
    #[test] fn rootfs_hash_detects_changes() {
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(verify_sha256(&mut &b"abc"[..], expected).is_ok());
        assert!(verify_sha256(&mut &b"abd"[..], expected).is_err());
        assert!(verify_sha256(&mut &b""[..], expected).is_err());
    }
}
