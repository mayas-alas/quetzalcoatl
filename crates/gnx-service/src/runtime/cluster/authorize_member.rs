use super::super::*;

pub(in crate::runtime) fn authorize_member(
    status: &Arc<RwLock<StatusResponse>>,
    member: &mut crate::state::PersistedState,
    pve_root_password: &str,
) -> Result<(), GateError> {
    super::persist_member_stage(status, member, "MEMBER_AUTHORIZING")?;
    if pve_root_password.is_empty() {
        return Err(GateError::new(
            "MEMBER_AUTHORIZATION_FAILED",
            Component::Proxmox,
            "protected PVE credential is empty",
        ));
    }
    if member.tailnet.is_empty() || member.controller.hostname == persisted_local_hostname(member)?
    {
        return Err(GateError::new(
            "MEMBER_AUTHORIZATION_FAILED",
            Component::Tailscale,
            "member authorization context is inconsistent",
        ));
    }
    Ok(())
}
