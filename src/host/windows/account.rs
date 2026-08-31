pub const SERVICE_NAME: &str = "QuetzalcoatlNext";
pub const SERVICE_DISPLAY_NAME: &str = "Quetzalcoatl Next";
pub const SERVICE_ACCOUNT: &str = r"NT SERVICE\QuetzalcoatlNext";

use std::path::Path;

use crate::error::GnxError;
use crate::process::CommandSpec;

pub fn grant_data_access(path: &Path) -> Result<(), GnxError> {
    let grant = format!(r"{SERVICE_ACCOUNT}:(OI)(CI)M");
    CommandSpec::new(r"C:\Windows\System32\icacls.exe")
        .arg(path)
        .args(["/grant", &grant, "/T", "/C"])
        .run_checked("service_data_acl")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_account_matches_service_name() {
        assert_eq!(SERVICE_ACCOUNT, format!(r"NT SERVICE\{SERVICE_NAME}"));
    }
}
