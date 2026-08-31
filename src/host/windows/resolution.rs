use std::collections::BTreeSet;
use std::net::IpAddr;
use std::path::PathBuf;

use crate::config::{Config, ControllerUrl};
use crate::error::GnxError;
use crate::process::CommandSpec;

const BEGIN: &str = "# BEGIN Quetzalcoatl Next controller bootstrap";
const END: &str = "# END Quetzalcoatl Next controller bootstrap";

pub fn apply(config: &Config) -> Result<(), GnxError> {
    let controller = config.validate()?;
    if config.mesh.bootstrap_addresses.is_empty() {
        return Ok(());
    }
    let path = hosts_path();
    let current = std::fs::read_to_string(&path)
        .map_err(|error| GnxError::io("controller_hosts_read", error.to_string()))?;
    let updated = render(
        &current,
        &controller,
        config.mesh.expected_domain.as_deref(),
        &config.mesh.bootstrap_addresses,
    )?;
    if updated != current {
        std::fs::write(&path, updated.as_bytes())
            .map_err(|error| GnxError::io("controller_hosts_write", error.to_string()))?;
        flush_dns()?;
        crate::logs::event(
            "info",
            "mesh",
            "controller_hosts",
            format!(
                "Bootstrap DNS instalado para {} con {} dirección(es)",
                controller.host(),
                config.mesh.bootstrap_addresses.len()
            ),
        );
    }
    Ok(())
}

pub fn verify(config: &Config) -> Result<String, GnxError> {
    let controller = config.validate()?;
    if config.mesh.bootstrap_addresses.is_empty() {
        return Ok("Resolución delegada al DNS del sistema".to_string());
    }
    let current = std::fs::read_to_string(hosts_path())
        .map_err(|error| GnxError::io("controller_hosts_read", error.to_string()))?;
    let expected = managed_lines(
        &controller,
        config.mesh.expected_domain.as_deref(),
        &config.mesh.bootstrap_addresses,
    );
    let actual = managed_block(&current)?;
    if actual == Some(expected.as_str()) {
        Ok(format!(
            "{} fijado a {}",
            controller.host(),
            config
                .mesh
                .bootstrap_addresses
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ))
    } else {
        Err(GnxError::new(
            "MESH_BOOTSTRAP_NOT_APPLIED",
            "mesh",
            "controller_hosts_verify",
            format!(
                "El bloque administrado no coincide en {}.",
                hosts_path().display()
            ),
            "Ejecute gnx repair para reaplicar exclusivamente el bloque GNX.",
            true,
            16,
        ))
    }
}

pub fn remove_managed() -> Result<(), GnxError> {
    let path = hosts_path();
    let current = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(GnxError::io("controller_hosts_read", error.to_string())),
    };
    let (without, removed) = remove_block(&current)?;
    if removed {
        std::fs::write(&path, without.as_bytes())
            .map_err(|error| GnxError::io("controller_hosts_write", error.to_string()))?;
        flush_dns()?;
    }
    Ok(())
}

fn render(
    current: &str,
    controller: &ControllerUrl,
    expected_domain: Option<&str>,
    addresses: &[IpAddr],
) -> Result<String, GnxError> {
    let (mut without, _) = remove_block(current)?;
    let aliases = aliases(controller, expected_domain);
    reject_conflicts(&without, &aliases, addresses)?;
    if !without.ends_with(['\n', '\r']) {
        without.push_str("\r\n");
    }
    if !without.trim_end().is_empty() {
        without.push_str("\r\n");
    }
    without.push_str(&managed_lines(controller, expected_domain, addresses));
    Ok(without)
}

fn managed_lines(
    controller: &ControllerUrl,
    expected_domain: Option<&str>,
    addresses: &[IpAddr],
) -> String {
    let aliases = aliases(controller, expected_domain).join(" ");
    let mut block = format!("{BEGIN}\r\n");
    for address in addresses {
        block.push_str(&format!("{address}\t{aliases}\r\n"));
    }
    block.push_str(&format!("{END}\r\n"));
    block
}

fn aliases(controller: &ControllerUrl, expected_domain: Option<&str>) -> Vec<String> {
    let mut aliases = BTreeSet::from([controller.host().to_string()]);
    if expected_domain.is_some_and(|domain| domain.eq_ignore_ascii_case("node.gnx"))
        && matches!(
            controller.host(),
            "controlplane.node.gnx" | "headscale.node.gnx"
        )
    {
        aliases.insert("controlplane.node.gnx".to_string());
        aliases.insert("headscale.node.gnx".to_string());
    }
    aliases.into_iter().collect()
}

fn reject_conflicts(
    content: &str,
    aliases: &[String],
    addresses: &[IpAddr],
) -> Result<(), GnxError> {
    for line in content.lines() {
        let data = line.split('#').next().unwrap_or_default();
        let mut fields = data.split_whitespace();
        let Some(address) = fields.next().and_then(|value| value.parse::<IpAddr>().ok()) else {
            continue;
        };
        if fields.any(|host| aliases.iter().any(|alias| host.eq_ignore_ascii_case(alias)))
            && !addresses.contains(&address)
        {
            return Err(GnxError::new(
                "MESH_BOOTSTRAP_CONFLICT",
                "mesh",
                "controller_hosts",
                format!("Existe una entrada ajena para un alias GNX apuntando a {address}."),
                "Retire el conflicto manual o use esa dirección en mesh.bootstrap_addresses; GNX no sobrescribió hosts.",
                false,
                16,
            ));
        }
    }
    Ok(())
}

fn managed_block(content: &str) -> Result<Option<&str>, GnxError> {
    let Some(start) = content.find(BEGIN) else {
        return Ok(None);
    };
    let Some(relative_end) = content[start..].find(END) else {
        return Err(malformed_block());
    };
    let end = start + relative_end + END.len();
    let end = content[end..]
        .find('\n')
        .map_or(content.len(), |newline| end + newline + 1);
    Ok(Some(&content[start..end]))
}

fn remove_block(content: &str) -> Result<(String, bool), GnxError> {
    let Some(block) = managed_block(content)? else {
        return Ok((content.to_string(), false));
    };
    let start = block.as_ptr() as usize - content.as_ptr() as usize;
    let end = start + block.len();
    let mut result = String::with_capacity(content.len() - block.len());
    result.push_str(&content[..start]);
    result.push_str(&content[end..]);
    Ok((result, true))
}

fn malformed_block() -> GnxError {
    GnxError::new(
        "MESH_BOOTSTRAP_BLOCK_INVALID",
        "mesh",
        "controller_hosts",
        "El archivo hosts contiene un marcador GNX incompleto.",
        "Corrija únicamente el bloque marcado de Quetzalcoatl Next y ejecute gnx repair.",
        false,
        16,
    )
}

fn flush_dns() -> Result<(), GnxError> {
    CommandSpec::new(r"C:\Windows\System32\ipconfig.exe")
        .arg("/flushdns")
        .run_checked("controller_dns_flush")?;
    Ok(())
}

fn hosts_path() -> PathBuf {
    let root = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    root.join("System32")
        .join("drivers")
        .join("etc")
        .join("hosts")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controller() -> ControllerUrl {
        ControllerUrl::parse("https://controlplane.node.gnx").unwrap()
    }

    #[test]
    fn renderer_manages_both_sovereign_aliases_only_inside_markers() {
        let output = render(
            "127.0.0.1 localhost\r\n",
            &controller(),
            Some("node.gnx"),
            &["192.168.50.20".parse().unwrap()],
        )
        .unwrap();
        assert!(output.contains("192.168.50.20\tcontrolplane.node.gnx headscale.node.gnx"));
        assert!(output.starts_with("127.0.0.1 localhost"));
    }

    #[test]
    fn renderer_replaces_only_its_previous_block() {
        let first = render(
            "# user entry\r\n",
            &controller(),
            Some("node.gnx"),
            &["192.168.50.20".parse().unwrap()],
        )
        .unwrap();
        let second = render(
            &first,
            &controller(),
            Some("node.gnx"),
            &["100.64.0.10".parse().unwrap()],
        )
        .unwrap();
        assert_eq!(second.matches(BEGIN).count(), 1);
        assert!(second.contains("# user entry"));
        assert!(!second.contains("192.168.50.20"));
    }

    #[test]
    fn renderer_rejects_conflicting_user_mapping() {
        let error = render(
            "10.0.0.1 controlplane.node.gnx\r\n",
            &controller(),
            Some("node.gnx"),
            &["192.168.50.20".parse().unwrap()],
        )
        .unwrap_err();
        assert_eq!(error.code, "MESH_BOOTSTRAP_CONFLICT");
    }
}
