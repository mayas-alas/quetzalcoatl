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
    let response = agent.get(controller.canonical()).call().map_err(|error| {
        GnxError::new(
            "MESH_CONTROLLER_UNAVAILABLE",
            "mesh",
            "controller_preflight",
            format!("{}: {error}", controller.canonical()),
            "Compruebe DNS, TLS y disponibilidad del endpoint configurado.",
            true,
            16,
        )
    })?;
    Ok(response.status().as_u16())
}
