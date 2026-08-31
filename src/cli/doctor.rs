use std::path::PathBuf;

use crate::error::GnxError;
use crate::report::{CheckState, DoctorReport};

pub fn run(config_path: PathBuf, json: bool) -> Result<(), GnxError> {
    let report = DoctorReport::collect(&config_path);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("DoctorReport siempre serializa")
        );
    } else {
        for check in &report.checks {
            let marker = match check.state {
                CheckState::Pass => "PASS",
                CheckState::Fail => "FAIL",
            };
            println!("[{marker}] {} — {}", check.id, check.detail);
        }
    }
    if report.has_blockers() {
        Err(GnxError::doctor_incomplete())
    } else {
        Ok(())
    }
}
