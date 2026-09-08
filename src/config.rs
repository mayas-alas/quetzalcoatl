use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: u32,
    pub instance: String,
    pub node: String,
    pub network: Network,
    #[serde(default)]
    pub routes: Vec<Route>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Network {
    pub subnet: String,
    pub identity_ip: Option<std::net::Ipv4Addr>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub hostname: String,
    pub upstream: String,
}
impl Config {
    pub fn parse(s: &str) -> Result<Self, String> {
        let c: Self = toml::from_str(s).map_err(|_| "INVALID_CONFIG".to_string())?;
        if c.schema != 1 {
            return Err("UNSUPPORTED_SCHEMA".into());
        }
        if !crate::domain::node::valid_name(&c.instance)
            || !crate::domain::node::valid_name(&c.node)
        {
            return Err("INVALID_IDENTITY".into());
        }
        crate::domain::access::validate(&c.network)?;
        crate::domain::control::validate(&c.routes)?;
        Ok(c)
    }
    pub fn revision(&self) -> String {
        use sha2::{Digest, Sha256};
        format!(
            "{:x}",
            Sha256::digest(
                [
                    serde_json::to_vec(self).unwrap(),
                    include_bytes!("../runtime/release.toml").to_vec()
                ]
                .concat()
            )
        )
    }
}
