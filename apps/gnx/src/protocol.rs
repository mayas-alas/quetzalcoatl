use serde::{Deserialize, Serialize};
use std::io;
use zeroize::Zeroize;

pub const VERSION: u16 = 1;
pub const MAX_FRAME: usize = 512 * 1024;
pub const SERVICE: &str = "GnXCore";
pub const PIPE: &str = r"\\.\pipe\GnX.Core.v1";
pub const DISTRO: &str = "gnx-core";
pub const ACCOUNT: &str = "gnxcoresvc";
pub const TARGET: &str = "gnx-core.target";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Start,
    Stop,
    Status,
    Enroll,
    Password,
    // Internal installer operation, deliberately absent from CLI parsing/help.
    Retire,
    Deploy,
}

impl Operation {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "start" => Some(Self::Start),
            "stop" => Some(Self::Stop),
            "status" => Some(Self::Status),
            "enroll" => Some(Self::Enroll),
            "password" => Some(Self::Password),
            _ => None,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u16,
    pub id: [u8; 16],
    pub operation: Operation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle: Option<crate::deployment::Bundle>,
}

impl Drop for Request {
    fn drop(&mut self) {
        if let Some(secret) = &mut self.secret {
            secret.zeroize();
        }
    }
}

impl Request {
    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.is_empty() || bytes.len() > MAX_FRAME {
            return Err(invalid("frame fuera de límite"));
        }
        let request: Self =
            serde_json::from_slice(bytes).map_err(|_| invalid("petición inválida"))?;
        if request.version != VERSION || request.id == [0; 16] {
            return Err(invalid("versión o correlación inválida"));
        }
        match (request.operation, &request.bundle) {
            (Operation::Deploy, Some(bundle)) => bundle.validate()?,
            (Operation::Deploy, None) => return Err(invalid("faltan Quadlets")),
            (_, Some(_)) => return Err(invalid("operación no admite archivos")),
            _ => {}
        }
        match request.operation {
            Operation::Enroll | Operation::Password => {
                let value = request
                    .secret
                    .as_deref()
                    .ok_or_else(|| invalid("falta entrada protegida"))?;
                if value.is_empty() || value.len() > 1024 || value.chars().any(char::is_control) {
                    return Err(invalid("entrada protegida inválida"));
                }
                if request.operation == Operation::Password && (value.len() < 12 || value == "root")
                {
                    return Err(invalid("contraseña debe tener al menos 12 bytes"));
                }
            }
            _ if request.secret.is_some() => {
                return Err(invalid("esta operación no acepta secretos"));
            }
            _ => {}
        }
        Ok(request)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeState {
    NotPrepared,
    Stopped,
    Starting,
    Active,
    Degraded,
    Unknown,
    Busy,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub version: u16,
    pub id: [u8; 16],
    pub ok: bool,
    pub state: NodeState,
    pub message: String,
}

impl Response {
    pub fn new(request: &Request, ok: bool, state: NodeState, message: &str) -> Self {
        Self {
            version: VERSION,
            id: request.id,
            ok,
            state,
            message: message.into(),
        }
    }

    pub fn decode(bytes: &[u8], id: [u8; 16]) -> io::Result<Self> {
        if bytes.len() > MAX_FRAME {
            return Err(invalid("respuesta fuera de límite"));
        }
        let response: Self =
            serde_json::from_slice(bytes).map_err(|_| invalid("respuesta inválida"))?;
        if response.version != VERSION || response.id != id || response.message.len() > 512 {
            return Err(invalid("respuesta no correlacionada"));
        }
        Ok(response)
    }
}

pub fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
