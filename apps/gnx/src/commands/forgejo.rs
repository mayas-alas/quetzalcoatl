use gnx_contracts::ForgejoAdminStage;

use crate::client;
use crate::error::CliResult;

pub(crate) fn show() -> CliResult<()> {
    print_credential(client::forgejo_admin_show()?, ForgejoAdminStage::Shown)
}

pub(crate) fn reset() -> CliResult<()> {
    print_credential(client::forgejo_admin_reset()?, ForgejoAdminStage::Reset)
}

fn print_credential(
    response: gnx_contracts::ForgejoAdminResponse,
    expected_stage: ForgejoAdminStage,
) -> CliResult<()> {
    if !response.accepted {
        return Err(format!(
            "{}: {}",
            response
                .error_code
                .as_deref()
                .unwrap_or("FORGEJO_ADMIN_FAILED"),
            response.message.as_deref().unwrap_or("operation rejected")
        ));
    }
    if response.stage != expected_stage {
        return Err("service returned an unexpected Forgejo admin stage".into());
    }
    let username = response
        .username
        .as_deref()
        .ok_or_else(|| "service omitted the Forgejo admin username".to_string())?;
    let password = response
        .password
        .as_deref()
        .ok_or_else(|| "service omitted the Forgejo admin password".to_string())?;
    println!("username: {username}");
    println!("password: {password}");
    Ok(())
}
