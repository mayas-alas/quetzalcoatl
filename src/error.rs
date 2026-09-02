use std::io;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid CLI arguments")]
    Arguments,
    #[error("--config is required")]
    ConfigRequired,
    #[error("configuration could not be read")]
    ConfigRead(#[source] io::Error),
    #[error("configuration is not valid")]
    ConfigParse(#[source] toml::de::Error),
    #[error("configuration version is not supported")]
    ConfigVersion,
    #[error("control_server is not a valid HTTPS endpoint")]
    Endpoint,
    #[error("release manifest could not be read")]
    ReleaseRead(#[source] io::Error),
    #[error("release manifest is not valid")]
    ReleaseParse(#[source] toml::de::Error),
    #[error("release manifest failed validation")]
    ReleaseInvalid,
    #[error("release evidence is incomplete")]
    ReleaseEvidence,
    #[error("this host is not supported")]
    HostUnsupported,
    #[error("this operation requires elevation")]
    ElevationRequired,
    #[error("mesh client is not installed")]
    ClientMissing,
    #[error("mesh client version does not match the release")]
    ClientVersion,
    #[error("client package could not be read")]
    PackageRead(#[source] io::Error),
    #[error("client package digest does not match the release")]
    PackageDigest,
    #[error("setup key file is not a regular file")]
    SetupKeyFile,
    #[error("an external operation could not start")]
    Spawn(#[source] io::Error),
    #[error("an external operation failed")]
    External { operation: &'static str, code: i32 },
    #[error("access DNS checks failed")]
    AccessReport {
        operation: &'static str,
        fields: String,
    },
}

impl Error {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Arguments => "ARGUMENTS",
            Self::ConfigRequired => "CONFIG_REQUIRED",
            Self::ConfigRead(_) => "CONFIG_READ",
            Self::ConfigParse(_) => "CONFIG_PARSE",
            Self::ConfigVersion => "CONFIG_VERSION",
            Self::Endpoint => "CONFIG_ENDPOINT",
            Self::ReleaseRead(_) => "RELEASE_READ",
            Self::ReleaseParse(_) => "RELEASE_PARSE",
            Self::ReleaseInvalid => "RELEASE_INVALID",
            Self::ReleaseEvidence => "RELEASE_EVIDENCE",
            Self::HostUnsupported => "HOST_UNSUPPORTED",
            Self::ElevationRequired => "ELEVATION_REQUIRED",
            Self::ClientMissing => "CLIENT_MISSING",
            Self::ClientVersion => "CLIENT_VERSION",
            Self::PackageRead(_) => "PACKAGE_READ",
            Self::PackageDigest => "PACKAGE_DIGEST",
            Self::SetupKeyFile => "SETUP_KEY_FILE",
            Self::Spawn(_) => "PROCESS_START",
            Self::External { operation, .. } | Self::AccessReport { operation, .. } => operation,
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Arguments
            | Self::ConfigRequired
            | Self::ConfigRead(_)
            | Self::ConfigParse(_)
            | Self::ConfigVersion
            | Self::Endpoint => 2,
            Self::ReleaseRead(_)
            | Self::ReleaseParse(_)
            | Self::ReleaseInvalid
            | Self::ReleaseEvidence
            | Self::PackageRead(_)
            | Self::PackageDigest => 3,
            Self::HostUnsupported | Self::ElevationRequired => 4,
            Self::ClientMissing | Self::ClientVersion | Self::SetupKeyFile => 5,
            Self::Spawn(_) | Self::External { .. } | Self::AccessReport { .. } => 6,
        }
    }
}
