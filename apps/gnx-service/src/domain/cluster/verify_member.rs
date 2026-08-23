use std::path::Path;
use std::sync::{Arc, RwLock};

use gnx_contracts::StatusResponse;

use crate::domain::errors::GateError;
use crate::domain::topology::persisted_local_hostname;
use crate::infrastructure::proxmox::verify_pve_identity;

pub(crate) fn verify_member(
    status: &Arc<RwLock<StatusResponse>>,
    podman: &Path,
    member: &mut crate::infrastructure::state::PersistedState,
) -> Result<(), GateError> {
    super::persist_member_stage(status, member, "MEMBER_VERIFYING")?;
    verify_pve_identity(podman, member.self_ip, persisted_local_hostname(member)?)
}
