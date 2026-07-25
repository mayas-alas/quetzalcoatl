use crate::client;
use crate::error::CliResult;
use crate::output;

pub(crate) fn run(json: bool) -> CliResult<()> {
    let status = client::status()?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&status)
                .map_err(|error| format!("cannot encode status: {error}"))?
        );
    } else {
        output::print_human(&status);
    }
    Ok(())
}
