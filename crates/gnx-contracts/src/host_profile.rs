use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HostProfile {
    pub schema_version: u8,
    pub product_version: String,
    pub detected: DetectedResources,
    pub selected: MachineProfile,
    pub supported: bool,
    pub cluster_member_supported: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DetectedResources {
    pub logical_cpus: u64,
    pub total_memory_mib: u64,
    pub system_disk_total_gib: u64,
    pub system_disk_free_gib: u64,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MachineProfile {
    pub capability: String,
    pub machine_cpus: u64,
    pub machine_memory_mib: u64,
    pub machine_disk_gib: u64,
    pub windows_memory_reserve_mib: u64,
}
