use std::path::Path;
use std::sync::{Arc, RwLock};

use gnx_contracts::StatusResponse;

use crate::domain::errors::GateError;
use crate::domain::lifecycle::Component;
use crate::domain::topology::persisted_local_hostname;
use crate::infrastructure::remote::{RuntimeOperation, bounded_text, runtime_agent};

pub(crate) fn confirm_membership(
    status: &Arc<RwLock<StatusResponse>>,
    podman: &Path,
    member: &mut crate::infrastructure::state::PersistedState,
) -> Result<(), GateError> {
    super::persist_member_stage(status, member, "MEMBER_CONFIRMING")?;
    let member_hostname = persisted_local_hostname(member)?;
    let input = format!(
        "{}\n{}\n{}\n{}\n",
        member.controller.ip, member.controller.hostname, member.self_ip, member_hostname
    );
    let output = runtime_agent(
        podman,
        RuntimeOperation::PveClusterConfirmMember,
        input.as_bytes(),
    )
    .map_err(|error| error.with_code("PVE_MEMBERSHIP_CONFIRM_FAILED", Component::Proxmox))?;
    if String::from_utf8_lossy(&output.stdout).trim() == "PVE_MEMBERSHIP=confirmed" {
        Ok(())
    } else {
        Err(GateError::new(
            "PVE_MEMBERSHIP_CONFIRM_FAILED",
            Component::Proxmox,
            format!(
                "cluster did not confirm controller and member visibility; output: {}",
                bounded_text(&output.stdout)
            ),
        ))
    }
}
