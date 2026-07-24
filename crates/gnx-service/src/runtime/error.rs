use super::*;

#[derive(Debug)]
pub(super) struct GateError {
    pub(super) code: &'static str,
    pub(super) component: Component,
    pub(super) message: String,
}

impl GateError {
    pub(super) fn new(
        code: &'static str,
        component: Component,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            component,
            message: message.into(),
        }
    }

    pub(super) fn command(message: impl Into<String>) -> Self {
        Self::new("RUNTIME_GATE_FAILED", Component::None, message)
    }

    pub(super) fn with_code(mut self, code: &'static str, component: Component) -> Self {
        self.code = code;
        self.component = component;
        self
    }
}
