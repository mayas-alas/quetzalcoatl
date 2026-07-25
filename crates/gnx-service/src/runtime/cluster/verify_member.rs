use super::super::*;

pub(in crate::runtime) fn verify_member(
    status: &Arc<RwLock<StatusResponse>>,
    podman: &Path,
    member: &mut crate::state::PersistedState,
) -> Result<(), GateError> {
    super::persist_member_stage(status, member, "MEMBER_VERIFYING")?;
    verify_pve_identity(podman, member.self_ip, persisted_local_hostname(member)?)
}
