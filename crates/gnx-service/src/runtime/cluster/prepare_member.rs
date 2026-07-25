use super::super::*;

pub(in crate::runtime) fn prepare_member(
    status: &Arc<RwLock<StatusResponse>>,
    member: &mut crate::state::PersistedState,
) -> Result<(), GateError> {
    super::persist_member_stage(status, member, "MEMBER_PREPARING")?;
    if !member.role.is_member()
        || member.controller.id == member.self_id
        || member.controller.ip == member.self_ip
        || !member.controller.hostname.starts_with("gnx-controller-")
        || !valid_discovered_hostname(&member.controller.hostname)
        || !valid_discovered_hostname(persisted_local_hostname(member)?)
    {
        return Err(GateError::new(
            "MEMBER_PREPARE_FAILED",
            Component::Tailscale,
            "persisted controller/member identity is not eligible for cluster join",
        ));
    }
    Ok(())
}
