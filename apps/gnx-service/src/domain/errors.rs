use super::lifecycle::Component;

#[derive(Debug)]
pub(crate) struct GateError {
    pub(crate) code: &'static str,
    pub(crate) component: Component,
    pub(crate) message: String,
}

impl GateError {
    pub(crate) fn new(
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

    pub(crate) fn command(message: impl Into<String>) -> Self {
        Self::new("RUNTIME_GATE_FAILED", Component::None, message)
    }

    pub(crate) fn with_code(mut self, code: &'static str, component: Component) -> Self {
        self.code = code;
        self.component = component;
        self
    }
}
