use crate::{
    config::Config,
    report::{Report, State},
};
pub fn run(mut r: Report, c: &Config) -> Report {
    if r.revision.as_deref() != Some(&c.revision()) {
        r.changes
            .push("Reconcile validated intent after prerequisites pass".into());
    }
    r.code = "PLAN_OBSERVED".into();
    r.state = State::Ready;
    r.next_action = None;
    r
}
