//! A basic no-op plugin intended to be used as a starting point for writing
//! your own plugin.

use crate::exports::litehouse::plugin::plugin::{Event, GuestRunner, Output, Subscription};

litehouse_plugin::generate!(NoopPlugin);

pub struct NoopPlugin;

impl GuestRunner for NoopPlugin {
    fn new(_nickname: String, _config: Option<String>) -> Self {
        Self
    }

    fn subscribe(&self) -> Result<Vec<Subscription>, u32> {
        Ok(vec![])
    }

    fn update(&self, _events: Vec<Event>) -> Result<bool, u32> {
        Ok(true)
    }

    fn outputs(&self) -> Result<_rt::Vec<Output>, u32> {
        todo!()
    }
}
