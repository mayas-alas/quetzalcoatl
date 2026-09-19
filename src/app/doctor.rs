use crate::{
    port::host::Host,
    report::{Report, State},
};
pub fn run(mut r: Report, h: &dyn Host) -> Report {
    match h.prerequisites() {
        Ok(()) => {
            r.code = "HOST_READY".into();
            r.state = State::Ready;
            r.next_action = None
        }
        Err(e) => {
            r.code = e;
            r.next_action = Some(
                "Use Linux with systemd, cgroup v2, Podman, and a verified GNX release.".into(),
            )
        }
    }
    r
}
