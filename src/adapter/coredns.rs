use crate::config::Config;
pub fn render(c: &Config) -> Result<String, String> {
    render_at(
        c,
        &c.network
            .identity_ip
            .ok_or("ENROLLMENT_REQUIRED")?
            .to_string(),
    )
}
pub fn render_at(_c: &Config, ip: &str) -> Result<String, String> {
    let _: std::net::Ipv4Addr = ip.parse().map_err(|_| "INVALID_PRIVATE_IP")?;
    Ok(format!("gnx:53 {{\n bind {ip}\n file /gnx.zone gnx\n}}\n.:53 {{\n bind {ip}\n template IN ANY {{\n rcode REFUSED\n }}\n}}\n"))
}
pub fn zone(c: &Config, ip: &str) -> String {
    let revision = c.revision();
    let serial = u32::from_str_radix(&revision[..8], 16).unwrap();
    let mut s=format!("$ORIGIN gnx.\n$TTL 60\n@ IN SOA ns.gnx. hostmaster.gnx. {serial} 60 60 3600 60\n@ IN NS ns.gnx.\nns IN A {ip}\ncompute IN A {ip}\n");
    s.push_str(&format!("proxmox IN A {ip}\n"));
    for r in &c.routes {
        s.push_str(&format!(
            "{} IN A {ip}\n",
            r.hostname.trim_end_matches(".gnx")
        ));
    }
    s
}
pub fn probe(ip: &str, c: &Config) -> Result<(), String> {
    let b = super::process::checked(
        "podman",
        &[
            "inspect",
            "--format",
            "{{.State.Pid}}",
            &format!("gnx-{}-access", c.instance),
        ],
        None,
        10,
    )?;
    let pid = String::from_utf8_lossy(&b)
        .trim()
        .parse::<u32>()
        .map_err(|_| "ACCESS_NAMESPACE_MISSING")?
        .to_string();
    if pid == "0" {
        return Err("ACCESS_NAMESPACE_MISSING".into());
    }
    for tcp in [false, true] {
        for (name, expected) in [
            ("compute.gnx", "NOERROR"),
            ("gnx-undeclared-probe.gnx", "NXDOMAIN"),
            ("example.org", "REFUSED"),
        ] {
            let target = format!("@{ip}");
            let mut a = vec![
                "--target", &pid, "--net", "--", "dig", &target, name, "A", "+time=2", "+tries=1",
            ];
            if tcp {
                a.push("+tcp")
            };
            let b =
                super::process::checked("nsenter", &a, None, 5).map_err(|_| "DNS_PROBE_FAILED")?;
            let text = String::from_utf8_lossy(&b);
            if !text.contains(&format!("status: {expected}"))
                || (expected != "REFUSED" && !text.contains(" aa"))
                || (expected == "NOERROR" && !text.contains(ip))
            {
                return Err("DNS_AUTHORITY_FAILED".into());
            }
        }
    }
    Ok(())
}
