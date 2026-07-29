use super::*;

const HOST_PROFILE_SCHEMA_VERSION: u8 = 1;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HostProfile {
    pub(super) schema_version: u8,
    pub(super) product_version: String,
    pub(super) detected: DetectedResources,
    pub(super) selected: MachineProfile,
    pub(super) supported: bool,
    pub(super) cluster_member_supported: bool,
    pub(super) warnings: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DetectedResources {
    pub(super) logical_cpus: u64,
    pub(super) total_memory_mib: u64,
    pub(super) system_disk_total_gib: u64,
    pub(super) system_disk_free_gib: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MachineProfile {
    pub(super) capability: String,
    pub(super) machine_cpus: u64,
    pub(super) machine_memory_mib: u64,
    pub(super) machine_disk_gib: u64,
    pub(super) windows_memory_reserve_mib: u64,
}

pub(super) fn load_host_profile() -> Result<HostProfile, GateError> {
    let program_data = env::var_os("ProgramData")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| {
            GateError::new(
                "HOST_PROFILE_INVALID",
                Component::None,
                "ProgramData is unavailable while loading the managed host profile",
            )
        })?;
    let path = program_data
        .join("Quetzalcoatl")
        .join("Installer")
        .join("host-profile.json");
    let bytes = fs::read(&path).map_err(|error| {
        GateError::new(
            "HOST_PROFILE_MISSING",
            Component::None,
            format!(
                "managed host profile is unavailable at {}: {error}; rerun QuetzalcoatlSetup.exe",
                path.display()
            ),
        )
    })?;
    let profile: HostProfile = serde_json::from_slice(&bytes).map_err(|error| {
        GateError::new(
            "HOST_PROFILE_INVALID",
            Component::None,
            format!("managed host profile contains invalid JSON: {error}"),
        )
    })?;
    validate_host_profile(&profile)?;
    Ok(profile)
}

pub(super) fn managed_wsl_config(profile: &HostProfile) -> String {
    format!(
        "[wsl2]\nprocessors={}\nmemory={}MB\nswap=2GB\nnestedVirtualization=true\n",
        profile.selected.machine_cpus, profile.selected.machine_memory_mib
    )
}

fn validate_host_profile(profile: &HostProfile) -> Result<(), GateError> {
    if profile.schema_version != HOST_PROFILE_SCHEMA_VERSION
        || profile.product_version != env!("CARGO_PKG_VERSION")
    {
        return Err(GateError::new(
            "HOST_PROFILE_INVALID",
            Component::None,
            "managed host profile belongs to an unsupported product or schema version; rerun QuetzalcoatlSetup.exe",
        ));
    }
    if !profile.supported {
        return Err(GateError::new(
            "HOST_RESOURCES_INSUFFICIENT",
            Component::None,
            profile
                .warnings
                .first()
                .cloned()
                .unwrap_or_else(|| "managed host profile is not runtime-capable".into()),
        ));
    }
    let selected = &profile.selected;
    if !(1..=6).contains(&selected.machine_cpus)
        || !(2048..=8192).contains(&selected.machine_memory_mib)
        || !(40..=100).contains(&selected.machine_disk_gib)
        || selected.windows_memory_reserve_mib < 3072
        || profile.detected.logical_cpus < selected.machine_cpus
        || profile.detected.total_memory_mib
            < selected
                .machine_memory_mib
                .saturating_add(selected.windows_memory_reserve_mib)
        || profile.detected.system_disk_total_gib < profile.detected.system_disk_free_gib
        || profile.detected.system_disk_free_gib < selected.machine_disk_gib.saturating_add(20)
    {
        return Err(GateError::new(
            "HOST_PROFILE_INVALID",
            Component::None,
            "managed host profile contains unsafe or inconsistent resource values",
        ));
    }
    if !matches!(
        selected.capability.as_str(),
        "lab" | "runtime" | "cluster-member"
    ) {
        return Err(GateError::new(
            "HOST_PROFILE_INVALID",
            Component::None,
            "managed host profile contains an unsupported capability",
        ));
    }
    if profile.cluster_member_supported && selected.capability != "cluster-member" {
        return Err(GateError::new(
            "HOST_PROFILE_INVALID",
            Component::None,
            "managed host profile has inconsistent cluster capability",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> HostProfile {
        HostProfile {
            schema_version: HOST_PROFILE_SCHEMA_VERSION,
            product_version: env!("CARGO_PKG_VERSION").into(),
            detected: DetectedResources {
                logical_cpus: 4,
                total_memory_mib: 5864,
                system_disk_total_gib: 100,
                system_disk_free_gib: 80,
            },
            selected: MachineProfile {
                capability: "lab".into(),
                machine_cpus: 2,
                machine_memory_mib: 2560,
                machine_disk_gib: 60,
                windows_memory_reserve_mib: 3072,
            },
            supported: true,
            cluster_member_supported: false,
            warnings: vec!["laboratory profile".into()],
        }
    }

    #[test]
    fn creates_wsl_configuration_from_the_selected_profile() {
        assert_eq!(
            managed_wsl_config(&profile()),
            "[wsl2]\nprocessors=2\nmemory=2560MB\nswap=2GB\nnestedVirtualization=true\n"
        );
    }

    #[test]
    fn accepts_the_bounded_lab_profile() {
        validate_host_profile(&profile()).expect("valid profile");
    }
}
