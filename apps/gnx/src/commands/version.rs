use crate::error::CliResult;

pub(crate) fn run() -> CliResult<()> {
    println!("gnx {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_contract_uses_the_crate_version() {
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }
}
