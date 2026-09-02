use crate::Result;

#[derive(Debug, Clone, Copy)]
pub struct HostState {
    pub elevated: bool,
}

pub trait Host {
    fn inspect(&self) -> Result<HostState>;
}
