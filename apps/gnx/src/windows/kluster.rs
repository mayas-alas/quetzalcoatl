use super::{
    deploy::command,
    identity::{Identity, Ownership},
    security,
};
use crate::{
    deployment::{Bundle, File},
    protocol::{DISTRO, invalid},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    time::Duration,
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Input {
    version: u16,
    owner: Ownership,
    bundle: Bundle,
}

pub(super) fn validate_owner(sid: &str) -> io::Result<()> {
    let saved: Input =
        serde_json::from_slice(&security::read_private("kluster-input.json", 128 * 1024)?)
            .map_err(|_| invalid("entrada kluster inválida"))?;
    if saved.version != 1 || saved.owner.sid != sid {
        return Err(invalid("entrada kluster ajena"));
    }
    saved.bundle.validate()
}

pub(super) fn bundle() -> io::Result<Bundle> {
    security::require_system()?;
    let owner = Ownership::read()?;
    let files = [
        (
            "registry.container",
            include_str!("../../../../stacks/kluster/quadlets/registry.container"),
        ),
        (
            "config/kluster-assets.json",
            include_str!("../../../../stacks/kluster/assets.json"),
        ),
        (
            "config/kluster-install.run",
            include_str!("../../../../stacks/kluster/scripts/install.run"),
        ),
        (
            "config/kluster-health.run",
            include_str!("../../../../stacks/kluster/scripts/health.run"),
        ),
        (
            "config/kluster-reconcile-observe.run",
            include_str!("../../../../stacks/kluster/scripts/reconcile-observe.run"),
        ),
        (
            "config/kluster-values.yaml",
            include_str!("../../../../stacks/kluster/kubeshark-values.yaml"),
        ),
        (
            "config/kluster-service-health.yaml",
            include_str!("../../../../stacks/kluster/service-health.yaml"),
        ),
        (
            "config/kluster-kernel.run",
            include_str!("../../../../stacks/kluster/scripts/prepare-kernel.run"),
        ),
        (
            "config/kluster-lxc.run",
            include_str!("../../../../stacks/kluster/scripts/prepare-lxc.run"),
        ),
    ]
    .into_iter()
    .map(|(path, contents)| File {
        path: path.into(),
        contents: contents.into(),
    })
    .collect();
    let bundle = Bundle { files };
    bundle.validate()?;
    match security::read_private("kluster-input.json", 128 * 1024) {
        Ok(bytes) => {
            let saved: Input =
                serde_json::from_slice(&bytes).map_err(|_| invalid("entrada kluster inválida"))?;
            if saved.version != 1
                || saved.owner.sid != owner.sid
                || saved.owner.distribution_id != owner.distribution_id
                || saved.bundle.digest()? != bundle.digest()?
            {
                return Err(invalid(
                    "perfil kluster cambiado o de otro nodo; requiere migración explícita",
                ));
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let _directory = security::private_directory()?;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(security::state_dir()?.join("kluster-input.json"))?;
            file.write_all(
                &serde_json::to_vec(&Input {
                    version: 1,
                    owner,
                    bundle: bundle.clone(),
                })
                .map_err(io::Error::other)?,
            )?;
            file.sync_all()?;
        }
        Err(e) => return Err(e),
    }
    Ok(bundle)
}

pub(super) fn prepare(identity: &Identity) -> io::Result<()> {
    // Static code under the verified dedicated identity; no client paths or scripts.
    let output = identity.run_payload(
        &["--distribution", DISTRO, "--exec", "/usr/bin/bash", "-c",
          "set -euo pipefail; umask 077; install -d -m 0755 /etc/gnx-core; test ! -L /etc/gnx-core/kluster-kernel.run; cat > /etc/gnx-core/kluster-kernel.run; chmod 0700 /etc/gnx-core/kluster-kernel.run"],
        include_bytes!("../../../../stacks/kluster/scripts/prepare-kernel.run"),
        Duration::from_secs(15))?;
    if output.code != 0 {
        return Err(io::Error::other(
            "transferencia de preparación WSL no confirmada",
        ));
    }
    let output = identity.run(
        &[
            "--distribution",
            DISTRO,
            "--exec",
            "/usr/bin/bash",
            "/etc/gnx-core/kluster-kernel.run",
            "enable",
        ],
        Duration::from_secs(60),
    )?;
    if output.code != 0 {
        return Err(io::Error::other(format!(
            "preparación fija del kernel WSL no confirmada (exit {}): {}",
            output.code,
            super::deploy::preparation_detail(&output)
        )));
    }
    Ok(())
}

pub(super) fn configure_lxc(
    identity: &Identity,
    id: &str,
    description: &str,
    complete: bool,
) -> io::Result<()> {
    command(
        identity,
        &[
            "/usr/bin/perl",
            "-",
            id,
            description,
            if complete { "check" } else { "apply" },
        ],
        Some(include_bytes!(
            "../../../../stacks/kluster/scripts/prepare-lxc.run"
        )),
        30,
    )?;
    Ok(())
}

pub(super) fn install(identity: &Identity, id: &str) -> io::Result<()> {
    command(
        identity,
        &[
            "pct",
            "exec",
            id,
            "--",
            "/usr/bin/bash",
            "/etc/gnx-app/config/kluster-install.run",
        ],
        None,
        1500,
    )?;
    health(identity, id)
}

pub(super) fn health(identity: &Identity, id: &str) -> io::Result<()> {
    command(
        identity,
        &[
            "pct",
            "exec",
            id,
            "--",
            "/usr/bin/bash",
            "/etc/gnx-app/config/kluster-health.run",
        ],
        None,
        300,
    )?;
    Ok(())
}
