//! Persisted installer contract. No passwords or executable paths are accepted from state.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage { Preparing, RebootRequired, Provisioning, Downloading, Configuring, Verifying, Complete, Failed }
impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Preparing => "Preparando Windows y WSL",
            Self::RebootRequired => "Es necesario reiniciar Windows",
            Self::Provisioning => "Preparando cuenta y servicio",
            Self::Downloading => "Descargando Ubuntu 24.04",
            Self::Configuring => "Configurando Linux y Podman",
            Self::Verifying => "Verificando el entorno",
            Self::Complete => "Instalación completada",
            Self::Failed => "La instalación necesita atención",
        }
    }
    pub fn busy(self) -> bool { !matches!(self, Self::RebootRequired | Self::Complete | Self::Failed) }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub version: u32,
    pub client_sid: String,
    pub stage: Stage,
    pub detail: String,
    pub updated_unix: u64,
    pub reboot_boot_id: Option<String>,
    pub provisioning_started: bool,
}
impl State {
    pub fn new(client_sid: String) -> Self {
        Self { version: 1, client_sid, stage: Stage::Preparing, detail: String::new(),
            updated_unix: crate::protocol::unix_now(), reboot_boot_id: None, provisioning_started: false }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || !valid_client_sid(&self.client_sid) { return Err("Estado del instalador incompatible o identidad inválida.".into()); }
        if self.stage == Stage::RebootRequired && self.reboot_boot_id.as_deref().is_none_or(str::is_empty) {
            return Err("Falta la identidad de arranque para el reinicio pendiente.".into());
        }
        Ok(())
    }
    pub fn set(&mut self, stage: Stage, detail: impl Into<String>) {
        self.stage = stage; self.detail = detail.into(); self.updated_unix = crate::protocol::unix_now();
    }
    pub fn reboot_still_pending(&self, boot_id: &str) -> bool { self.reboot_boot_id.as_deref() == Some(boot_id) }
}

pub fn valid_client_sid(s: &str) -> bool {
    s.starts_with("S-1-5-21-") && s.len() < 190 && s.split('-').count() == 8
        && s.split('-').skip(1).all(|v| !v.is_empty() && v.bytes().all(|c| c.is_ascii_digit()) && v.parse::<u32>().is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> State { State::new("S-1-5-21-1-2-3-1001".into()) }
    #[test] fn state_roundtrip() { let s = state(); let decoded: State = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap(); assert!(decoded.validate().is_ok()); }
    #[test] fn refuse_unknown_version_and_identity() { let mut s = state(); s.version = 2; assert!(s.validate().is_err()); s.version = 1; s.client_sid = "S-1-5-18".into(); assert!(s.validate().is_err()); }
    #[test] fn reboot_is_not_a_retry_in_same_boot() { let mut s = state(); s.reboot_boot_id = Some("boot-1".into()); assert!(s.reboot_still_pending("boot-1")); assert!(!s.reboot_still_pending("boot-2")); }
    #[test] fn only_real_completion_is_complete() { for stage in [Stage::Preparing, Stage::Provisioning, Stage::Downloading, Stage::Configuring, Stage::Verifying] { assert!(stage.busy()); assert_ne!(stage, Stage::Complete); } }
    #[test] fn reboot_requires_checkpoint() {
        let mut s = state(); s.stage = Stage::RebootRequired;
        assert!(s.validate().is_err()); s.reboot_boot_id = Some("boot-1".into()); assert!(s.validate().is_ok());
    }
    #[test] fn state_cannot_supply_executable_or_password() {
        let mut value = serde_json::to_value(state()).unwrap();
        value["executable"] = "cmd.exe".into();
        assert!(serde_json::from_value::<State>(value).is_err());
        assert!(!serde_json::to_string(&state()).unwrap().contains("password"));
    }
    #[test] fn reject_task_injection() { assert!(!valid_client_sid("S-1-5-21-1-2-3-1001';Start-Process calc;#")); }
}
