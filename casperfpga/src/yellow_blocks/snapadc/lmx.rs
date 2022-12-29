use crate::transport::Transport;
use std::sync::{
    Mutex,
    Weak,
};

/// Internal SNAP clock synthesizer - LMX2581
#[derive(Debug)]
pub struct LmxSynth<T> {
    /// Upwards pointer to the parent class' transport
    transport: Weak<Mutex<T>>,
}

impl<T> LmxSynth<T>
where
    T: Transport,
{
    const NAME: &'static str = "lmx_ctrl";

    pub fn new(transport: Weak<Mutex<T>>) -> Self {
        Self { transport }
    }
}
