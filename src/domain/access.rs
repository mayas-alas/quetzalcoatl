pub fn validate(n: &crate::config::Network) -> Result<(), String> {
    let (ip, prefix) = n.subnet.split_once('/').ok_or("INVALID_SUBNET")?;
    let ip: std::net::Ipv4Addr = ip.parse().map_err(|_| "INVALID_SUBNET")?;
    let p: u32 = prefix.parse().map_err(|_| "INVALID_SUBNET")?;
    if !(8..=30).contains(&p) || !ip.is_private() || u32::from(ip) & (!0u32 >> p) != 0 {
        return Err("INVALID_SUBNET".into());
    }
    Ok(())
}
