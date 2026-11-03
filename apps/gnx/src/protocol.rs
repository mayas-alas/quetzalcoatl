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
    UpdateCheck,
    UpdateApply,
    Recovery,
}

impl Operation {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "start" => Some(Self::Start),
            "stop" => Some(Self::Stop),
            "status" => Some(Self::Status),
            "enroll" => Some(Self::Enroll),
            "password" => Some(Self::Password),
            "recovery" => Some(Self::Recovery),
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog: Option<crate::updates::Catalog>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub progress: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recover_missing: Option<MissingDeployment>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MissingDeployment {
    pub vmid: u32,
    pub digest: String,
    pub sid: String,
    pub distribution_id: String,
    pub attempt: u32,
}

impl std::fmt::Display for MissingDeployment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "LXC {} ausente en PVE, ausencia confirmada por inventario nativo (intento {}). Recuperar archiva los recibos de ese despliegue: la reserva y, si llegó a completarse, también la completion y el registro del stack privado (el servicio del nodo sigue activo sin stack). Después solicita un guest nuevo con la misma configuración y credenciales. No recupera datos del guest eliminado ni borra otros recursos, guests, identidades o CA",
            self.vmid, self.attempt
        )
    }
}

impl std::error::Error for MissingDeployment {}

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
            (Operation::Deploy, None) => {} // Fixed private-services input, prepared by the service.
            (_, Some(_)) => return Err(invalid("operación no admite archivos")),
            _ => {}
        }
        match (request.operation, &request.catalog) {
            (Operation::UpdateCheck | Operation::UpdateApply, Some(catalog)) => {
                catalog.validate()?
            }
            (_, Some(_)) => return Err(invalid("esta operación no admite catálogos")),
            _ => {}
        }
        if request.progress && request.operation != Operation::Deploy {
            return Err(invalid("esta operación no admite progreso"));
        }
        if let Some(recovery) = &request.recover_missing {
            if request.operation != Operation::Deploy
                || !(100..=999999999).contains(&recovery.vmid)
                || recovery.digest.len() != 64
                || !recovery.digest.bytes().all(|c| c.is_ascii_hexdigit())
                || recovery.sid.len() > 256
                || recovery.distribution_id.len() > 64
                || recovery.attempt >= 32
            {
                return Err(invalid("confirmación de recuperación inválida"));
            }
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
            // The account identifier travels as the protected entry: it is not a
            // credential, but it is personal data; no echo, argv or logs.
            Operation::Recovery => {
                let value = request
                    .secret
                    .as_deref()
                    .ok_or_else(|| invalid("falta la cuenta operadora"))?;
                if value.is_empty()
                    || value.len() > 256
                    || value.chars().any(char::is_control)
                    || value.trim() != value
                {
                    return Err(invalid("cuenta operadora inválida"));
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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub progress: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recover_missing: Option<MissingDeployment>,
}

impl Response {
    pub fn new(request: &Request, ok: bool, state: NodeState, message: &str) -> Self {
        Self {
            version: VERSION,
            id: request.id,
            ok,
            state,
            message: message.into(),
            progress: false,
            recover_missing: None,
        }
    }

    pub fn decode(bytes: &[u8], id: [u8; 16]) -> io::Result<Self> {
        if bytes.len() > MAX_FRAME {
            return Err(invalid("respuesta fuera de límite"));
        }
        let response: Self =
            serde_json::from_slice(bytes).map_err(|_| invalid("respuesta inválida"))?;
        if response.version != VERSION || response.id != id || response.message.len() > 8192 {
            return Err(invalid("respuesta no correlacionada"));
        }
        Ok(response)
    }
}

pub fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
