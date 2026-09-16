//! Privileged resumable setup worker. The GUI only reads state and requests UAC.
use super::*;
use crate::installer::{Stage, State};
use std::io::Write;
use windows_sys::Win32::UI::{Shell::{ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_NOCLOSEPROCESS, SEE_MASK_NOASYNC}, WindowsAndMessaging::SW_HIDE};

const ROOT: &str = r"C:\ProgramData\QuetzalcoatlGNX-Setup";
const TASK: &str = "QuetzalcoatlGNX-Resume";
const UI_TASK: &str = "QuetzalcoatlGNX-SetupUI";
const POWERSHELL: &str = r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe";
fn root() -> PathBuf { PathBuf::from(ROOT) }
pub fn diagnostic_path() -> PathBuf { root().join("setup.log") }
pub fn caller_sid() -> Result<String, String> { current_sid() }

pub(super) fn lock_engine(client_sid: &str) -> Result<Option<Handle>, String> {
    if !root().exists() { return Ok(None); }
    secure_root(Some(client_sid))?;
    lock_at(&root().join("engine.lock")).map(Some)
}

pub(super) fn provisioning_progress(client_sid: &str) -> Result<(), String> {
    if !root().exists() { return Ok(()); }
    secure_root(Some(client_sid))?;
    if let Some(mut state) = load_state()? {
        if state.client_sid == client_sid && state.provisioning_started {
            transition(&mut state, Stage::Provisioning, "WSL preparado. Creando la cuenta no administradora y el servicio dedicado.")?;
        }
    }
    Ok(())
}

pub(super) fn atomic_write(path: &Path, data: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension("new");
    let mut file = fs::OpenOptions::new().write(true).create(true).truncate(true).open(&temporary).map_err(|e| e.to_string())?;
    file.write_all(data).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    unsafe {
        if MoveFileExW(wide(temporary.to_str().ok_or("Invalid state path")?).as_ptr(), wide(path.to_str().ok_or("Invalid state path")?).as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) == 0 { return Err(err("atomic state replacement")); }
    }
    Ok(())
}
pub fn load_state() -> Result<Option<State>, String> {
    let path = root().join("state.json");
    match fs::read(path) {
        Ok(bytes) => { let state: State = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?; state.validate()?; Ok(Some(state)) }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
fn save(state: &State) -> Result<(), String> {
    state.validate()?;
    atomic_write(&root().join("state.json"), &serde_json::to_vec_pretty(state).map_err(|e| e.to_string())?)
}
fn transition(state: &mut State, stage: Stage, detail: impl Into<String>) -> Result<(), String> { state.set(stage, detail); save(state) }

fn execute(exe: &Path, args: &[&str], timeout: u64) -> Result<i32, String> {
    let out = fs::OpenOptions::new().create(true).append(true).open(diagnostic_path()).map_err(|e| e.to_string())?;
    let err_out = out.try_clone().map_err(|e| e.to_string())?;
    let mut child = Command::new(exe).creation_flags(CREATE_NO_WINDOW).current_dir(root()).env("PSModulePath", r"C:\Windows\System32\WindowsPowerShell\v1.0\Modules").args(args).stdin(Stdio::null()).stdout(out).stderr(err_out).spawn().map_err(|e| e.to_string())?;
    let begin = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? { return status.code().ok_or("Process exited without a code".into()); }
        if begin.elapsed() > Duration::from_secs(timeout) { let _ = child.kill(); let _ = child.wait(); return Err("Operation timed out; inspect setup.log before retrying.".into()); }
        thread::sleep(Duration::from_millis(200));
    }
}
fn ps(script: &str) -> Result<(), String> {
    let code = execute(Path::new(POWERSHELL), &["-NoProfile", "-NonInteractive", "-Command", &format!("$ErrorActionPreference='Stop'; {script}")], 120)?;
    if code != 0 { return Err(format!("Windows setup operation failed ({code}); see {}", diagnostic_path().display())); }
    Ok(())
}
fn boot_id() -> Result<String, String> {
    // CIM returns a stable identifier across retries within the same boot.
    ps(&format!("(Get-CimInstance Win32_OperatingSystem).LastBootUpTime.ToUniversalTime().Ticks.ToString() | Set-Content -Encoding ASCII '{ROOT}\\boot-id.txt'"))?;
    Ok(fs::read_to_string(root().join("boot-id.txt")).map_err(|e| e.to_string())?.trim().to_owned())
}

fn reject_reparse(path: &Path) -> Result<(), String> {
    use std::os::windows::fs::MetadataExt;
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(m) if m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 => return Err(format!("Refusing reparse point: {}", ancestor.display())),
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}
fn secure_root(client_sid: Option<&str>) -> Result<(), String> {
    reject_reparse(&root())?;
    if !root().exists() {
        let sid = client_sid.ok_or("No resumable installation found")?;
        if !valid_sid(sid) { return Err("Invalid client SID".into()); }
        unsafe {
            let mut sd = null_mut();
            let sddl = wide(&format!("O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;FRFX;;;{sid})"));
            if ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl.as_ptr(), 1, &mut sd, null_mut()) == 0 { return Err(err("setup directory security")); }
            let sa = SECURITY_ATTRIBUTES { nLength: size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor: sd, bInheritHandle: 0 };
            let ok = CreateDirectoryW(wide(ROOT).as_ptr(), &sa);
            LocalFree(sd);
            if ok == 0 { return Err(err("create protected setup directory")); }
        }
    }
    // Do not run executables or write logs in a precreated, user-writable directory.
    let script = format!(r#"$ErrorActionPreference='Stop'; $a=Get-Acl '{ROOT}'; $owner=$a.GetOwner([Security.Principal.SecurityIdentifier]).Value; if($owner -notin @('S-1-5-18','S-1-5-32-544')) {{ throw 'Untrusted setup owner' }}; foreach($r in $a.Access) {{ $sid=$r.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value; if($r.AccessControlType -eq 'Allow' -and $sid -notin @('S-1-5-18','S-1-5-32-544') -and ([int]$r.FileSystemRights -band 0xD0156)) {{ throw 'Writable setup directory' }} }}"#);
    let status = Command::new(POWERSHELL).creation_flags(CREATE_NO_WINDOW).env("PSModulePath", r"C:\Windows\System32\WindowsPowerShell\v1.0\Modules").args(["-NoProfile", "-NonInteractive", "-Command", &script]).status().map_err(|e| e.to_string())?;
    if !status.success() { return Err("Setup directory failed ownership/ACL validation.".into()); }
    for name in ["state.json", "setup.log", "worker.lock", "engine.lock", "boot-id.txt", SETUP_EXE, CLI_EXE] { reject_reparse(&root().join(name))?; }
    Ok(())
}
fn lock_worker() -> Result<Handle, String> { lock_at(&root().join("worker.lock")) }
fn lock_at(path: &Path) -> Result<Handle, String> {
    let path = wide(path.to_str().ok_or("Invalid lock path")?);
    let handle = unsafe { CreateFileW(path.as_ptr(), GENERIC_READ | GENERIC_WRITE, 0, null(), OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, null_mut()) };
    if handle == INVALID_HANDLE_VALUE { return Err("Ya hay un instalador trabajando (o no se puede bloquear su estado).".into()); }
    Ok(Handle(handle))
}

fn register_tasks(sid: &str) -> Result<(), String> {
    if !valid_sid(sid) { return Err("Invalid SID".into()); }
    let exe = root().join(SETUP_EXE);
    ps(&format!(r#"
$a=New-ScheduledTaskAction -Execute '{exe}' -Argument '--resume';
$t=New-ScheduledTaskTrigger -AtStartup;
$p=New-ScheduledTaskPrincipal -UserId 'S-1-5-18' -LogonType ServiceAccount -RunLevel Highest;
$s=New-ScheduledTaskSettingsSet -StartWhenAvailable -ExecutionTimeLimit (New-TimeSpan -Hours 2) -MultipleInstances IgnoreNew -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries;
Register-ScheduledTask -TaskName '{TASK}' -Action $a -Trigger $t -Principal $p -Settings $s -Force | Out-Null;
$a=New-ScheduledTaskAction -Execute '{exe}' -Argument '--gui';
$t=New-ScheduledTaskTrigger -AtLogOn -User '{sid}';
$p=New-ScheduledTaskPrincipal -UserId '{sid}' -LogonType Interactive -RunLevel Limited;
Register-ScheduledTask -TaskName '{UI_TASK}' -Action $a -Trigger $t -Principal $p -Settings $s -Force | Out-Null;
"#, exe=exe.display()))
}
fn remove_tasks() -> Result<(), String> {
    ps(&format!("foreach($n in @('{TASK}','{UI_TASK}')) {{ if(Get-ScheduledTask -TaskName $n -ErrorAction SilentlyContinue) {{ Unregister-ScheduledTask -TaskName $n -Confirm:$false }} }}"))
}

/// Launch a separate elevated worker. UAC cancellation is returned to the UI.
pub struct Worker(Handle);
impl Worker {
    pub fn exit_code(&self) -> Result<Option<u32>, String> {
        unsafe {
            if WaitForSingleObject(self.0.0, 0) == WAIT_TIMEOUT { return Ok(None); }
            let mut code = 0;
            if GetExitCodeProcess(self.0.0, &mut code) == 0 { return Err(err("worker result")); }
            Ok(Some(code))
        }
    }
}
pub fn request_action(action: &str, sid: &str) -> Result<Worker, String> {
    if !valid_sid(sid) || !matches!(action, "--start" | "--restart") { return Err("Invalid installer action".into()); }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let args = format!("{action} {sid}");
    let verb = wide("runas");
    let executable = wide(exe.to_str().ok_or("Invalid executable path")?);
    let parameters = wide(&args);
    let mut info: SHELLEXECUTEINFOW = unsafe { zeroed() };
    info.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
    info.lpVerb = verb.as_ptr(); info.lpFile = executable.as_ptr(); info.lpParameters = parameters.as_ptr(); info.nShow = SW_HIDE;
    if unsafe { ShellExecuteExW(&mut info) } == 0 { return Err(err("No se autorizó la elevación o no se pudo iniciar el instalador")); }
    if info.hProcess.is_null() { return Err("Windows no devolvió un identificador del motor.".into()); }
    Ok(Worker(Handle(info.hProcess)))
}

pub fn start(client_sid: Option<&str>) -> Result<i32, String> {
    if !elevated() { return Err("El motor de instalación requiere elevación.".into()); }
    secure_root(client_sid)?;
    let _lock = lock_worker()?;
    let mut state = match load_state()? {
        Some(state) => {
            if let Some(sid) = client_sid { if sid != state.client_sid { return Err("Esta instalación pertenece a otro usuario.".into()); } }
            state
        }
        None => {
            let sid = client_sid.ok_or("No installation state to resume")?;
            if base().exists() || account_sid(ACCOUNT).is_ok() { return Err("Existing installation/account requires administrator inspection; not adopting it.".into()); }
            // Refuse collisions rather than replacing unrelated scheduled tasks.
            ps(&format!("foreach($n in @('{TASK}','{UI_TASK}')) {{ if(Get-ScheduledTask -TaskName $n -ErrorAction SilentlyContinue) {{ throw 'Scheduled task name collision' }} }}"))?;
            let source = std::env::current_exe().map_err(|e| e.to_string())?;
            let companion = source.with_file_name(CLI_EXE);
            if !companion.is_file() { return Err(format!("Falta {CLI_EXE} junto al instalador.")); }
            fs::copy(&source, root().join(SETUP_EXE)).map_err(|e| e.to_string())?;
            fs::copy(companion, root().join(CLI_EXE)).map_err(|e| e.to_string())?;
            let state = State::new(sid.to_owned()); save(&state)?; state
        }
    };
    let result = resume(&mut state);
    match result {
        Ok(code) => Ok(code),
        Err(e) => { transition(&mut state, Stage::Failed, &e)?; Err(e) }
    }
}
fn resume(state: &mut State) -> Result<i32, String> {
    if state.stage == Stage::Complete { remove_tasks()?; return Ok(0); }
    register_tasks(&state.client_sid)?;
    let boot = boot_id()?;
    if state.reboot_still_pending(&boot) {
        transition(state, Stage::RebootRequired, "Guarda tu trabajo y reinicia. La instalación continuará automáticamente al arrancar.")?;
        return Ok(3010);
    }
    state.reboot_boot_id = None;
    if !base().join("config.json").exists() {
        if base().exists() || account_sid(ACCOUNT).is_ok() {
            return Err("Preparación parcial de cuenta/carpetas: se conservó para diagnóstico. No se sobrescriben recursos; requiere revisión administrativa.".into());
        }
        transition(state, Stage::Preparing, "Habilitando características de Windows y preparando WSL; puede tardar varios minutos.")?;
        state.provisioning_started = true; save(state)?;
        let sid = state.client_sid.clone();
        let code = execute(&root().join(SETUP_EXE), &["--engine-install", &sid], 1800)?;
        if code == 3010 {
            state.reboot_boot_id = Some(boot);
            transition(state, Stage::RebootRequired, "Windows necesita reiniciarse. Las tareas de reanudación ya están registradas.")?;
            return Ok(3010);
        }
        if code != 0 { return Err(format!("El motor de instalación terminó con código {code}. Consulta setup.log.")); }
    }
    if !state.provisioning_started { return Err("Refusing to adopt an unowned runtime installation.".into()); }
    ensure_service(&state.client_sid, state.stage == Stage::Failed)?;
    transition(state, Stage::Verifying, "Esperando confirmación del servicio dedicado; instalar el servicio no implica que Linux esté listo.")?;
    let begin = Instant::now();
    loop {
        if begin.elapsed() > Duration::from_secs(3600) { return Err("Tiempo límite esperando el entorno. Consulta runtime.log antes de reintentar.".into()); }
        let path = base().join("private/runtime-status.json");
        if let Ok(mut file) = fs::File::open(path) {
            let mut bytes = Vec::new();
            std::io::Read::by_ref(&mut file).take((MAX_MESSAGE + 1) as u64).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
            if bytes.len() > MAX_MESSAGE { return Err("Runtime report exceeds the protocol limit.".into()); }
            let mut report: Report = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            let phase = report.state.clone();
            let detail = report.detail.clone();
            report.validate_at(unix_now())?;
            if report.verified {
                transition(state, Stage::Complete, "WSL, systemd, cgroups v2 y Podman verificados. No se ha desplegado todavía una aplicación.")?;
                remove_tasks()?;
                return Ok(0);
            }
            if phase == "degraded" { return Err(detail); }
            let stage = match phase.as_str() { "downloading" => Stage::Downloading, "configuring" => Stage::Configuring, _ => Stage::Verifying };
            if state.stage != stage { transition(state, stage, "El servicio está trabajando bajo la cuenta dedicada. No cierres Windows durante la descarga/configuración.")?; }
        }
        thread::sleep(Duration::from_secs(2));
    }
}

fn ensure_service(client_sid: &str, retry: bool) -> Result<(), String> {
    let cfg = config()?;
    if cfg.client_sid != client_sid || cfg.account_sid != account_sid(ACCOUNT)? { return Err("Runtime identity does not match installer state.".into()); }
    // Check both executable and logon identity before starting an existing service.
    let repair = if retry { format!("Stop-Service '{SERVICE}' -ErrorAction Stop; Remove-Item '{BASE}\\private\\runtime-status.json' -ErrorAction SilentlyContinue; Start-Service '{SERVICE}'") } else { format!("if($s.State -eq 'Stopped') {{ Remove-Item '{BASE}\\private\\runtime-status.json' -ErrorAction SilentlyContinue; Start-Service '{SERVICE}' }}") };
    ps(&format!(r#"$s=Get-CimInstance Win32_Service -Filter "Name='{SERVICE}'"; if(!$s -or $s.PathName -ne '"{BASE}\{CLI_EXE}" --service' -or $s.StartName -ne '.\{ACCOUNT}') {{ throw 'Service configuration mismatch' }}; {repair}"#))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temp() -> PathBuf {
        let path = std::env::temp_dir().join(format!("gnx-state-test-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir(&path).unwrap(); path
    }
    #[test] fn atomic_state_replaces_existing_file() {
        let dir = temp(); let path = dir.join("state.json");
        atomic_write(&path, b"first").unwrap(); atomic_write(&path, b"second").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        assert!(!path.with_extension("new").exists()); fs::remove_dir_all(dir).unwrap();
    }
    #[test] fn worker_lock_is_exclusive_and_released() {
        let dir = temp(); let path = dir.join("lock");
        let first = lock_at(&path).unwrap(); assert!(lock_at(&path).is_err());
        drop(first); drop(lock_at(&path).unwrap()); fs::remove_dir_all(dir).unwrap();
    }
}

pub fn restart(client_sid: &str) -> Result<i32, String> {
    if !elevated() { return Err("Restart requires elevation".into()); }
    secure_root(Some(client_sid))?;
    let _lock = lock_worker()?;
    let state = load_state()?.ok_or("No installation to resume")?;
    if state.client_sid != client_sid || state.stage != Stage::RebootRequired { return Err("No hay un reinicio pendiente para esta instalación.".into()); }
    register_tasks(client_sid)?;
    // Do not use /f or a positive shutdown timeout
    // (Windows implies forced app closure for positive timeouts). Countdown lives in GUI.
    let code = execute(Path::new(r"C:\Windows\System32\shutdown.exe"), &["/r", "/t", "0"], 15)?;
    if code != 0 { return Err(format!("Windows rechazó el reinicio ({code}).")); }
    Ok(0)
}
