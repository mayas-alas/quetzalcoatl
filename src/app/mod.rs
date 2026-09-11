pub mod apply;
pub mod doctor;
pub mod plan;
pub mod status;
use crate::{
    config::Config,
    port::{host::Host, runtime::Runtime, state::StateStore},
    report::{Report, State},
};
pub fn execute(op: &str, c: &Config, h: &dyn Host, r: &dyn Runtime, s: &dyn StateStore) -> Report {
    let mut out = Report::new(
        op,
        State::ActionRequired,
        "RUNTIME_NOT_READY",
        Some("Install the pinned runtime and complete enrollment; run doctor."),
    );
    out.revision = match s.current() {
        Ok(v) => v,
        Err(e) => {
            return Report::new(
                op,
                State::Failed,
                &e,
                Some("Repair protected state before retrying."),
            )
        }
    };
    out.capabilities = r.observe();
    out.routes = r.optional_routes();
    out.public_root = r.public_root();
    out.access_ip = r.access_ip();
    if op == "doctor" {
        return doctor::run(out, h);
    }
    if op == "plan" {
        return plan::run(out, c);
    }
    if op == "apply" {
        return apply::run(out, c, h, r, s);
    }
    status::run(out)
}
