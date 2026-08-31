use std::time::Duration;

use crate::config::ControllerUrl;
use crate::error::GnxError;
use ureq::tls::{RootCerts, TlsConfig};

pub fn verify_controller(controller: &ControllerUrl) -> Result<u16, GnxError> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .max_redirects(0)
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into();
    let health_url = health_url(controller);
    let response = agent.get(&health_url).call().map_err(|error| {
        GnxError::new(
            "MESH_CONTROLLER_UNAVAILABLE",
            "mesh",
            "controller_preflight",
            format!("{health_url}: {error}"),
            "Compruebe bootstrap DNS, confianza TLS y disponibilidad de /health en Headscale.",
            true,
            16,
        )
    })?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(GnxError::new(
            "MESH_CONTROLLER_UNHEALTHY",
            "mesh",
            "controller_health",
            format!("{health_url} respondió HTTP {status}."),
            "Corrija el reverse proxy o Headscale hasta que /health responda 2xx.",
            true,
            16,
        ));
    }
    Ok(status)
}

fn health_url(controller: &ControllerUrl) -> String {
    format!("{}/health", controller.canonical())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_check_targets_headscale_endpoint() {
        let controller = ControllerUrl::parse("https://controlplane.node.gnx").unwrap();
        assert_eq!(
            health_url(&controller),
            "https://controlplane.node.gnx/health"
        );
    }
}
