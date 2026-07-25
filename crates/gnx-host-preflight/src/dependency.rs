use std::env;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

use winreg::RegKey;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};

use crate::exit_codes::*;
use crate::journal::{InstallJournal, log_path};
use crate::staging::{StageError, StageRequest};
use crate::{checks, windows};

const UNINSTALL_ROOT: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall";

#[derive(Clone, Copy)]
pub enum Dependency {
    Wsl,
    Podman,
}

pub enum InstallOutcome {
    Success(String),
    Reboot(String),
}

#[derive(Debug)]
pub struct InstallFailure {
    pub code: &'static str,
    pub exit_code: i32,
    pub message: String,
}

#[derive(Clone, Copy)]
struct DependencySpec {
    id: &'static str,
    version: &'static str,
    file_name: &'static str,
    size: u64,
    sha256: &'static str,
    product_code: &'static str,
    installing_phase: &'static str,
    installed_phase: &'static str,
    log_name: &'static str,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum InstalledState {
    Absent,
    Compatible,
    Incompatible,
}

impl Dependency {
    fn spec(self) -> DependencySpec {
        match self {
            Self::Wsl => DependencySpec {
                id: "wsl",
                version: "2.7.10.0",
                file_name: "wsl.2.7.10.0.x64.msi",
                size: 258_605_056,
                sha256: "1A62F90A43C03CC5BDA47DFD0B6FAF496AC70FD4389190518120A4F84FC895CF",
                product_code: "{FC60318A-104F-41CF-A030-1AA3E98DBEFC}",
                installing_phase: "WSL_INSTALLING",
                installed_phase: "WSL_INSTALLED",
                log_name: "wsl-2.7.10.0-install.log",
            },
            Self::Podman => DependencySpec {
                id: "podman",
                version: "6.0.1",
                file_name: "podman-installer-windows-amd64.msi",
                size: 27_414_528,
                sha256: "3B65848F2D9AE652A15C35F2496A9ECE2E07F28746FA651415D519AE7C5902AD",
                product_code: "{661EDED1-C5BC-430C-8802-015B34A382FA}",
                installing_phase: "PODMAN_INSTALLING",
                installed_phase: "PODMAN_INSTALLED",
                log_name: "podman-6.0.1-install.log",
            },
        }
    }

    pub fn check_id(self) -> &'static str {
        match self {
            Self::Wsl => "wsl_install",
            Self::Podman => "podman_install",
        }
    }
}

pub fn install(dependency: Dependency) -> Result<InstallOutcome, InstallFailure> {
    let spec = dependency.spec();
    let mut journal = InstallJournal::load().map_err(journal_failure)?;
    journal.begin(spec.installing_phase).map_err(|error| {
        if error.message().contains("exceeded") {
            InstallFailure {
                code: "INSTALL_RESUME_LIMIT_REACHED",
                exit_code: INSTALL_RESUME_LIMIT,
                message: error.message().into(),
            }
        } else {
            journal_failure(error)
        }
    })?;

    match installed_state(spec).map_err(|failure| record_failure(&mut journal, failure))? {
        InstalledState::Compatible => {
            post_validate(dependency).map_err(|failure| record_failure(&mut journal, failure))?;
            journal
                .complete(spec.installed_phase)
                .map_err(journal_failure)?;
            return Ok(InstallOutcome::Success(format!(
                "pinned {} {} is already installed",
                spec.id, spec.version
            )));
        }
        InstalledState::Incompatible => {
            let failure = InstallFailure {
                code: match dependency {
                    Dependency::Wsl => "WSL_VERSION_INCOMPATIBLE",
                    Dependency::Podman => "PODMAN_VERSION_INCOMPATIBLE",
                },
                exit_code: match dependency {
                    Dependency::Wsl => WSL_UNAVAILABLE,
                    Dependency::Podman => PODMAN_INCOMPATIBLE,
                },
                message: format!(
                    "an incompatible {} MSI registration is present; expected {}",
                    spec.id, spec.version
                ),
            };
            return Err(record_failure(&mut journal, failure));
        }
        InstalledState::Absent => {}
    }

    let request = StageRequest {
        dependency_id: spec.id,
        version: spec.version,
        file_name: spec.file_name,
        expected_size: spec.size,
        expected_sha256: spec.sha256,
    };
    let staged = crate::staging::stage(&request)
        .map_err(|error| record_failure(&mut journal, stage_failure(error)))?;
    let log = log_path(spec.log_name).map_err(journal_failure)?;
    let exit_code =
        run_msi(&staged, &log).map_err(|failure| record_failure(&mut journal, failure))?;

    let state = installed_state(spec).map_err(|failure| record_failure(&mut journal, failure))?;
    if state != InstalledState::Compatible {
        let failure = InstallFailure {
            code: "DEPENDENCY_INSTALLATION_NOT_REGISTERED",
            exit_code: INSTALL_MSI_FAILED,
            message: format!(
                "{} MSI returned {exit_code} but the pinned registration was not present; log: {}",
                spec.id,
                log.display()
            ),
        };
        return Err(record_failure(&mut journal, failure));
    }

    if exit_code == 0 {
        post_validate(dependency).map_err(|failure| record_failure(&mut journal, failure))?;
    }
    journal
        .complete(spec.installed_phase)
        .map_err(journal_failure)?;

    let message = format!(
        "installed pinned {} {} from stable cache; log: {}",
        spec.id,
        spec.version,
        log.display()
    );
    if matches!(exit_code, 1641 | 3010) {
        Ok(InstallOutcome::Reboot(message))
    } else {
        Ok(InstallOutcome::Success(message))
    }
}

fn installed_state(spec: DependencySpec) -> Result<InstalledState, InstallFailure> {
    let path = format!("{UNINSTALL_ROOT}\\{}", spec.product_code);
    let key = match RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(&path, KEY_READ | KEY_WOW64_64KEY)
    {
        Ok(key) => key,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(InstalledState::Absent),
        Err(error) => {
            return Err(operational(format!(
                "cannot inspect {} MSI registration: {error}",
                spec.id
            )));
        }
    };
    let installer: u32 = registry_value(&key, "WindowsInstaller", spec.id)?;
    let version: String = registry_value(&key, "DisplayVersion", spec.id)?;
    if installer == 1 && version == spec.version {
        Ok(InstalledState::Compatible)
    } else {
        Ok(InstalledState::Incompatible)
    }
}

fn registry_value<T: winreg::types::FromRegValue>(
    key: &RegKey,
    name: &str,
    dependency: &str,
) -> Result<T, InstallFailure> {
    key.get_value(name).map_err(|error| {
        operational(format!(
            "cannot read {dependency} MSI registration value {name}: {error}"
        ))
    })
}

fn run_msi(msi: &Path, log: &Path) -> Result<i32, InstallFailure> {
    let msiexec = windows::system32_file("msiexec.exe").map_err(operational)?;
    let status = Command::new(msiexec)
        .arg("/i")
        .arg(msi)
        .arg("ALLUSERS=1")
        .arg("REBOOT=ReallySuppress")
        .arg("/qn")
        .arg("/L*V")
        .arg(log)
        .status()
        .map_err(|error| InstallFailure {
            code: "DEPENDENCY_MSI_LAUNCH_FAILED",
            exit_code: INSTALL_MSI_FAILED,
            message: format!(
                "cannot launch Windows Installer for {}: {error}",
                msi.display()
            ),
        })?;
    let code = status.code().ok_or_else(|| InstallFailure {
        code: "DEPENDENCY_MSI_TERMINATED",
        exit_code: INSTALL_MSI_FAILED,
        message: format!(
            "Windows Installer terminated while installing {}",
            msi.display()
        ),
    })?;
    if matches!(code, 0 | 1641 | 3010) {
        Ok(code)
    } else {
        Err(InstallFailure {
            code: "DEPENDENCY_MSI_FAILED",
            exit_code: INSTALL_MSI_FAILED,
            message: format!(
                "Windows Installer returned {code} for {}; log: {}",
                msi.display(),
                log.display()
            ),
        })
    }
}

fn post_validate(dependency: Dependency) -> Result<(), InstallFailure> {
    match dependency {
        Dependency::Wsl => {
            let wsl = windows::system32_file("wsl.exe").map_err(operational)?;
            let status = Command::new(&wsl)
                .args(["--set-default-version", "2"])
                .status()
                .map_err(|error| {
                    operational(format!("cannot set WSL default version 2: {error}"))
                })?;
            if !status.success() {
                return Err(InstallFailure {
                    code: "WSL2_DEFAULT_CONFIGURATION_FAILED",
                    exit_code: WSL_UNAVAILABLE,
                    message: "wsl --set-default-version 2 did not succeed".into(),
                });
            }
            match checks::wsl() {
                checks::Verdict::Pass(_) => Ok(()),
                checks::Verdict::Fail(message)
                | checks::Verdict::Error(message)
                | checks::Verdict::Reboot(message) => Err(InstallFailure {
                    code: "WSL_POST_VALIDATION_FAILED",
                    exit_code: WSL_UNAVAILABLE,
                    message,
                }),
            }
        }
        Dependency::Podman => {
            let program_files = env::var_os("ProgramFiles")
                .map(PathBuf::from)
                .ok_or_else(|| operational("ProgramFiles is unavailable"))?;
            let binary = program_files.join("Podman").join("podman.exe");
            if binary.is_file() {
                Ok(())
            } else {
                Err(InstallFailure {
                    code: "PODMAN_BINARY_MISSING",
                    exit_code: PODMAN_INCOMPATIBLE,
                    message: format!(
                        "pinned Podman MSI is registered but podman.exe is absent at {}",
                        binary.display()
                    ),
                })
            }
        }
    }
}

fn stage_failure(error: StageError) -> InstallFailure {
    match error {
        StageError::Missing(message) => InstallFailure {
            code: "INSTALL_PAYLOAD_MISSING",
            exit_code: INSTALL_PAYLOAD_MISSING,
            message,
        },
        StageError::Invalid(message) => InstallFailure {
            code: "INSTALL_PAYLOAD_INVALID",
            exit_code: INSTALL_PAYLOAD_INVALID,
            message,
        },
        StageError::Io(message) => InstallFailure {
            code: "INSTALL_STAGING_FAILED",
            exit_code: INSTALL_STAGING_FAILED,
            message,
        },
    }
}

fn record_failure(journal: &mut InstallJournal, failure: InstallFailure) -> InstallFailure {
    if let Err(error) = journal.record_error(failure.code, &failure.message) {
        return journal_failure(error);
    }
    failure
}

fn journal_failure(error: crate::journal::JournalError) -> InstallFailure {
    operational(error.message())
}

fn operational(message: impl Into<String>) -> InstallFailure {
    InstallFailure {
        code: "INSTALL_OPERATIONAL_ERROR",
        exit_code: OPERATIONAL_ERROR,
        message: message.into(),
    }
}
