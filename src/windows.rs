use crate::protocol::*;
use sha2::{Digest, Sha256};
use std::{ffi::c_void, fs, io::Read, mem::{size_of, zeroed}, path::{Path, PathBuf}, process::{Command, Stdio}, ptr::{null, null_mut}, sync::{atomic::{AtomicBool, Ordering}, Mutex}, thread, time::{Duration, Instant}};
use windows_sys::Win32::{Foundation::*, Security::{*, Authorization::*, Authentication::Identity::*, Cryptography::*}, NetworkManagement::NetManagement::*, Storage::FileSystem::*, System::{Pipes::*, Services::*, Threading::*}};

const BASE: &str = r"C:\ProgramData\IsolatedRuntime";
static STOP: AtomicBool = AtomicBool::new(false);
static REPORT: Mutex<Option<Report>> = Mutex::new(None);
static mut STATUS_HANDLE: SERVICE_STATUS_HANDLE = null_mut();
fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() }
fn err(context: &str) -> String { format!("{context}: {}", std::io::Error::last_os_error()) }
fn base() -> PathBuf { PathBuf::from(BASE) }
fn config() -> Result<Config, String> { serde_json::from_slice(&fs::read(base().join("config.json")).map_err(|_| "broker_unavailable: installation not found".to_string())?).map_err(|e| e.to_string()) }

struct Handle(HANDLE);
impl Drop for Handle { fn drop(&mut self) { unsafe { if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE { CloseHandle(self.0); } } } }

fn token_sid(token: HANDLE) -> Result<String, String> { unsafe {
    let mut n = 0;
    GetTokenInformation(token, TokenUser, null_mut(), 0, &mut n);
    if n == 0 { return Err(err("token size")); }
    let mut buf = vec![0usize; (n as usize).div_ceil(size_of::<usize>())];
    if GetTokenInformation(token, TokenUser, buf.as_mut_ptr().cast(), n, &mut n) == 0 { return Err(err("token identity")); }
    let user = &*(buf.as_ptr() as *const TOKEN_USER);
    let mut text = null_mut();
    if ConvertSidToStringSidW(user.User.Sid, &mut text) == 0 { return Err(err("SID")); }
    let mut len = 0; while *text.add(len) != 0 { len += 1; }
    let sid = String::from_utf16_lossy(std::slice::from_raw_parts(text, len));
    LocalFree(text.cast()); Ok(sid)
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
    let handle = Handle(CreateFileW(path.as_ptr(), FILE_READ_DATA | FILE_WRITE_DATA | FILE_READ_ATTRIBUTES | FILE_WRITE_ATTRIBUTES | SYNCHRONIZE, 0, null(), OPEN_EXISTING, SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION, null_mut()));
    if handle.0 == INVALID_HANDLE_VALUE { return Err(err("access_denied")); }
    let mut pid = 0;
    if GetNamedPipeServerProcessId(handle.0, &mut pid) == 0 { return Err(err("server identity")); }
    // Compare the pipe endpoint with the process registered by the service manager.
    let manager = OpenSCManagerW(null(), null(), SC_MANAGER_CONNECT);
    if manager.is_null() { return Err(err("service manager")); }
    let service = OpenServiceW(manager, wide(SERVICE).as_ptr(), SERVICE_QUERY_STATUS);
    if service.is_null() { CloseServiceHandle(manager); return Err(err("service identity")); }
    let mut status: SERVICE_STATUS_PROCESS = zeroed(); let mut needed = 0;
    let ok = QueryServiceStatusEx(service, SC_STATUS_PROCESS_INFO, (&mut status as *mut SERVICE_STATUS_PROCESS).cast(), size_of::<SERVICE_STATUS_PROCESS>() as u32, &mut needed);
    CloseServiceHandle(service); CloseServiceHandle(manager);
    if ok == 0 || status.dwProcessId != pid || status.dwCurrentState != SERVICE_RUNNING { return Err("untrusted_server: endpoint does not match running service".into()); }
    let mode = PIPE_READMODE_MESSAGE | PIPE_NOWAIT;
    if SetNamedPipeHandleState(handle.0, &mode, null(), null()) == 0 { return Err(err("pipe mode")); }
    let request = match op { Operation::Check => b"check".as_slice(), Operation::Status => b"status".as_slice() };
    let mut written = 0;
    if WriteFile(handle.0, request.as_ptr(), request.len() as u32, &mut written, null_mut()) == 0 { return Err(err("request")); }
    let data = read_message(handle.0, Duration::from_secs(2))?;
    let report: Report = serde_json::from_slice(&data).map_err(|_| "protocol_mismatch: invalid response".to_string())?;
    if report.protocol != 1 { return Err("protocol_mismatch".into()); }
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
    if !valid_sid(&cfg.client_sid) { return Err("invalid client SID".into()); }
    let acl = wide(&format!("D:P(A;;GA;;;SY)(A;;GA;;;{})(A;;0x00100083;;;{})", cfg.account_sid, cfg.client_sid));
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
            if request != b"check" && request != b"status" { return Err("unsupported operation".into()); }
            let report = REPORT.lock().map_err(|_| "state lock".to_string())?.clone().unwrap_or_else(|| Report::new("unknown", "supervisor initializing", false));
            serde_json::to_vec(&report).map_err(|e| e.to_string())
        })();
        if let Ok(data) = response { let mut n = 0; WriteFile(pipe.0, data.as_ptr(), data.len() as u32, &mut n, null_mut()); thread::sleep(Duration::from_millis(50)); }
        DisconnectNamedPipe(pipe.0);
    }
    Ok(())
} }

fn run(exe: &Path, args: &[&str], seconds: u64) -> Result<(), String> {
    let mut child = Command::new(exe).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())?;
    let started = Instant::now();
    loop {
        match child.try_wait().map_err(|e| e.to_string())? { Some(s) if s.success() => return Ok(()), Some(s) => return Err(format!("{} exited with {s}", exe.display())), None => {} }
        if started.elapsed() > Duration::from_secs(seconds) || STOP.load(Ordering::Relaxed) { let _ = child.kill(); let _ = child.wait(); return Err(format!("{} timed out", exe.display())); }
        thread::sleep(Duration::from_millis(100));
    }
}
fn wsl() -> PathBuf { PathBuf::from(r"C:\Windows\System32\wsl.exe") }
fn supervise() {
    let result = initialize_runtime();
    if let Err(e) = result { *REPORT.lock().unwrap() = Some(Report::new("degraded", &e, false)); return; }
    while !STOP.load(Ordering::Relaxed) {
        // Supervisor, not a user query, is responsible for keeping the runtime active.
        let result = run(&wsl(), &["-d", DISTRO, "-u", "root", "--", "bash", "-lc", "test \"$(cat /proc/1/comm)\" = systemd && test -f /sys/fs/cgroup/cgroup.controllers && test -x /usr/lib/systemd/system-generators/podman-system-generator && runuser -u runtime -- env XDG_RUNTIME_DIR=/run/user/$(id -u runtime) podman info --format '{{.Host.CgroupsVersion}}' | grep -qx v2"], 20);
        *REPORT.lock().unwrap() = Some(match result { Ok(()) => Report::new("ready", "systemd, cgroups v2 and rootless Podman verified; no application Quadlet deployed", true), Err(e) => Report::new("degraded", &e, false) });
        for _ in 0..100 { if STOP.load(Ordering::Relaxed) { break; } thread::sleep(Duration::from_millis(100)); }
    }
}
fn initialize_runtime() -> Result<(), String> {
    let data = base().join("private");
    if !data.join("initialized").exists() {
        if data.join("distro").exists() { return Err("partial initialization detected; administrator recovery required (existing distro preserved)".into()); }
        run(&wsl(), &["--import", DISTRO, data.join("distro").to_str().unwrap(), data.join("rootfs.tar").to_str().unwrap(), "--version", "2"], 300)?;
        run(&wsl(), &["-d", DISTRO, "-u", "root", "--", "bash", "-lc", include_str!("bootstrap.sh")], 600)?;
        run(&wsl(), &["--terminate", DISTRO], 30)?;
        fs::write(data.join("initialized"), b"1").map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn account_sid(name: &str) -> Result<String, String> { unsafe {
    let mut n = 0; let mut dn = 0; let mut kind = 0;
    LookupAccountNameW(null(), wide(name).as_ptr(), null_mut(), &mut n, null_mut(), &mut dn, &mut kind);
    if n == 0 { return Err(err("lookup account")); }
    let mut sid = vec![0u8; n as usize]; let mut domain = vec![0u16; dn as usize];
    if LookupAccountNameW(null(), wide(name).as_ptr(), sid.as_mut_ptr().cast(), &mut n, domain.as_mut_ptr(), &mut dn, &mut kind) == 0 { return Err(err("lookup SID")); }
    let mut text = null_mut(); if ConvertSidToStringSidW(sid.as_mut_ptr().cast(), &mut text) == 0 { return Err(err("account SID")); }
    let mut len = 0; while *text.add(len) != 0 { len += 1; }
    let result = String::from_utf16_lossy(std::slice::from_raw_parts(text, len)); LocalFree(text.cast()); Ok(result)
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

pub fn setup(args: &[String]) -> Result<(), String> {
    if args == ["preflight"] {
        println!("{}", serde_json::json!({"platform":"windows", "architecture":std::env::consts::ARCH, "elevated":elevated(), "current_sid":current_sid()?, "wsl_launcher_present":wsl().exists(), "installation_present":base().exists(), "dedicated_runtime_verified":false, "changes_made":false})); return Ok(());
    }
    if args.len() != 4 || args[0] != "install" { return Err("Usage: runtime-setup install <rootfs.tar> <sha256> <client-SID>".into()); }
    if !elevated() { return Err("Open an elevated console to install; check/status never require elevation.".into()); }
    if !valid_sid(&args[3]) { return Err("Expected a local/domain user SID, not a group or system identity.".into()); }
    if args[2].len() != 64 || !args[2].bytes().all(|b| b.is_ascii_hexdigit()) { return Err("SHA256 must contain exactly 64 hexadecimal characters.".into()); }
    if base().exists() || account_sid(ACCOUNT).is_ok() { return Err("Existing installation or account detected; refusing to overwrite. Administrator inspection required.".into()); }
    let source = fs::canonicalize(&args[1]).map_err(|e| e.to_string())?;
    let mut input = fs::File::open(&source).map_err(|e| e.to_string())?; let mut hash = Sha256::new(); let mut buffer = [0u8; 65536];
    loop { let n = input.read(&mut buffer).map_err(|e| e.to_string())?; if n == 0 { break; } hash.update(&buffer[..n]); }
    if format!("{:x}", hash.finalize()) != args[2].to_ascii_lowercase() { return Err("Rootfs SHA256 mismatch; no installation changes made.".into()); }
    let companion = std::env::current_exe().map_err(|e| e.to_string())?.with_file_name("runtime.exe");
    if !companion.is_file() { return Err("Place runtime.exe next to runtime-setup.exe.".into()); }
    // Host prerequisites happen before creating a dedicated identity.
    let status = Command::new(&wsl()).args(["--install", "--no-distribution", "--web-download", "--no-launch"]).status().map_err(|e| e.to_string())?;
    if status.code() == Some(3010) { println!("Restart Windows and run the same install command again."); std::process::exit(3010); }
    if !status.success() { return Err(format!("WSL preparation failed ({status}); restart if requested and retry.")); }
    fs::create_dir(base()).map_err(|e| e.to_string())?;
    // Protect the newly created directory before putting executable content in it.
    run(Path::new(r"C:\Windows\System32\icacls.exe"), &[BASE, "/inheritance:r", "/grant:r", "*S-1-5-18:(OI)(CI)F", "*S-1-5-32-544:(OI)(CI)F", &format!("*{}:(OI)(CI)RX", args[3])], 15)?;
    unsafe {
        let mut random = [0u8; 32]; if BCryptGenRandom(null_mut(), random.as_mut_ptr(), 32, BCRYPT_USE_SYSTEM_PREFERRED_RNG) != 0 { return Err("secure random generation failed".into()); }
        let password = format!("Aa1!{}", random.iter().map(|b| format!("{b:02x}")).collect::<String>());
        let mut password_w = wide(&password); let mut user = wide(ACCOUNT);
        let info = USER_INFO_1 { usri1_name: user.as_mut_ptr(), usri1_password: password_w.as_mut_ptr(), usri1_password_age: 0, usri1_priv: USER_PRIV_USER, usri1_home_dir: null_mut(), usri1_comment: null_mut(), usri1_flags: UF_SCRIPT | UF_DONT_EXPIRE_PASSWD | UF_PASSWD_CANT_CHANGE, usri1_script_path: null_mut() };
        let mut parm = 0; let code = NetUserAdd(null(), 1, (&info as *const USER_INFO_1).cast(), &mut parm);
        if code != 0 { return Err(format!("Dedicated account creation failed: {code}, parameter {parm}")); }
        let sid = account_sid(ACCOUNT)?; account_rights(&sid)?;
        run(Path::new(r"C:\Windows\System32\icacls.exe"), &[BASE, "/grant", &format!("*{sid}:(OI)(CI)RX")], 15)?;
        let private = base().join("private"); fs::create_dir(&private).map_err(|e| e.to_string())?;
        run(Path::new(r"C:\Windows\System32\icacls.exe"), &[private.to_str().unwrap(), "/inheritance:r", "/grant:r", "*S-1-5-18:(OI)(CI)F", "*S-1-5-32-544:(OI)(CI)F", &format!("*{sid}:(OI)(CI)F")], 15)?;
        fs::copy(&source, private.join("rootfs.tar")).map_err(|e| e.to_string())?;
        fs::copy(companion, base().join("runtime.exe")).map_err(|e| e.to_string())?;
        let cfg = Config { client_sid: args[3].clone(), account_sid: sid, rootfs_sha256: args[2].to_ascii_lowercase() };
        fs::write(base().join("config.json"), serde_json::to_vec_pretty(&cfg).unwrap()).map_err(|e| e.to_string())?;
        let scm = OpenSCManagerW(null(), null(), SC_MANAGER_CREATE_SERVICE);
        if scm.is_null() { return Err(err("service manager")); }
        let binary = wide(&format!("\"{}\" --service", base().join("runtime.exe").display()));
        let service = CreateServiceW(scm, wide(SERVICE).as_ptr(), wide("Isolated runtime query service").as_ptr(), SERVICE_START | SERVICE_QUERY_STATUS, SERVICE_WIN32_OWN_PROCESS, SERVICE_AUTO_START, SERVICE_ERROR_NORMAL, binary.as_ptr(), null(), null_mut(), null(), wide(&format!(".\\{ACCOUNT}")).as_ptr(), password_w.as_ptr());
        for ch in &mut password_w { std::ptr::write_volatile(ch, 0); }
        if service.is_null() { CloseServiceHandle(scm); return Err(err("service creation (account retained for recovery)")); }
        let started = StartServiceW(service, 0, null()); CloseServiceHandle(service); CloseServiceHandle(scm);
        if started == 0 { return Err(err("service start (installation retained for recovery)")); }
    }
    println!("Service installed. Runtime initialization is asynchronous; use runtime check. Installation does not yet mean runtime readiness."); Ok(())
}
