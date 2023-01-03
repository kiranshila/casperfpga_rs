use crate::transport::Transport;
use std::sync::{
    Mutex,
    Weak,
};

/// Internal SNAP clock synthesizer - LMX2581
#[derive(Debug)]
pub struct Synth<T> {
    /// Upwards pointer to the parent class' transport
    _transport: Weak<Mutex<T>>,
}

impl<T> Synth<T>
where
    T: Transport,
{
    const _NAME: &'static str = "lmx_ctrl";

    #[must_use]
    pub fn new(transport: Weak<Mutex<T>>) -> Self {
        Self {
            _transport: transport,
        }
    }
}
