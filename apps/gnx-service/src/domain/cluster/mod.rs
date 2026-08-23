use std::sync::{Arc, RwLock};

use gnx_contracts::StatusResponse;

use crate::application::status::set_member_stage_status;
use crate::domain::errors::GateError;
use crate::domain::topology::store_persisted_state;

mod authorize_member;
mod confirm_membership;
mod prepare_member;
mod verify_member;

pub(crate) use authorize_member::authorize_member;
pub(crate) use confirm_membership::confirm_membership;
pub(crate) use prepare_member::prepare_member;
pub(crate) use verify_member::verify_member;

pub(crate) fn persist_member_stage(
    status: &Arc<RwLock<StatusResponse>>,
    member: &mut crate::infrastructure::state::PersistedState,
    stage: &str,
) -> Result<(), GateError> {
    member.stage = stage.into();
    store_persisted_state(member)?;
    set_member_stage_status(status, &member.controller.hostname, stage);
    Ok(())
}
