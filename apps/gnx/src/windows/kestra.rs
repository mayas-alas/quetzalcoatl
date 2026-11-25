use super::{identity::Ownership, security};
use crate::{
    deployment::{Bundle, File},
    protocol::invalid,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
};

const INPUT: &str = "kestra-input.json";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Input {
    version: u16,
    owner: Ownership,
    bundle: Bundle,
}

fn saved() -> io::Result<Input> {
    let bytes = zeroize::Zeroizing::new(security::read_private(INPUT, 128 * 1024)?);
    let saved: Input = serde_json::from_slice(&bytes)
        .map_err(|_| invalid("entrada Kestra inválida; no se regenera"))?;
    let paths = [
        "kestra.network",
        "kestra-postgres.container",
        "kestra.container",
        "config/kestra.yml",
        "config/kestra-postgres.env",
    ];
    if saved.version != 1
        || saved.bundle.files.len() != paths.len()
        || paths
            .iter()
            .any(|path| !saved.bundle.files.iter().any(|f| f.path == *path))
    {
        return Err(invalid("entrada Kestra incompatible"));
    }
    saved.bundle.validate()?;
    Ok(saved)
}

pub(super) fn validate_owner(sid: &str) -> io::Result<()> {
    if saved()?.owner.sid != sid {
        return Err(invalid("entrada Kestra ajena"));
    }
    Ok(())
}

pub(super) fn bundle() -> io::Result<Bundle> {
    security::require_system()?;
    let owner = Ownership::read()?;
    match saved() {
        Ok(saved) => {
            if saved.owner.sid != owner.sid || saved.owner.distribution_id != owner.distribution_id
            {
                return Err(invalid("Kestra pertenece a otro nodo; no se reutiliza"));
            }
            return Ok(saved.bundle);
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let postgres = zeroize::Zeroizing::new(super::deploy::random_key()?);
    // Guarantee the upstream uppercase/digit password contract as well as entropy.
    let admin = zeroize::Zeroizing::new(format!("Aa1{}", super::deploy::random_key()?));
    let files = [
        (
            "kestra.network",
            include_str!("../../../../stacks/kestra/quadlets/kestra.network").into(),
        ),
        (
            "kestra-postgres.container",
            include_str!("../../../../stacks/kestra/quadlets/kestra-postgres.container").into(),
        ),
        (
            "kestra.container",
            include_str!("../../../../stacks/kestra/quadlets/kestra.container").into(),
        ),
        (
            "config/kestra.yml",
            include_str!("../../../../stacks/kestra/kestra.yml")
                .replace("__POSTGRES_PASSWORD__", postgres.as_str())
                .replace("__ADMIN_PASSWORD__", admin.as_str()),
        ),
        (
            "config/kestra-postgres.env",
            format!(
                "POSTGRES_DB=kestra\nPOSTGRES_USER=kestra\nPOSTGRES_PASSWORD={}\n",
                postgres.as_str()
            ),
        ),
    ]
    .into_iter()
    .map(|(path, contents)| File {
        path: path.into(),
        contents,
    })
    .collect();
    let saved = Input {
        version: 1,
        owner,
        bundle: Bundle { files },
    };
    saved.bundle.validate()?;
    let _directory = security::private_directory()?;
    let bytes = zeroize::Zeroizing::new(serde_json::to_vec(&saved).map_err(io::Error::other)?);
    // Save the exact input before reserving a guest; retries/upgrades keep it.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(security::state_dir()?.join(INPUT))?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(saved.bundle)
}
