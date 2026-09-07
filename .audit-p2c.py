import io


def load(path):
    return io.open(path, encoding="utf-8").read()


def sub(text, before, after):
    assert text.count(before) == 1, (before[:60], text.count(before))
    return text.replace(before, after)


# ---------------------------------------------------------------- broker.rs
path = "src/windows/broker.rs"
text = load(path)

text = sub(
    text,
    """fn action_code(action: [&str; 2]) -> Option<u8> {
    match action {
        ["access", "configure"] => Some(1),
        ["access", "apply"] => Some(2),
        ["access", "dns"] => Some(3),
        ["compute", "apply"] => Some(4),
        ["compute", "status"] => Some(5),
        ["compute", "credentials"] => Some(6),
        ["controller", "apply"] => Some(7),
        ["controller", "status"] => Some(8),
        _ => None,
    }
}

fn code_action(code: u8) -> Option<[&'static str; 2]> {
    match code {
        1 => Some(["access", "configure"]),
        2 => Some(["access", "apply"]),
        3 => Some(["access", "dns"]),
        4 => Some(["compute", "apply"]),
        5 => Some(["compute", "status"]),
        6 => Some(["compute", "credentials"]),
        7 => Some(["controller", "apply"]),
        8 => Some(["controller", "status"]),
        _ => None,
    }
}""",
    """/// Wire contract: the request code is the position in this table plus one.
const ACTIONS: [(&str, &str); 8] = [
    ("access", "configure"),
    ("access", "apply"),
    ("access", "dns"),
    ("compute", "apply"),
    ("compute", "status"),
    ("compute", "credentials"),
    ("controller", "apply"),
    ("controller", "status"),
];

pub(crate) fn action_code(action: [&str; 2]) -> Option<u8> {
    ACTIONS
        .iter()
        .position(|candidate| *candidate == action)
        .map(|index| index as u8 + 1)
}

fn code_action(code: u8) -> Option<[&'static str; 2]> {
    usize::from(code)
        .checked_sub(1)
        .and_then(|index| ACTIONS.get(index))
        .copied()
}""",
)

text = sub(
    text,
    """        if !sid.starts_with("S-1-")
            || !sid
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'-' || byte == b'S')
        {""",
    """        if !crate::windows::account::valid_sid(sid) {""",
)

text = text.replace("map_err(Error::Spawn)", "map_err(Error::Io)")
text = text.replace("Err(Error::Spawn(error))", "Err(Error::Io(error))")
io.open(path, "w", encoding="utf-8", newline="").write(text)

# --------------------------------------------------------------- account.rs
path = "src/windows/account.rs"
text = load(path)
text = sub(text, "fn valid_sid(value: &str) -> bool {", "pub(crate) fn valid_sid(value: &str) -> bool {")
io.open(path, "w", encoding="utf-8", newline="").write(text)

# --------------------------------------------------------------- runtime.rs
path = "src/windows/runtime.rs"
text = load(path)
text = sub(
    text,
    """pub fn sync_config(config: &[u8]) -> Result<()> {
    if config.len() > 1024 * 1024 {
        return Err(Error::Operation("BROKER_CONFIG_SIZE"));
    }
    checked(""",
    """pub fn sync_config(config: &[u8]) -> Result<()> {
    checked(""",
)
io.open(path, "w", encoding="utf-8", newline="").write(text)

# ------------------------------------------------------------------- cli.rs
path = "src/cli.rs"
text = load(path)
text = sub(
    text,
    """    fn command_args(&self) -> [&'static str; 2] {
        match self.command {
            Capability::Access { action } => [
                "access",
                match action {
                    AccessAction::Configure => "configure",
                    AccessAction::Apply => "apply",
                    AccessAction::Dns => "dns",
                },
            ],
            Capability::Compute { action } => [
                "compute",
                match action {
                    ComputeAction::Apply => "apply",
                    ComputeAction::Status => "status",
                    ComputeAction::Credentials => "credentials",
                },
            ],
            Capability::Controller { action } => [
                "controller",
                match action {
                    ControllerAction::Apply => "apply",
                    ControllerAction::Status => "status",
                },
            ],
        }
    }""",
    """    fn command_args(&self) -> [&'static str; 2] {
        let action = match self.command {
            Capability::Access { action } => match action {
                AccessAction::Configure => "configure",
                AccessAction::Apply => "apply",
                AccessAction::Dns => "dns",
            },
            Capability::Compute { action } => match action {
                ComputeAction::Apply => "apply",
                ComputeAction::Status => "status",
                ComputeAction::Credentials => "credentials",
            },
            Capability::Controller { action } => match action {
                ControllerAction::Apply => "apply",
                ControllerAction::Status => "status",
            },
        };
        let capability = match self.command {
            Capability::Access { .. } => "access",
            Capability::Compute { .. } => "compute",
            Capability::Controller { .. } => "controller",
        };
        [capability, action]
    }""",
)
text = sub(
    text,
    """    #[test]
    fn secrets_have_no_cli_position() {""",
    """    #[cfg(windows)]
    #[test]
    fn every_command_maps_to_a_broker_action() {
        for args in [
            ["gnx", "access", "configure"],
            ["gnx", "access", "apply"],
            ["gnx", "access", "dns"],
            ["gnx", "compute", "apply"],
            ["gnx", "compute", "status"],
            ["gnx", "compute", "credentials"],
            ["gnx", "controller", "apply"],
            ["gnx", "controller", "status"],
        ] {
            let cli = Cli::try_parse_from(args).unwrap();
            assert!(crate::windows::broker::action_code(cli.command_args()).is_some());
        }
    }

    #[test]
    fn secrets_have_no_cli_position() {""",
)
io.open(path, "w", encoding="utf-8", newline="").write(text)

# ---------------------------------------------------------------- enroll.sh
path = "runtime/access/enroll.sh"
text = load(path)
text = sub(
    text,
    """cat > "$key"
result=0
"$@" --auth-key="file:$key" || result=$?
rm -f -- "$key"
trap - EXIT
exit "$result\"""",
    """cat > "$key"
"$@" --auth-key="file:$key\"""",
)
io.open(path, "w", encoding="utf-8", newline="").write(text)
print("windows + cli + enroll ok")
