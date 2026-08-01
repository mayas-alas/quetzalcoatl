use crate::domain::errors::GateError;
use crate::infrastructure::payload::{load_machine_image, load_payload_files};
use crate::infrastructure::platform_bundle::validate_platform_bundle;
use crate::infrastructure::podman::installed_machine_image;

pub(crate) fn validate() -> Result<(), GateError> {
    load_payload_files()?;
    validate_platform_bundle()?;
    let machine_image = load_machine_image()?;
    installed_machine_image(&machine_image)?;
    Ok(())
}
