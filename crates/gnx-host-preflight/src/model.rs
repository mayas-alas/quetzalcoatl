use serde::Serialize;

#[derive(Serialize)]
pub struct Report {
    pub schema_version: u8,
    pub status: Status,
    pub exit_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_profile: Option<crate::host_profile::HostProfile>,
    pub checks: Vec<Check>,
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pass,
    Fail,
    Error,
    RebootRequired,
}

#[derive(Serialize)]
pub struct Check {
    pub id: &'static str,
    pub status: Status,
    pub message: String,
}

impl Report {
    pub fn new() -> Self {
        Self {
            schema_version: 1,
            status: Status::Pass,
            exit_code: 0,
            host_profile: None,
            checks: Vec::new(),
        }
    }
}
