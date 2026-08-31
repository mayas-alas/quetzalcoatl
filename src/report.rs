use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use crate::config::Config;
use crate::error::GnxError;
use crate::process::CommandSpec;
use crate::state::{OperationalState, default_state_path};

#[derive(Debug, Serialize)]
pub struct StatusReport {
    pub schema: u32,
    pub product_version: &'static str,
    pub stage: String,
    pub host: HostReport,
    pub config_path: String,
    pub controller_url: Option<String>,
    pub controller: &'static str,
    pub machine: String,
    pub mesh: String,
    pub docktail: String,
    pub proxmox: String,
    pub infra: String,
    pub note: &'static str,
}

#[derive(Debug, Serialize)]
pub struct HostReport {
    pub os: &'static str,
    pub architecture: &'static str,
}

impl StatusReport {
    pub fn collect(config_path: &Path) -> Result<Self, GnxError> {
        let controller_url = if config_path.exists() {
            let config = Config::load(config_path)?;
            Some(config.validate()?.canonical().to_string())
        } else {
            None
        };

        let state = OperationalState::load(&default_state_path())?.unwrap_or_default();
        Ok(Self {
            schema: 1,
            product_version: env!("CARGO_PKG_VERSION"),
            stage: state.stage.as_str().to_string(),
            host: HostReport {
                os: std::env::consts::OS,
                architecture: std::env::consts::ARCH,
            },
            config_path: config_path.display().to_string(),
            controller_url,
            controller: "not_checked",
            machine: state.machine,
            mesh: state.mesh,
            docktail: state.docktail,
            proxmox: state.proxmox,
            infra: state.infra,
            note: "GNX sólo reporta READY después de completar todos los gates.",
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckState {
    Pass,
    Fail,
    NotImplemented,
}

#[derive(Debug, Serialize)]
pub struct DoctorCheck {
    pub id: &'static str,
    pub state: CheckState,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    pub fn collect(config_path: &Path) -> Self {
        let mut checks = Vec::new();

        let supported_host = matches!(std::env::consts::OS, "windows" | "linux")
            && std::env::consts::ARCH == "x86_64";
        checks.push(DoctorCheck {
            id: "host.target",
            state: if supported_host {
                CheckState::Pass
            } else {
                CheckState::Fail
            },
            detail: format!("{} / {}", std::env::consts::OS, std::env::consts::ARCH),
        });

        match Config::load(config_path) {
            Ok(config) => match config.validate() {
                Ok(controller) => {
                    checks.push(DoctorCheck {
                        id: "config.schema",
                        state: CheckState::Pass,
                        detail: config_path.display().to_string(),
                    });
                    checks.push(DoctorCheck {
                        id: "mesh.controller_policy",
                        state: CheckState::Pass,
                        detail: controller.canonical().to_string(),
                    });
                    match crate::runtime::headscale::verify_controller(&controller) {
                        Ok(status) => checks.push(DoctorCheck {
                            id: "mesh.controller_tls",
                            state: CheckState::Pass,
                            detail: format!("HTTPS respondió con status {status}"),
                        }),
                        Err(error) => checks.push(DoctorCheck {
                            id: "mesh.controller_tls",
                            state: CheckState::Fail,
                            detail: format!("{}: {}", error.code, error.message),
                        }),
                    }
                }
                Err(error) => checks.push(DoctorCheck {
                    id: "config.validation",
                    state: CheckState::Fail,
                    detail: format!("{}: {}", error.code, error.message),
                }),
            },
            Err(error) => checks.push(DoctorCheck {
                id: "config.load",
                state: CheckState::Fail,
                detail: format!("{}: {}", error.code, error.message),
            }),
        }

        let podman = crate::runtime::machine::podman_executable();
        let podman_ready = push_process_check(
            &mut checks,
            "host.podman_cli",
            CommandSpec::new(&podman)
                .arg("--version")
                .timeout(Duration::from_secs(30)),
            "podman disponible",
        );
        if podman_ready {
            let machine_ready = push_process_check(
                &mut checks,
                "runtime.machine",
                CommandSpec::new(&podman)
                    .args(["machine", "inspect", crate::config::MACHINE_NAME])
                    .timeout(Duration::from_secs(60)),
                "Podman Machine quetzalcoatl existe",
            );
            if machine_ready {
                push_process_check(
                    &mut checks,
                    "runtime.kvm",
                    CommandSpec::new(&podman)
                        .args([
                            "machine",
                            "ssh",
                            crate::config::MACHINE_NAME,
                            "test",
                            "-c",
                            "/dev/kvm",
                        ])
                        .timeout(Duration::from_secs(30)),
                    "/dev/kvm disponible en la celda runtime",
                );
                push_process_check(
                    &mut checks,
                    "runtime.opentofu",
                    CommandSpec::new(&podman)
                        .args([
                            "machine",
                            "ssh",
                            crate::config::MACHINE_NAME,
                            "podman",
                            "exec",
                            "gnx-proxmox",
                            "pct",
                            "exec",
                            "200",
                            "--",
                            "/usr/local/bin/tofu",
                            "version",
                        ])
                        .timeout(Duration::from_secs(30)),
                    "OpenTofu instalado dentro del LXC gnx-infra-runner",
                );
            }
        }

        checks.push(DoctorCheck {
            id: "docktail.headscale_services",
            state: CheckState::NotImplemented,
            detail: "Bloqueado por MESH-SVC-01; nunca equivale a READY.".to_string(),
        });

        Self { checks }
    }

    pub fn has_blockers(&self) -> bool {
        self.checks
            .iter()
            .any(|check| !matches!(check.state, CheckState::Pass))
    }
}

fn push_process_check(
    checks: &mut Vec<DoctorCheck>,
    id: &'static str,
    command: CommandSpec,
    success_detail: &'static str,
) -> bool {
    match command.run("doctor_process") {
        Ok(output) if output.success() => {
            checks.push(DoctorCheck {
                id,
                state: CheckState::Pass,
                detail: success_detail.to_string(),
            });
            true
        }
        Ok(output) => {
            checks.push(DoctorCheck {
                id,
                state: CheckState::Fail,
                detail: format!("exit {:?}: {}", output.exit_code, output.stderr.trim()),
            });
            false
        }
        Err(error) => {
            checks.push(DoctorCheck {
                id,
                state: CheckState::Fail,
                detail: format!("{}: {}", error.code, error.message),
            });
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_check_blocks_healthy_diagnosis() {
        let report = DoctorReport {
            checks: vec![DoctorCheck {
                id: "mesh.gate",
                state: CheckState::NotImplemented,
                detail: "pending".to_string(),
            }],
        };

        assert!(report.has_blockers());
    }
}
