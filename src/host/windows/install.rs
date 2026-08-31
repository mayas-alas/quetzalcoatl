use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::ptr::{self, null_mut};
use std::time::Duration;

use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_DELAY_UNTIL_REBOOT, MoveFileExW};
use windows_sys::Win32::System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject};
use windows_sys::Win32::UI::Shell::{
    IsUserAnAdmin, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    HWND_BROADCAST, MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MessageBoxW, SMTO_ABORTIFHUNG,
    SW_SHOWNORMAL, SendMessageTimeoutW, WM_SETTINGCHANGE,
};

use crate::config::{data_root, default_config_path};
use crate::error::GnxError;
use crate::host::windows::{account, download, reboot, service, wsl};
use crate::host::{InstallOptions, InstallOutcome, UninstallOutcome};
use crate::journal::{InstallCheckpoint, OperationJournal, default_journal_path};
use crate::process::CommandSpec;
use crate::state::{OperationalState, Stage, default_state_path};

const DEFAULT_CONFIG: &str = include_str!("../../../config.example.toml");
const REGISTRY_ENVIRONMENT: &str =
    r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
const REGISTRY_RUN: &str = r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
const TRAY_VALUE: &str = "QuetzalcoatlNextTray";

pub fn install(options: InstallOptions) -> Result<InstallOutcome, GnxError> {
    if !is_elevated() {
        if options.elevated {
            return Err(GnxError::new(
                "HOST_ELEVATION_REQUIRED",
                "host",
                "install",
                "La instalación no recibió un token elevado.",
                "Acepte el diálogo UAC y vuelva a intentar.",
                false,
                9,
            ));
        }
        let parameters = if options.resume {
            "__install --elevated --resume"
        } else {
            "__install --elevated"
        };
        let exit_code = elevate(parameters, "instalar WSL, Podman y el servicio")?;
        if exit_code != 0 {
            show_result(
                "Quetzalcoatl Next — instalación incompleta",
                &format!(
                    "El instalador terminó con código {exit_code}.\n\nConsulte: {}",
                    crate::logs::default_log_path().display()
                ),
                true,
            );
            return Err(GnxError::new(
                "INSTALL_ELEVATED_CHILD_FAILED",
                "install",
                "windows_elevate",
                format!("El proceso elevado terminó con código {exit_code}."),
                format!(
                    "Consulte gnx logs o {}.",
                    crate::logs::default_log_path().display()
                ),
                true,
                14,
            ));
        }
        launch_tray_for_current_user()?;
        show_install_result();
        return Ok(InstallOutcome::RelaunchedElevated);
    }

    let journal_path = default_journal_path();
    let mut journal = match OperationJournal::load(&journal_path)? {
        Some(journal)
            if journal.target_version != env!("CARGO_PKG_VERSION")
                && journal.checkpoint != InstallCheckpoint::Completed =>
        {
            return Err(GnxError::new(
                "INSTALL_VERSION_CONFLICT",
                "install",
                "resume",
                format!(
                    "Journal {} pertenece a GNX {}.",
                    journal.operation_id, journal.target_version
                ),
                "Reanude con el mismo binario antes de activar otro release.",
                false,
                15,
            ));
        }
        Some(journal) if journal.target_version == env!("CARGO_PKG_VERSION") => journal,
        Some(_) | None => OperationJournal::new_install(),
    };
    advance(&mut journal, InstallCheckpoint::Elevated, &journal_path)?;

    let installed_executable = install_files()?;
    register_tray(&installed_executable)?;
    advance(
        &mut journal,
        InstallCheckpoint::FilesInstalled,
        &journal_path,
    )?;

    let wsl_outcome = wsl::ensure()?;
    advance(&mut journal, InstallCheckpoint::WslEnabled, &journal_path)?;

    let podman_reboot = install_podman()?;
    advance(
        &mut journal,
        InstallCheckpoint::PodmanInstalled,
        &journal_path,
    )?;

    if wsl_outcome == wsl::WslOutcome::RebootRequired || podman_reboot {
        reboot::register_resume(&installed_executable)?;
        journal.reboot_required = true;
        journal.save(&journal_path)?;
        OperationalState {
            stage: Stage::RebootRequired,
            ..OperationalState::default()
        }
        .save(&default_state_path())?;
        println!("GNX instaló sus archivos y Podman. Windows debe reiniciarse para continuar.");
        return Ok(InstallOutcome::RebootRequired);
    }

    service::stop()?;
    let credential = account::ensure_runtime_account()?;
    service::register(&installed_executable, credential)?;
    account::grant_data_access(&data_root())?;
    advance(
        &mut journal,
        InstallCheckpoint::ServiceRegistered,
        &journal_path,
    )?;
    OperationalState {
        stage: Stage::Installed,
        ..OperationalState::default()
    }
    .save(&default_state_path())?;
    service::start()?;
    advance(
        &mut journal,
        InstallCheckpoint::MachineRequested,
        &journal_path,
    )?;
    journal.reboot_required = false;
    advance(&mut journal, InstallCheckpoint::Completed, &journal_path)?;
    reboot::unregister_resume()?;

    println!("GNX quedó en PATH. Abra una shell nueva y ejecute: gnx status");
    Ok(InstallOutcome::Installed)
}

pub fn uninstall(elevated: bool) -> Result<UninstallOutcome, GnxError> {
    if !is_elevated() {
        if elevated {
            return Err(GnxError::new(
                "HOST_ELEVATION_REQUIRED",
                "host",
                "uninstall",
                "La desinstalación no recibió un token elevado.",
                "Acepte el diálogo UAC y vuelva a intentar.",
                false,
                9,
            ));
        }
        let exit_code = elevate("uninstall --elevated", "retirar GNX y Podman CLI")?;
        if exit_code != 0 {
            return Err(GnxError::new(
                "UNINSTALL_ELEVATED_CHILD_FAILED",
                "install",
                "windows_elevate",
                format!("El proceso elevado terminó con código {exit_code}."),
                "Consulte el log persistente de GNX.",
                true,
                14,
            ));
        }
        return Ok(UninstallOutcome::RelaunchedElevated);
    }

    service::remove()?;
    unregister_tray()?;
    let podman_reboot = uninstall_podman()?;
    reboot::unregister_resume()?;
    let install_directory = install_directory();
    remove_from_machine_path(&install_directory)?;
    let binary_reboot = delete_or_schedule(&installed_executable())?;
    let _ = fs::remove_dir(&install_directory);
    OperationalState {
        stage: Stage::Uninstalled,
        ..OperationalState::default()
    }
    .save(&default_state_path())?;

    if podman_reboot || binary_reboot {
        Ok(UninstallOutcome::RebootRequired)
    } else {
        Ok(UninstallOutcome::Removed)
    }
}

pub fn installed_executable() -> PathBuf {
    install_directory().join("gnx.exe")
}

pub fn running_from_installed_path() -> bool {
    let Ok(current) = std::env::current_exe() else {
        return false;
    };
    current
        .display()
        .to_string()
        .eq_ignore_ascii_case(&installed_executable().display().to_string())
}

fn install_directory() -> PathBuf {
    std::env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"))
        .join("QuetzalcoatlNext")
}

fn advance(
    journal: &mut OperationJournal,
    checkpoint: InstallCheckpoint,
    path: &Path,
) -> Result<(), GnxError> {
    if journal.checkpoint < checkpoint {
        journal.advance(checkpoint)?;
        journal.save(path)?;
    }
    Ok(())
}

fn is_elevated() -> bool {
    // SAFETY: IsUserAnAdmin has no pointer arguments and only inspects the current token.
    unsafe { IsUserAnAdmin() != 0 }
}

fn elevate(parameters: &str, purpose: &str) -> Result<u32, GnxError> {
    let executable = std::env::current_exe()
        .map_err(|error| GnxError::io("windows_elevate", error.to_string()))?;
    let verb = wide("runas");
    let file = wide(executable.as_os_str());
    let parameters = wide(parameters);
    let mut execute = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: verb.as_ptr(),
        lpFile: file.as_ptr(),
        lpParameters: parameters.as_ptr(),
        nShow: SW_SHOWNORMAL,
        ..Default::default()
    };
    // SAFETY: all string buffers and SHELLEXECUTEINFOW remain live until the call returns.
    if unsafe { ShellExecuteExW(&mut execute) } == 0 || execute.hProcess.is_null() {
        return Err(GnxError::new(
            "HOST_ELEVATION_CANCELLED",
            "host",
            "windows_elevate",
            std::io::Error::last_os_error().to_string(),
            format!("Acepte UAC para que GNX pueda {purpose}."),
            true,
            9,
        ));
    }
    crate::logs::event(
        "info",
        "install",
        "windows_elevate",
        "Esperando al proceso elevado",
    );
    // SAFETY: hProcess is owned by this function until CloseHandle below.
    let waited = unsafe { WaitForSingleObject(execute.hProcess, INFINITE) };
    let mut exit_code = u32::MAX;
    let read_exit = waited == WAIT_OBJECT_0
        // SAFETY: hProcess is valid and exit_code points to writable memory.
        && unsafe { GetExitCodeProcess(execute.hProcess, &mut exit_code) } != 0;
    // SAFETY: hProcess was returned by ShellExecuteExW and is closed exactly once.
    unsafe { CloseHandle(execute.hProcess) };
    if !read_exit {
        return Err(GnxError::io(
            "windows_elevate_wait",
            std::io::Error::last_os_error().to_string(),
        ));
    }
    Ok(exit_code)
}

fn install_files() -> Result<PathBuf, GnxError> {
    let install_directory = install_directory();
    let destination = installed_executable();
    fs::create_dir_all(&install_directory)
        .map_err(|error| GnxError::io("windows_files", error.to_string()))?;
    let source = std::env::current_exe()
        .map_err(|error| GnxError::io("windows_files", error.to_string()))?;
    if !paths_equal(&source, &destination) {
        if destination.exists() {
            service::stop()?;
        }
        fs::copy(&source, &destination)
            .map_err(|error| GnxError::io("windows_files", error.to_string()))?;
    }

    fs::create_dir_all(data_root())
        .map_err(|error| GnxError::io("windows_files", error.to_string()))?;
    let config_path = default_config_path();
    if !config_path.exists() {
        fs::write(&config_path, DEFAULT_CONFIG)
            .map_err(|error| GnxError::io("windows_config", error.to_string()))?;
    }
    add_to_machine_path(&install_directory)?;
    Ok(destination)
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    left.display()
        .to_string()
        .eq_ignore_ascii_case(&right.display().to_string())
}

fn query_machine_path() -> Result<String, GnxError> {
    let query = CommandSpec::new(r"C:\Windows\System32\reg.exe")
        .args(["QUERY", REGISTRY_ENVIRONMENT, "/v", "Path"])
        .run_checked("path_query")?;
    query
        .stdout
        .lines()
        .find_map(|line| {
            line.split_once("REG_EXPAND_SZ")
                .map(|(_, value)| value.trim().to_string())
        })
        .or_else(|| {
            query.stdout.lines().find_map(|line| {
                line.split_once("REG_SZ")
                    .map(|(_, value)| value.trim().to_string())
            })
        })
        .ok_or_else(|| GnxError::io("path_query", "No se pudo interpretar PATH de máquina"))
}

fn write_machine_path(value: String) -> Result<(), GnxError> {
    CommandSpec::new(r"C:\Windows\System32\reg.exe")
        .args([
            "ADD",
            REGISTRY_ENVIRONMENT,
            "/v",
            "Path",
            "/t",
            "REG_EXPAND_SZ",
            "/d",
        ])
        .arg(value)
        .arg("/f")
        .run_checked("path_update")?;
    broadcast_environment_change();
    Ok(())
}

fn broadcast_environment_change() {
    let environment = wide("Environment");
    let mut result = 0_usize;
    // SAFETY: UTF-16 data remains live for this bounded, synchronous broadcast.
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            environment.as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            5_000,
            &mut result,
        );
    }
}

fn add_to_machine_path(install_directory: &Path) -> Result<(), GnxError> {
    let current_path = query_machine_path()?;
    if current_path.split(';').any(|entry| {
        entry
            .trim()
            .eq_ignore_ascii_case(&install_directory.display().to_string())
    }) {
        return Ok(());
    }
    write_machine_path(format!("{current_path};{}", install_directory.display()))
}

fn remove_from_machine_path(install_directory: &Path) -> Result<(), GnxError> {
    let current_path = query_machine_path()?;
    let filtered = current_path
        .split(';')
        .filter(|entry| {
            !entry
                .trim()
                .eq_ignore_ascii_case(&install_directory.display().to_string())
        })
        .collect::<Vec<_>>()
        .join(";");
    if filtered != current_path {
        write_machine_path(filtered)?;
    }
    Ok(())
}

fn register_tray(executable: &Path) -> Result<(), GnxError> {
    let command = format!("\"{}\" __tray", executable.display());
    CommandSpec::new(r"C:\Windows\System32\reg.exe")
        .args(["ADD", REGISTRY_RUN, "/v", TRAY_VALUE, "/t", "REG_SZ", "/d"])
        .arg(command)
        .arg("/f")
        .run_checked("tray_register")?;
    Ok(())
}

fn unregister_tray() -> Result<(), GnxError> {
    let output = CommandSpec::new(r"C:\Windows\System32\reg.exe")
        .args(["DELETE", REGISTRY_RUN, "/v", TRAY_VALUE, "/f"])
        .run("tray_unregister")?;
    if output.success() || output.exit_code == Some(1) {
        Ok(())
    } else {
        Err(GnxError::process(
            "tray_unregister",
            Path::new(r"C:\Windows\System32\reg.exe"),
            output.stderr,
            true,
        ))
    }
}

fn launch_tray_for_current_user() -> Result<(), GnxError> {
    let executable = installed_executable();
    if !executable.exists() {
        return Err(GnxError::io(
            "tray_launch",
            format!("No existe {}.", executable.display()),
        ));
    }
    Command::new(&executable)
        .arg("__tray")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| GnxError::io("tray_launch", error.to_string()))?;
    crate::logs::event(
        "info",
        "tray",
        "launch",
        "Bandeja iniciada en la sesión interactiva",
    );
    Ok(())
}

fn show_install_result() {
    let state = OperationalState::load(&default_state_path())
        .ok()
        .flatten()
        .unwrap_or_default();
    let body = if state.stage == Stage::RebootRequired {
        "La preparación del host terminó y Windows debe reiniciarse. GNX continuará automáticamente después del inicio de sesión.".to_string()
    } else {
        format!(
            "La instalación base terminó. Abra una shell nueva y ejecute gnx status.\n\nLogs: {}",
            crate::logs::default_log_path().display()
        )
    };
    show_result("Quetzalcoatl Next", &body, false);
}

fn show_result(title: &str, body: &str, error: bool) {
    let title = wide(title);
    let body = wide(body);
    // SAFETY: strings are NUL-terminated and no owner window is required.
    unsafe {
        MessageBoxW(
            null_mut(),
            body.as_ptr(),
            title.as_ptr(),
            MB_OK
                | if error {
                    MB_ICONERROR
                } else {
                    MB_ICONINFORMATION
                },
        );
    }
}

fn install_podman() -> Result<bool, GnxError> {
    let podman = std::env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"))
        .join("Podman")
        .join("podman.exe");
    if CommandSpec::new(&podman)
        .arg("--version")
        .timeout(Duration::from_secs(30))
        .run("podman_version")
        .is_ok_and(|output| output.success())
    {
        return Ok(false);
    }

    let dependency = download::podman_dependency()?;
    println!(
        "Descargando {} {} desde el release oficial…",
        dependency.publisher, dependency.version
    );
    let installer = download::download_verified(&dependency, &data_root().join("cache"))?;
    let log_path = data_root().join("podman-msi.log");
    let output = CommandSpec::new(r"C:\Windows\System32\msiexec.exe")
        .arg("/package")
        .arg(installer)
        .arg("/quiet")
        .arg("/norestart")
        .arg("ALLUSERS=1")
        .arg("MACHINE_PROVIDER=wsl")
        .arg("/l*v")
        .arg(log_path)
        .timeout(Duration::from_secs(1800))
        .run("podman_install")?;
    match output.exit_code {
        Some(0) => Ok(false),
        Some(1641 | 3010) => Ok(true),
        _ => Err(GnxError::process(
            "podman_install",
            Path::new(r"C:\Windows\System32\msiexec.exe"),
            output.stderr,
            true,
        )),
    }
}

fn uninstall_podman() -> Result<bool, GnxError> {
    let dependency = download::podman_dependency()?;
    let installer = download::download_verified(&dependency, &data_root().join("cache"))?;
    let log_path = data_root().join("podman-msi-uninstall.log");
    let output = CommandSpec::new(r"C:\Windows\System32\msiexec.exe")
        .arg("/uninstall")
        .arg(installer)
        .arg("/quiet")
        .arg("/norestart")
        .arg("/l*v")
        .arg(log_path)
        .timeout(Duration::from_secs(1800))
        .run("podman_uninstall")?;
    match output.exit_code {
        Some(0 | 1605) => Ok(false),
        Some(1641 | 3010) => Ok(true),
        _ => Err(GnxError::process(
            "podman_uninstall",
            Path::new(r"C:\Windows\System32\msiexec.exe"),
            output.stderr,
            true,
        )),
    }
}

fn delete_or_schedule(path: &Path) -> Result<bool, GnxError> {
    match fs::remove_file(path) {
        Ok(()) => return Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => {}
    }
    let encoded = wide(path.as_os_str());
    // SAFETY: path is a NUL-terminated UTF-16 buffer and NULL destination requests deletion.
    let result = unsafe { MoveFileExW(encoded.as_ptr(), ptr::null(), MOVEFILE_DELAY_UNTIL_REBOOT) };
    if result == 0 {
        Err(GnxError::io(
            "windows_binary_remove",
            std::io::Error::last_os_error().to_string(),
        ))
    } else {
        Ok(true)
    }
}

fn wide(value: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    value
        .as_ref()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
