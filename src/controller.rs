use std::path::Path;

use crate::{Error, Result, config::Config};

const CA: &str = include_str!("../runtime/controller/ca.sh");
const CADDY: &str = include_str!("../runtime/controller/Caddyfile");
const UNIT: &str = include_str!("../runtime/controller/gnx-controller.container");

pub fn apply(config: &Config) -> Result<String> {
    crate::platform::root()?;
    let state = Path::new(&config.controller.state_dir);
    crate::platform::private_dir(state)?;
    let upstream_ca = Path::new(&config.compute.state_dir).join("upstream-ca.crt");
    if !upstream_ca.is_file() {
        return Err(Error::Operation("COMPUTE_REQUIRED"));
    }

    let ca = Path::new(&config.host.runtime_dir).join("controller/ca.sh");
    crate::platform::install(&ca, CA, 0o755)?;
    if config.controller.autonomous_ca {
        let mut command = vec![
            ca.to_str().ok_or(Error::ConfigInvalid)?,
            &config.controller.state_dir,
        ];
        command.extend(
            config
                .access
                .services
                .iter()
                .map(|service| service.alias.as_str()),
        );
        crate::platform::run(&command, None, "CA_GENERATION")?;
    }

    let private_sites = if config.controller.autonomous_ca {
        config
            .access
            .services
            .iter()
            .map(|service| {
                format!(
                    "https://{} {{\n    tls /etc/gnx/tls/server.crt /etc/gnx/tls/server.key\n    import compute\n}}",
                    service.alias
                )
            })
            .chain(std::iter::once(
                "http://pki.gnx {\n    root * /srv/gnx\n    file_server\n}".into(),
            ))
            .collect::<Vec<String>>()
            .join("\n\n")
    } else {
        String::new()
    };
    let caddy = CADDY.replace("@PRIVATE_SITES@", &private_sites);
    crate::platform::install(&state.join("Caddyfile"), &caddy, 0o600)?;
    crate::platform::install(
        Path::new("/etc/containers/systemd/gnx-controller.container"),
        UNIT,
        0o644,
    )?;
    systemctl(&["daemon-reload"], "CONTROLLER_INSTALL")?;
    systemctl(
        &["enable", "--now", "gnx-controller.service"],
        "CONTROLLER_SERVICE",
    )?;
    status(config)
}

pub fn status(config: &Config) -> Result<String> {
    systemctl(
        &["is-active", "--quiet", "gnx-controller.service"],
        "CONTROLLER_SERVICE",
    )?;
    crate::platform::run(
        &[
            "curl",
            "--fail",
            "--silent",
            "--max-time",
            "15",
            "--output",
            "/dev/null",
            &config.compute.verify_endpoint,
        ],
        None,
        "CONTROLLER_ROUTE",
    )?;
    if config.controller.autonomous_ca {
        let alias = &config.access.services[0].alias;
        let resolve = format!("{alias}:443:127.0.0.1");
        let url = format!("https://{alias}/");
        let root = Path::new(&config.controller.state_dir).join("public/root.crt");
        crate::platform::run(
            &[
                "curl",
                "--fail",
                "--silent",
                "--max-time",
                "15",
                "--output",
                "/dev/null",
                "--cacert",
                root.to_str().ok_or(Error::ConfigInvalid)?,
                "--resolve",
                &resolve,
                &url,
            ],
            None,
            "AUTONOMOUS_CA_ROUTE",
        )?;
    }
    Ok(format!(
        "controller\nAutonomous CA: {}",
        if config.controller.autonomous_ca {
            "enabled"
        } else {
            "disabled"
        }
    ))
}

fn systemctl(args: &[&str], operation: &'static str) -> Result<()> {
    let mut command = vec!["systemctl"];
    command.extend_from_slice(args);
    crate::platform::run(&command, None, operation).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_do_not_embed_private_keys() {
        assert!(!CA.contains("BEGIN PRIVATE KEY"));
        assert!(!CADDY.contains("BEGIN PRIVATE KEY"));
        assert!(CADDY.contains("@PRIVATE_SITES@"));
    }
}
