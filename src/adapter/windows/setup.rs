use crate::port::host::{SetupHost, SetupObservation};
use std::path::Path;

pub struct WindowsSetupHost;

impl SetupHost for WindowsSetupHost {
    fn preflight_setup(&self) -> SetupObservation {
        preflight()
    }
}

fn preflight() -> SetupObservation {
    let legacy_program = Path::new(r"C:\Program Files\QuetzalcoatlNext");
    let current_program = Path::new(r"C:\Program Files\GNX");
    let legacy_state = Path::new(r"C:\ProgramData\QuetzalcoatlNext");
    let service_binary = current_program.join("gnx-service.exe");
    let old_tray_binary = current_program.join("gnx-tray.exe");

    // rama-mvp already installs under C:\Program Files\GNX. The tray binary
    // and absence of the 0.3.1 service distinguish that layout from the target.
    let legacy_present = legacy_program.exists()
        || legacy_state.exists()
        || (old_tray_binary.exists() && !service_binary.exists());

    SetupObservation {
        legacy_present,
        target_present: service_binary.exists(),
    }
}
