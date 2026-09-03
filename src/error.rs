use std::io;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid CLI arguments")]
    Arguments,
    #[error("configuration could not be read")]
    ConfigRead(#[source] io::Error),
    #[error("configuration is not valid")]
    ConfigParse(#[source] toml::de::Error),
    #[error("configuration failed validation")]
    ConfigInvalid,
    #[error("this host is not supported")]
    HostUnsupported,
    #[error("an external operation could not start")]
    Spawn(#[source] io::Error),
    #[error("an operation failed")]
    Operation(&'static str),
    #[error("access checks failed")]
    AccessReport {
        operation: &'static str,
        fields: String,
    },
}

impl Error {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Arguments => "ARGUMENTS",
            Self::ConfigRead(_) => "CONFIG_READ",
            Self::ConfigParse(_) => "CONFIG_PARSE",
            Self::ConfigInvalid => "CONFIG_INVALID",
            Self::HostUnsupported => "HOST_UNSUPPORTED",
            Self::Spawn(_) => "PROCESS_START",
            Self::Operation(label)
            | Self::AccessReport {
                operation: label, ..
            } => label,
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Arguments | Self::ConfigRead(_) | Self::ConfigParse(_) | Self::ConfigInvalid => 2,
            Self::HostUnsupported => 4,
            Self::Spawn(_) | Self::Operation(_) | Self::AccessReport { .. } => 6,
        }
    }
}
