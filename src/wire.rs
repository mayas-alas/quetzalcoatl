use crate::{
    config::Config,
    domain::secret::{Secret, SecretKind},
};
use zeroize::Zeroizing;
pub const MAX_INTENT: usize = 65536;
pub const MAX_SECRET: usize = 4096;
pub const MAX_FRAME: usize = MAX_INTENT + MAX_SECRET + 16;
pub fn opcode(op: &str) -> Option<u8> {
    match op {
        "plan" => Some(1),
        "apply" => Some(2),
        "status" => Some(3),
        "doctor" => Some(4),
        _ => None,
    }
}
pub fn operation(code: u8) -> Option<&'static str> {
    match code {
        1 => Some("plan"),
        2 => Some("apply"),
        3 => Some("status"),
        4 => Some("doctor"),
        _ => None,
    }
}
pub fn encode(
    op: &str,
    intent: &str,
    secret: Option<&Secret>,
) -> Result<Zeroizing<Vec<u8>>, String> {
    let op = opcode(op).ok_or("INVALID_OPERATION")?;
    Config::parse(intent)?;
    if intent.len() > MAX_INTENT || (secret.is_some() && op != 2) {
        return Err("INVALID_FRAME".into());
    }
    let bytes = secret.map(|s| &s.value[..]).unwrap_or(&[]);
    if bytes.len() > MAX_SECRET {
        return Err("INVALID_FRAME".into());
    }
    let mut b = Zeroizing::new(Vec::with_capacity(16 + intent.len() + bytes.len()));
    b.extend_from_slice(b"GNX1");
    b.push(op);
    b.push(
        secret
            .map(|s| match s.kind {
                SecretKind::ComputePassword => 1,
                SecretKind::AccessEnrollment => 2,
            })
            .unwrap_or(0),
    );
    b.extend_from_slice(&[0, 0]);
    b.extend_from_slice(&(intent.len() as u32).to_le_bytes());
    b.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    b.extend_from_slice(intent.as_bytes());
    b.extend_from_slice(bytes);
    Ok(b)
}
pub struct Request {
    pub operation: &'static str,
    pub intent: String,
    pub secret: Option<Secret>,
}
pub fn decode(b: &[u8]) -> Result<Request, String> {
    if b.len() < 16 || &b[..4] != b"GNX1" || b[6..8] != [0, 0] {
        return Err("INVALID_FRAME".into());
    }
    let op = operation(b[4]).ok_or("INVALID_OPERATION")?;
    let n = u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
    let m = u32::from_le_bytes(b[12..16].try_into().unwrap()) as usize;
    if n > MAX_INTENT || m > MAX_SECRET || b.len() != 16 + n + m || (m > 0 && op != "apply") {
        return Err("INVALID_FRAME".into());
    }
    let intent = std::str::from_utf8(&b[16..16 + n])
        .map_err(|_| "INVALID_FRAME")?
        .to_string();
    Config::parse(&intent)?;
    let kind = match b[5] {
        0 if m == 0 => None,
        1 if m > 0 => Some(SecretKind::ComputePassword),
        2 if m > 0 => Some(SecretKind::AccessEnrollment),
        _ => return Err("INVALID_FRAME".into()),
    };
    let secret = kind
        .map(|k| Secret::new(k, b[16 + n..].to_vec()))
        .transpose()?;
    Ok(Request {
        operation: op,
        intent,
        secret,
    })
}
