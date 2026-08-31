pub mod cli;
pub mod config;
pub mod download;
pub mod error;
pub mod host;
pub mod journal;
pub mod logs;
pub mod process;
pub mod report;
pub mod runtime;
pub mod secrets;
pub mod state;

use std::ffi::OsString;
use std::process::ExitCode;

pub fn run<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match cli::execute(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            logs::error(&error);
            eprintln!("{error}");
            ExitCode::from(error.exit_code())
        }
    }
}
