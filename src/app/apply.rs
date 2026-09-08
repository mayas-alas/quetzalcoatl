use crate::{
    config::Config,
    domain::secret::{Secret, SecretKind},
    port::{host::Host, runtime::Runtime, state::StateStore},
    report::{Report, State},
};
pub fn run(out: Report, c: &Config, h: &dyn Host, r: &dyn Runtime, s: &dyn StateStore) -> Report {
    run_secret(out, c, h, r, s, None)
}
pub fn run_secret(
    mut out: Report,
    c: &Config,
    h: &dyn Host,
    r: &dyn Runtime,
    s: &dyn StateStore,
    secret: Option<&Secret>,
) -> Report {
    let mut mutated = false;
    let result = (|| {
        h.prerequisites()?;
        let _guard = s.acquire()?;
        let previous = s.previous()?;
        if s.interrupted()? {
            r.restore(previous.as_ref())?;
            s.abort()?;
        }
        // Observe again under the lock to prevent a stale decision across concurrent apply.
        let current = s.current()?;
        let caps = r.observe();
        if current.as_deref() == Some(&c.revision())
            && caps.len() == 3
            && caps.iter().all(|x| x.healthy)
        {
            return Ok(());
        }
        s.stage(c)?;
        s.phase("reconciling")?;
        mutated = true;
        let candidate = (|| {
            r.reconcile_secret(c, secret)?;
            let caps = r.observe();
            if caps.len() != 3 || caps.iter().any(|x| !x.healthy) {
                return Err("HEALTH_FAILED".into());
            }
            s.phase("verified")?;
            s.promote()
        })();
        if let Err(e) = candidate {
            if r.restore(previous.as_ref()).is_err() {
                return Err("ROLLBACK_FAILED".into());
            }
            s.abort()?;
            return Err(e);
        }
        Ok::<(), String>(())
    })();
    out.capabilities = r.observe();
    out.routes = r.optional_routes();
    out.public_root = r.public_root();
    match result {
        Ok(()) => {
            out.revision = Some(c.revision());
            super::status::run(out)
        }
        Err(e) => {
            out.state = if e.ends_with("_REQUIRED") || e == "APPLY_BUSY" {
                State::ActionRequired
            } else {
                State::Failed
            };
            out.secret_kind = match e.as_str() {
                "COMPUTE_PASSWORD_REQUIRED" => Some(SecretKind::ComputePassword),
                "ACCESS_ENROLLMENT_REQUIRED" => Some(SecretKind::AccessEnrollment),
                _ => None,
            };
            out.next_action=Some(match e.as_str(){"COMPUTE_PASSWORD_REQUIRED"=>"Enter a new Compute password using the hidden prompt (at least 16 printable characters).","ACCESS_ENROLLMENT_REQUIRED"=>"Provide a Tailscale enrollment key using the hidden prompt, then configure split DNS for .gnx.","APPLY_BUSY"=>"Another apply owns this instance. Wait and retry.","ROLLBACK_FAILED"=>"Recovery is incomplete; preserve state and run apply again.",_ if mutated=>"Candidate was not promoted. Review doctor and capability diagnostics, then retry.",_=>"Resolve the reported prerequisite and retry."}.into());
            out.code = e;
            out
        }
    }
}
