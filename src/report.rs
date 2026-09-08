use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Failure {
    pub code: String,
    pub message: String,
    pub action_required: bool,
}

impl Failure {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            action_required: false,
        }
    }
    pub fn action(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            action_required: true,
        }
    }
    pub fn exit_code(&self) -> u8 {
        if self.action_required { 2 } else { 1 }
    }
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Failure {}
impl From<std::io::Error> for Failure {
    fn from(error: std::io::Error) -> Self {
        Self::new("IO", error.to_string())
    }
}

pub fn output(capability: &str, phase: &str, result: crate::Result<serde_json::Value>) -> u8 {
    let (state, code, details, exit) = match result {
        Ok(value) => ("READY", "OK".to_string(), value, 0),
        Err(error) => {
            let exit = error.exit_code();
            (
                if error.action_required {
                    "ACTION_REQUIRED"
                } else {
                    "FAILED"
                },
                error.code,
                serde_json::json!({"message":error.message}),
                exit,
            )
        }
    };
    println!(
        "{}",
        serde_json::json!({"schema":1,"state":state,"capability":capability,
        "phase":phase,"code":code,"scope":"local","details":details})
    );
    exit
}
