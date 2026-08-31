use std::path::PathBuf;

use crate::error::GnxError;
use crate::report::StatusReport;

pub fn run(config_path: PathBuf, json: bool) -> Result<(), GnxError> {
    let report = StatusReport::collect(&config_path)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("StatusReport siempre serializa")
        );
    } else {
        println!("GNX {} — {}", report.product_version, report.stage);
        println!("Host: {} / {}", report.host.os, report.host.architecture);
        println!("Config: {}", report.config_path);
        println!(
            "Controller: {}",
            report.controller_url.as_deref().unwrap_or("no configurado")
        );
        println!("Machine: {}", report.machine);
        println!("Mesh: {}", report.mesh);
        println!("Docktail: {}", report.docktail);
        println!("Proxmox: {}", report.proxmox);
        println!("Infra: {}", report.infra);
        if let Some(error) = report.last_error.as_deref() {
            println!("Último error: {error}");
        }
        println!("Logs: gnx logs --tail 50");
        println!("{}", report.note);
    }
    Ok(())
}
