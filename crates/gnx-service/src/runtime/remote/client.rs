use super::super::*;
use super::operation::RuntimeOperation;
use super::transport::{machine_stdin, machine_stdin_output};

pub(in crate::runtime) fn runtime_agent(
    podman: &Path,
    operation: RuntimeOperation,
    input: &[u8],
) -> Result<Output, GateError> {
    let command = runtime_agent_command(operation);
    machine_stdin(podman, command, input)
}

pub(in crate::runtime) fn runtime_agent_output(
    podman: &Path,
    operation: RuntimeOperation,
    input: &[u8],
) -> Result<Output, GateError> {
    let command = runtime_agent_command(operation);
    machine_stdin_output(podman, command, input)
}

fn runtime_agent_command(operation: RuntimeOperation) -> Vec<OsString> {
    let mut command = Vec::with_capacity(operation.argv().len() + 1);
    command.push(OsString::from(RUNTIME_AGENT_BIN));
    command.extend(operation.argv().iter().map(|value| OsString::from(*value)));
    command
}

pub(in crate::runtime) fn verify_runtime_agent(podman: &Path) -> Result<(), GateError> {
    let output = runtime_agent(podman, RuntimeOperation::Ping, &[])
        .map_err(|error| error.with_code("RUNTIME_AGENT_FAILED", Component::PodmanMachine))?;
    if String::from_utf8_lossy(&output.stdout).trim() != "GNX_RUNTIME_AGENT=1" {
        return Err(GateError::new(
            "RUNTIME_AGENT_FAILED",
            Component::PodmanMachine,
            "Fedora runtime agent did not confirm protocol version 1",
        ));
    }
    Ok(())
}
