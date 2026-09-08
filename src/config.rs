use crate::{Result, report::Failure};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, net::Ipv4Addr, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: u32,
    pub instance: String,
    pub node: String,
    pub state_dir: String,
    pub units_dir: String,
    pub network: Network,
    pub images: Images,
    #[serde(default)]
    pub routes: Vec<Route>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Network {
    pub subnet: String,
    pub access_ip: Ipv4Addr,
    pub compute_ip: Ipv4Addr,
    pub tailnet_ip: Option<Ipv4Addr>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Images {
    pub access: String,
    pub dns: String,
    pub control: String,
    pub compute: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub hostname: String,
    pub upstream: String,
}

pub fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && value.as_bytes()[0].is_ascii_lowercase()
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn safe_path(value: &str) -> bool {
    value.starts_with('/')
        && value.len() > 1
        && !value.ends_with('/')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b))
        && value
            .split('/')
            .skip(1)
            .all(|c| !c.is_empty() && c != "." && c != "..")
}

impl Config {
    pub fn read(path: &Path) -> Result<Self> {
        Self::parse(&std::fs::read_to_string(path)?)
    }
    pub fn parse(text: &str) -> Result<Self> {
        // TOML errors can quote the user's input, which may contain a misplaced secret.
        let config: Self = toml::from_str(text).map_err(|_| {
            Failure::new(
                "CONFIG_PARSE",
                "Invalid TOML or unknown field; consult config/gnx.example.toml",
            )
        })?;
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> Result<()> {
        let invalid = |why| Failure::new("CONFIG_INVALID", why);
        if self.schema != 1 || !identifier(&self.instance) || !identifier(&self.node) {
            return Err(invalid(
                "schema must be 1; instance and node must be lowercase identifiers",
            ));
        }
        if !safe_path(&self.state_dir)
            || !safe_path(&self.units_dir)
            || self.state_dir == self.units_dir
            || Path::new(&self.state_dir).starts_with(&self.units_dir)
            || Path::new(&self.units_dir).starts_with(&self.state_dir)
        {
            return Err(invalid(
                "state_dir and units_dir must be separate, absolute Linux paths without traversal",
            ));
        }
        let subnet: ipnet::Ipv4Net = self
            .network
            .subnet
            .parse()
            .map_err(|_| invalid("Invalid IPv4 subnet"))?;
        if !(16..=29).contains(&subnet.prefix_len())
            || !subnet.network().is_private()
            || !subnet.broadcast().is_private()
        {
            return Err(invalid(
                "Backplane requires a private IPv4 /16 through /29 subnet",
            ));
        }
        for ip in [self.network.access_ip, self.network.compute_ip] {
            if !subnet.contains(&ip)
                || ip == subnet.network()
                || ip == subnet.broadcast()
                || u32::from(ip) == u32::from(subnet.network()) + 1
            {
                return Err(invalid(
                    "Service IPs must be distinct subnet hosts, excluding network, gateway and broadcast",
                ));
            }
        }
        if self.network.access_ip == self.network.compute_ip {
            return Err(invalid("Service IP collision"));
        }
        if self
            .network
            .tailnet_ip
            .is_some_and(|ip| u32::from(ip) >> 22 != u32::from(Ipv4Addr::new(100, 64, 0, 0)) >> 22)
        {
            return Err(invalid("tailnet_ip must belong to 100.64.0.0/10"));
        }
        for (image, repository) in [
            (&self.images.access, "docker.io/tailscale/tailscale"),
            (&self.images.dns, "docker.io/coredns/coredns"),
            (&self.images.control, "docker.io/library/caddy"),
            (&self.images.compute, "docker.io/dockurr/proxmox"),
        ] {
            let Some(digest) = image.strip_prefix(&format!("{repository}@sha256:")) else {
                return Err(invalid(
                    "Images must use the selected repositories and immutable sha256 digests",
                ));
            };
            if digest.len() != 64
                || !digest
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            {
                return Err(invalid("Invalid image digest"));
            }
        }
        let mut names: HashSet<String> = ["proxmox.gnx".into(), "ns.gnx".into()].into();
        for route in &self.routes {
            let label = route
                .hostname
                .strip_suffix(".gnx")
                .ok_or(invalid("Routes must be single-label .gnx names"))?;
            if !identifier(label) || !names.insert(route.hostname.clone()) {
                return Err(invalid("Duplicate or invalid route hostname"));
            }
            let url =
                url::Url::parse(&route.upstream).map_err(|_| invalid("Invalid upstream URL"))?;
            if !matches!(url.scheme(), "http" | "https")
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || url.path() != "/"
                || route
                    .upstream
                    .bytes()
                    .any(|b| b.is_ascii_whitespace() || b"{}\\\"%".contains(&b))
            {
                return Err(invalid(
                    "Upstreams must be HTTP(S) origins without credentials, paths, query or template syntax",
                ));
            }
            let Some(host) = url.host_str() else {
                return Err(invalid("Upstream host is required"));
            };
            if host.ends_with(".gnx")
                || host == "gnx"
                || host == "localhost"
                || host
                    .trim_matches(['[', ']'])
                    .parse::<std::net::Ipv6Addr>()
                    .is_ok_and(|ip| {
                        ip.is_loopback() || ip.is_unspecified() || ip.to_ipv4_mapped().is_some()
                    })
                || host.parse::<Ipv4Addr>().is_ok_and(|ip| {
                    ip.is_loopback()
                        || ip.is_unspecified()
                        || Some(ip) == self.network.tailnet_ip
                        || subnet.contains(&ip)
                })
            {
                return Err(invalid(
                    "Upstream must not route back into GNX or local control endpoints",
                ));
            }
        }
        Ok(())
    }
    pub fn revision(&self) -> String {
        use sha2::{Digest, Sha256};
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).expect("serializable config"))
        )
    }
    pub fn unit(&self, name: &str) -> String {
        format!("{}-{name}", self.instance)
    }
}
