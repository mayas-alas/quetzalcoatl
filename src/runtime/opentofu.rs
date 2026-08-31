use std::path::PathBuf;

use serde::Deserialize;

use crate::download::Artifact;
use crate::error::GnxError;

pub const SYSTEMD_UNIT: &str = include_str!("../../runtime/gnx-opentofu.service");
pub const RUNNER_BOOTSTRAP: &str = include_str!("../../guest/infra-runner-bootstrap.sh");
pub const RUNNER_COMMAND: &str = include_str!("../../guest/infra-runner-run.sh");
pub const RUNNER_SYSTEMD_UNIT: &str = include_str!("../../guest/units/gnx-opentofu.service");
pub const VERSIONS_TF: &str = include_str!("../../infra/opentofu/versions.tf");
pub const VARIABLES_TF: &str = include_str!("../../infra/opentofu/variables.tf");
pub const MAIN_TF: &str = include_str!("../../infra/opentofu/main.tf");
pub const OUTPUTS_TF: &str = include_str!("../../infra/opentofu/outputs.tf");
pub const PROVIDER_LOCK: &str = include_str!("../../infra/opentofu/.terraform.lock.hcl");

const DEPENDENCY_LOCK: &str = include_str!("../../dependencies.lock.toml");

#[derive(Debug, Deserialize)]
struct LockFile {
    runtime: RuntimeDependencies,
}

#[derive(Debug, Deserialize)]
struct RuntimeDependencies {
    opentofu: OpenTofuDependency,
}

#[derive(Debug, Deserialize)]
pub struct OpenTofuDependency {
    pub id: String,
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

pub fn dependency() -> Result<OpenTofuDependency, GnxError> {
    toml::from_str::<LockFile>(DEPENDENCY_LOCK)
        .map(|lock| lock.runtime.opentofu)
        .map_err(|error| {
            GnxError::new(
                "DEPENDENCY_LOCK_INVALID",
                "opentofu",
                "lock_parse",
                error.to_string(),
                "Corrija dependencies.lock.toml antes de instalar.",
                false,
                10,
            )
        })
}

pub fn download() -> Result<(OpenTofuDependency, PathBuf), GnxError> {
    let dependency = dependency()?;
    let path = crate::download::download_verified(
        Artifact {
            id: &dependency.id,
            url: &dependency.url,
            sha256: &dependency.sha256,
            size: dependency.size,
        },
        &crate::config::data_root().join("cache"),
    )?;
    Ok((dependency, path))
}

pub fn validate_unit() -> bool {
    SYSTEMD_UNIT.contains("Type=oneshot")
        && SYSTEMD_UNIT.contains("proxmox.service")
        && SYSTEMD_UNIT.contains("infra-runner-bootstrap.sh")
        && !SYSTEMD_UNIT.contains("/usr/local/bin/tofu")
        && RUNNER_BOOTSTRAP.contains("runner_vmid=200")
        && RUNNER_BOOTSTRAP.contains("pct create")
        && RUNNER_BOOTSTRAP.contains("PROXMOX_VE_API_TOKEN")
        && RUNNER_COMMAND.contains("tofu apply")
        && RUNNER_SYSTEMD_UNIT.contains("Type=oneshot")
        && VERSIONS_TF.contains("bpg/proxmox")
        && VERSIONS_TF.contains("0.111.1")
        && MAIN_TF.contains("proxmox_virtual_environment_container")
        && MAIN_TF.contains("unprivileged  = true")
        && !MAIN_TF.contains("mount_point")
        && !SYSTEMD_UNIT.contains("local-exec")
        && !SYSTEMD_UNIT.contains("remote-exec")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opentofu_is_one_shot() {
        assert!(validate_unit());
    }

    #[test]
    fn opentofu_is_not_executed_in_podman_machine() {
        assert!(!SYSTEMD_UNIT.contains("tofu init"));
        assert!(RUNNER_COMMAND.contains("/usr/local/bin/tofu init"));
    }
}
