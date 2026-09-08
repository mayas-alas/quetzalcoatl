pub mod config;
pub mod plan;
pub mod report;
#[cfg(target_os = "linux")]
pub mod runtime;

pub type Result<T> = std::result::Result<T, report::Failure>;
