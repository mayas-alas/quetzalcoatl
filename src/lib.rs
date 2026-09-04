pub mod cli;
pub mod config;
pub mod error;
pub mod platform;

#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod access;
#[cfg(target_os = "linux")]
pub mod compute;
#[cfg(target_os = "linux")]
pub mod controller;

pub use error::{Error, Result};
