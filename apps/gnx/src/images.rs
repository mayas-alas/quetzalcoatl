use crate::protocol::invalid;
use std::io;

// Pins remain the default. Tracking a registry tag must be an explicit choice
// paired with Podman's native pull-on-start policy; no implicit latest or timer.
pub fn validate(image: &str, pull: Option<&str>) -> io::Result<()> {
    let pull = pull.unwrap_or("missing");
    if !matches!(pull, "missing" | "never" | "newer") {
        return Err(invalid("política de imagen no admitida"));
    }
    if let Some((name, digest)) = image.split_once("@sha256:") {
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-_/:".contains(&b))
            || digest.len() != 64
            || !digest.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(invalid("referencia de imagen inválida"));
        }
        if pull == "newer" {
            return Err(invalid("Pull=newer requiere un tag, no un digest fijado"));
        }
        return Ok(());
    }
    if pull != "newer" {
        return Err(invalid("imagen sin digest fijado; un tag exige Pull=newer"));
    }
    if !qualified_tag(image) {
        return Err(invalid(
            "actualización requiere registro/repositorio:tag explícitos, sin URL ni credenciales",
        ));
    }
    Ok(())
}

fn qualified_tag(image: &str) -> bool {
    let Some((name, tag)) = image.rsplit_once(':') else {
        return false;
    };
    if name.len() > 255
        || tag.is_empty()
        || tag.len() > 128
        || !tag
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        || !tag
            .as_bytes()
            .first()
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
    {
        return false;
    }
    let Some((registry, repository)) = name.split_once('/') else {
        return false;
    };
    let (host, port) = match registry.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (registry, None),
    };
    if host.len() > 253
        || (!host.contains('.') && host != "localhost" && port.is_none())
        || !host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && lower_alphanumeric_edges(label)
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
        || port.is_some_and(|port| {
            port.is_empty()
                || !port.bytes().all(|b| b.is_ascii_digit())
                || !port.parse::<u16>().is_ok_and(|number| number > 0)
        })
    {
        return false;
    }
    // A deliberately small subset of registry names: ASCII DNS (optional port)
    // and repository components. No transports, IPv6 literals or auth in names.
    repository.split('/').all(repository_component)
}

fn repository_component(mut part: &str) -> bool {
    loop {
        let end = part
            .bytes()
            .take_while(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            .count();
        if end == 0 {
            return false;
        }
        part = &part[end..];
        if part.is_empty() {
            return true;
        }
        part = if let Some(tail) = part.strip_prefix("__") {
            tail
        } else if part.starts_with(['.', '_']) {
            &part[1..]
        } else if part.starts_with('-') {
            part.trim_start_matches('-')
        } else {
            return false;
        };
    }
}

fn lower_alphanumeric_edges(value: &str) -> bool {
    let valid = |b: &u8| b.is_ascii_lowercase() || b.is_ascii_digit();
    value.as_bytes().first().is_some_and(valid) && value.as_bytes().last().is_some_and(valid)
}
