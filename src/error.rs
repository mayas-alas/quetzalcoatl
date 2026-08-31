use std::fmt;
use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GnxError {
    pub code: &'static str,
    pub component: &'static str,
    pub operation: &'static str,
    pub message: String,
    pub action: String,
    pub retryable: bool,
    #[serde(skip)]
    exit_code: u8,
}

impl GnxError {
    pub fn new(
        code: &'static str,
        component: &'static str,
        operation: &'static str,
        message: impl Into<String>,
        action: impl Into<String>,
        retryable: bool,
        exit_code: u8,
    ) -> Self {
        Self {
            code,
            component,
            operation,
            message: message.into(),
            action: action.into(),
            retryable,
            exit_code,
        }
    }

    pub fn config_not_found(path: &Path) -> Self {
        Self {
            code: "CONFIG_NOT_FOUND",
            component: "config",
            operation: "load",
            message: format!("No existe la configuración en {}.", path.display()),
            action: "Cree config.toml a partir de config.example.toml y vuelva a intentar."
                .to_string(),
            retryable: false,
            exit_code: 2,
        }
    }

    pub fn config_invalid(message: impl Into<String>) -> Self {
        Self {
            code: "CONFIG_INVALID",
            component: "config",
            operation: "validate",
            message: message.into(),
            action: "Corrija la configuración; GNX no aplicó cambios.".to_string(),
            retryable: false,
            exit_code: 2,
        }
    }

    pub fn controller_invalid(message: impl Into<String>) -> Self {
        Self {
            code: "MESH_CONTROLLER_URL_INVALID",
            component: "mesh",
            operation: "controller_preflight",
            message: message.into(),
            action: concat!(
                "Use una URL HTTPS con nombre DNS, sin credenciales, query ni fragment; ",
                "por ejemplo https://controlplane.node.gnx."
            )
            .to_string(),
            retryable: false,
            exit_code: 2,
        }
    }

    pub fn doctor_incomplete() -> Self {
        Self {
            code: "DOCTOR_INCOMPLETE",
            component: "doctor",
            operation: "diagnose",
            message: "El diagnóstico contiene checks fallidos o pendientes.".to_string(),
            action: "Cierre los checks indicados antes de considerar el runtime saludable."
                .to_string(),
            retryable: false,
            exit_code: 4,
        }
    }

    pub fn io(operation: &'static str, message: impl Into<String>) -> Self {
        Self::new(
            "GNX_IO_ERROR",
            "gnx",
            operation,
            message,
            "Revise permisos y espacio disponible; no se asumió que la operación terminó.",
            true,
            5,
        )
    }

    pub fn process(
        operation: &'static str,
        program: &Path,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self::new(
            "PROCESS_FAILED",
            "process",
            operation,
            format!("{}: {}", program.display(), message.into()),
            "Consulte el diagnóstico acotado y corrija la dependencia indicada.",
            retryable,
            6,
        )
    }

    pub fn unsupported_host(message: impl Into<String>) -> Self {
        Self::new(
            "HOST_UNSUPPORTED",
            "host",
            "preflight",
            message,
            "Use Windows o Linux x86_64 dentro de la matriz soportada.",
            false,
            7,
        )
    }

    pub fn exit_code(&self) -> u8 {
        self.exit_code
    }
}

impl fmt::Display for GnxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {}\nAcción: {}",
            self.code, self.message, self.action
        )
    }
}

impl std::error::Error for GnxError {}
