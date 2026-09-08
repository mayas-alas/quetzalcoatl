use crate::report::{Report, State};
pub fn run(mut r: Report) -> Report {
    if r.revision.is_some() && r.capabilities.len() == 3 && r.capabilities.iter().all(|c| c.healthy)
    {
        r.state = State::Ready;
        r.code = "OK".into();
        r.next_action = None;
        if r.routes.iter().any(|c| !c.healthy) {
            r.state = State::ActionRequired;
            r.code = "OPTIONAL_ROUTE_UNHEALTHY".into();
            r.next_action = Some(
                "Restore the reported external upstream; required capabilities remain healthy."
                    .into(),
            );
        }
    }
    r
}
