use super::*;

mod authorize_member;
mod confirm_membership;
mod prepare_member;
mod verify_member;

pub(super) use authorize_member::authorize_member;
pub(super) use confirm_membership::confirm_membership;
pub(super) use prepare_member::prepare_member;
pub(super) use verify_member::verify_member;

pub(super) fn persist_member_stage(
    status: &Arc<RwLock<StatusResponse>>,
    member: &mut crate::state::PersistedState,
    stage: &str,
) -> Result<(), GateError> {
    member.stage = stage.into();
    store_persisted_state(member)?;
    set_member_stage_status(status, &member.controller.hostname, stage);
    Ok(())
}
