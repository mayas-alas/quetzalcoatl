#[cfg(windows)]
use std::env;
#[cfg(windows)]
use std::fs::{self, OpenOptions};
#[cfg(windows)]
use std::io::Write;
#[cfg(windows)]
use std::process::Command;

use gnx_contracts::{DetectedResources, HostProfile, MachineProfile};
use serde::Deserialize;

const PROFILE_SCHEMA_VERSION: u8 = 1;
const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;
const MIN_HOST_MEMORY_MIB: u64 = 4096;
const MIN_MACHINE_MEMORY_MIB: u64 = 2048;
const MIN_LOGICAL_CPUS: u64 = 4;
const MIN_MACHINE_DISK_GIB: u64 = 40;
const WINDOWS_DISK_RESERVE_GIB: u64 = 20;
const MAX_MACHINE_MEMORY_MIB: u64 = 8192;
const MAX_MACHINE_CPUS: u64 = 6;
const MAX_MACHINE_DISK_GIB: u64 = 100;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResources {
    logical_cpus: u64,
    total_memory_bytes: u64,
    system_disk_total_bytes: u64,
    system_disk_free_bytes: u64,
}

#[derive(Debug)]
pub struct ProfileError {
    message: String,
}

impl ProfileError {
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[cfg(windows)]
pub fn detect_and_store(maintenance_requested: bool) -> Result<HostProfile, ProfileError> {
    let raw = detect_resources()?;
    let maintenance = maintenance_requested || installed_product_exists()?;
    let profile = calculate(raw, maintenance);
    store(&profile)?;
    Ok(profile)
}

pub fn summary(profile: &HostProfile) -> String {
    format!(
        "CPU={} logical; RAM={} MiB; disk={} GiB free/{} GiB total; profile={}; machine={} CPU, {} MiB RAM, {} GiB disk; cluster_member_supported={}",
        profile.detected.logical_cpus,
        profile.detected.total_memory_mib,
        profile.detected.system_disk_free_gib,
        profile.detected.system_disk_total_gib,
        profile.selected.capability,
        profile.selected.machine_cpus,
        profile.selected.machine_memory_mib,
        profile.selected.machine_disk_gib,
        profile.cluster_member_supported,
    )
}

#[cfg(windows)]
fn detect_resources() -> Result<RawResources, ProfileError> {
    let powershell =
        crate::windows::system32_file("WindowsPowerShell\\v1.0\\powershell.exe").map_err(error)?;
    let script = r#"$ErrorActionPreference='Stop';$computer=Get-CimInstance -ClassName Win32_ComputerSystem;$drive=Get-CimInstance -ClassName Win32_LogicalDisk | Where-Object { $_.DeviceID -eq $env:SystemDrive } | Select-Object -First 1;if ($null -eq $drive) { throw 'system drive was not found' };[ordered]@{logical_cpus=[uint64]$computer.NumberOfLogicalProcessors;total_memory_bytes=[uint64]$computer.TotalPhysicalMemory;system_disk_total_bytes=[uint64]$drive.Size;system_disk_free_bytes=[uint64]$drive.FreeSpace}|ConvertTo-Json -Compress"#;
    let output = Command::new(&powershell)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
        .map_err(|run_error| {
            error(format!(
                "cannot launch host resource inventory using {}: {run_error}",
                powershell.display()
            ))
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(error(format!(
            "host resource inventory failed with exit code {:?}: {}",
            output.status.code(),
            stderr.trim()
        )));
    }
    serde_json::from_slice(&output.stdout).map_err(|parse_error| {
        error(format!(
            "host resource inventory returned invalid JSON: {parse_error}"
        ))
    })
}

fn calculate(raw: RawResources, maintenance: bool) -> HostProfile {
    let detected = DetectedResources {
        logical_cpus: raw.logical_cpus,
        total_memory_mib: raw.total_memory_bytes / MIB,
        system_disk_total_gib: raw.system_disk_total_bytes / GIB,
        system_disk_free_gib: raw.system_disk_free_bytes / GIB,
    };

    let windows_memory_reserve_mib = if detected.total_memory_mib < 8192 {
        3072
    } else {
        4096
    };
    let available_memory = detected
        .total_memory_mib
        .saturating_sub(windows_memory_reserve_mib);
    let machine_memory_mib = round_down(available_memory.min(MAX_MACHINE_MEMORY_MIB), 512);
    let machine_cpus = detected
        .logical_cpus
        .saturating_sub(2)
        .clamp(1, MAX_MACHINE_CPUS);
    let available_disk = detected
        .system_disk_free_gib
        .saturating_sub(WINDOWS_DISK_RESERVE_GIB);
    let fresh_machine_disk_gib = round_down(available_disk.min(MAX_MACHINE_DISK_GIB), 5);
    let machine_disk_gib = if maintenance {
        fresh_machine_disk_gib.max(MIN_MACHINE_DISK_GIB)
    } else {
        fresh_machine_disk_gib
    };

    let memory_supported = detected.total_memory_mib >= MIN_HOST_MEMORY_MIB
        && machine_memory_mib >= MIN_MACHINE_MEMORY_MIB;
    let cpu_supported = detected.logical_cpus >= MIN_LOGICAL_CPUS && machine_cpus >= 2;
    let disk_supported = if maintenance {
        detected.system_disk_total_gib
            >= WINDOWS_DISK_RESERVE_GIB.saturating_add(MIN_MACHINE_DISK_GIB)
    } else {
        machine_disk_gib >= MIN_MACHINE_DISK_GIB
    };
    let supported = memory_supported && cpu_supported && disk_supported;
    let cluster_member_supported = supported
        && detected.total_memory_mib >= 12 * 1024
        && machine_memory_mib >= 6 * 1024
        && machine_cpus >= 4;

    let capability = if !supported {
        "install-only"
    } else if cluster_member_supported {
        "cluster-member"
    } else if detected.total_memory_mib >= 8192 {
        "runtime"
    } else {
        "lab"
    };

    let mut warnings = Vec::new();
    if !memory_supported {
        warnings.push(format!(
            "memory is insufficient: detected {} MiB; at least {} MiB and {} MiB assignable to the managed machine are required",
            detected.total_memory_mib, MIN_HOST_MEMORY_MIB, MIN_MACHINE_MEMORY_MIB
        ));
    }
    if !cpu_supported {
        warnings.push(format!(
            "CPU capacity is insufficient: detected {} logical processors; at least {} are required",
            detected.logical_cpus, MIN_LOGICAL_CPUS
        ));
    }
    if !disk_supported {
        warnings.push(format!(
            "disk capacity is insufficient: detected {} GiB free; at least {} GiB free is required to reserve Windows space and create the managed disk",
            detected.system_disk_free_gib,
            WINDOWS_DISK_RESERVE_GIB + MIN_MACHINE_DISK_GIB
        ));
    } else if maintenance
        && detected.system_disk_free_gib < WINDOWS_DISK_RESERVE_GIB + MIN_MACHINE_DISK_GIB
    {
        warnings.push(format!(
            "disk has {} GiB free, below the {} GiB required for a fresh managed-machine allocation; maintenance reuses the existing allocation",
            detected.system_disk_free_gib,
            WINDOWS_DISK_RESERVE_GIB + MIN_MACHINE_DISK_GIB
        ));
    }
    if supported && !cluster_member_supported {
        warnings.push(
            "selected profile is suitable for installation and laboratory runtime testing but is not certified for a complete Proxmox cluster member"
                .into(),
        );
    }

    HostProfile {
        schema_version: PROFILE_SCHEMA_VERSION,
        product_version: env!("CARGO_PKG_VERSION").into(),
        detected,
        selected: MachineProfile {
            capability: capability.into(),
            machine_cpus,
            machine_memory_mib,
            machine_disk_gib,
            windows_memory_reserve_mib,
        },
        supported,
        cluster_member_supported,
        warnings,
    }
}

#[cfg(windows)]
fn installed_product_exists() -> Result<bool, ProfileError> {
    let program_files = env::var_os("ProgramFiles")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| error("ProgramFiles is unavailable"))?;
    let service = program_files.join("Quetzalcoatl").join("gnx-service.exe");
    match fs::metadata(service) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(io_error) if io_error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(io_error) => Err(error(format!(
            "cannot inspect the installed Quetzalcoatl service: {io_error}"
        ))),
    }
}

#[cfg(windows)]
fn store(profile: &HostProfile) -> Result<(), ProfileError> {
    let root = crate::dependencies::staging::installer_root()
        .map_err(|stage_error| error(stage_error.message()))?;
    fs::create_dir_all(&root).map_err(|create_error| {
        error(format!(
            "cannot create installer directory for host profile: {create_error}"
        ))
    })?;
    let path = root.join("host-profile.json");
    let temporary = root.join("host-profile.json.next");
    let _ = fs::remove_file(&temporary);
    let bytes =
        serde_json::to_vec_pretty(profile).map_err(|_| error("cannot encode host-profile.json"))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|write_error| {
            error(format!(
                "cannot create host-profile.json.next: {write_error}"
            ))
        })?;
    file.write_all(&bytes)
        .and_then(|_| file.write_all(b"\n"))
        .and_then(|_| file.flush())
        .and_then(|_| file.sync_all())
        .map_err(|write_error| {
            error(format!(
                "cannot persist host-profile.json.next: {write_error}"
            ))
        })?;
    if path.exists() {
        fs::remove_file(&path).map_err(|remove_error| {
            error(format!("cannot replace host-profile.json: {remove_error}"))
        })?;
    }
    fs::rename(&temporary, &path).map_err(|rename_error| {
        error(format!("cannot activate host-profile.json: {rename_error}"))
    })?;
    let summary_path = root.join("host-profile.txt");
    fs::write(&summary_path, format!("{}\n", summary(profile))).map_err(|write_error| {
        error(format!(
            "cannot persist host-profile.txt at {}: {write_error}",
            summary_path.display()
        ))
    })?;
    Ok(())
}

fn round_down(value: u64, unit: u64) -> u64 {
    value / unit * unit
}

fn error(message: impl Into<String>) -> ProfileError {
    ProfileError {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(cpus: u64, memory_mib: u64, total_disk_gib: u64, free_disk_gib: u64) -> RawResources {
        RawResources {
            logical_cpus: cpus,
            total_memory_bytes: memory_mib * MIB,
            system_disk_total_bytes: total_disk_gib * GIB,
            system_disk_free_bytes: free_disk_gib * GIB,
        }
    }

    #[test]
    fn six_gib_host_gets_a_bounded_lab_profile() {
        let profile = calculate(raw(4, 5864, 100, 80), false);
        assert!(profile.supported);
        assert!(!profile.cluster_member_supported);
        assert_eq!(profile.selected.capability, "lab");
        assert_eq!(profile.selected.machine_cpus, 2);
        assert_eq!(profile.selected.machine_memory_mib, 2560);
        assert_eq!(profile.selected.machine_disk_gib, 60);
    }

    #[test]
    fn twelve_gib_host_gets_the_cluster_profile() {
        let profile = calculate(raw(8, 12 * 1024, 160, 130), false);
        assert!(profile.supported);
        assert!(profile.cluster_member_supported);
        assert_eq!(profile.selected.capability, "cluster-member");
        assert_eq!(profile.selected.machine_cpus, 6);
        assert_eq!(profile.selected.machine_memory_mib, 8192);
        assert_eq!(profile.selected.machine_disk_gib, 100);
    }

    #[test]
    fn small_host_is_install_only() {
        let profile = calculate(raw(2, 4091, 64, 50), false);
        assert!(!profile.supported);
        assert_eq!(profile.selected.capability, "install-only");
    }

    #[test]
    fn maintenance_reuses_the_existing_disk_allocation() {
        let profile = calculate(raw(12, 16 * 1024, 475, 59), true);
        assert!(profile.supported);
        assert_eq!(profile.selected.machine_disk_gib, MIN_MACHINE_DISK_GIB);
        assert!(
            profile
                .warnings
                .iter()
                .any(|warning| warning.contains("maintenance reuses the existing allocation"))
        );
    }
}
