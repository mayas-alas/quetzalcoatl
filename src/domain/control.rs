use std::collections::HashSet;
pub fn validate(routes: &[crate::config::Route]) -> Result<(), String> {
    if routes.len() > 64 {
        return Err("TOO_MANY_ROUTES".into());
    }
    let mut names = HashSet::from(["compute.gnx", "proxmox.gnx", "app.gnx", "ns.gnx"]);
    for r in routes {
        let name = r.hostname.strip_suffix(".gnx").ok_or("INVALID_HOSTNAME")?;
        if !super::node::valid_name(name) || !names.insert(&r.hostname) {
            return Err("DUPLICATE_OR_INVALID_HOSTNAME".into());
        }
        // Only an origin is operator intent. Reject parser normalization and config injection.
        if r.upstream.len() > 2048
            || r.upstream
                .bytes()
                .any(|b| b <= 32 || b >= 127 || b == b'\\')
        {
            return Err("INVALID_UPSTREAM".into());
        }
        let u = url::Url::parse(&r.upstream).map_err(|_| "INVALID_UPSTREAM")?;
        if !matches!(u.scheme(), "http" | "https")
            || u.host_str().is_none()
            || !u.username().is_empty()
            || u.password().is_some()
            || u.query().is_some()
            || u.fragment().is_some()
            || u.path() != "/"
            || r.upstream.contains(['@', '{', '}', '"', '\'', '%'])
        {
            return Err("INVALID_UPSTREAM".into());
        }
    }
    Ok(())
}
