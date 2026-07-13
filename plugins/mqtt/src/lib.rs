//! MQTT plugin skeleton for the Litehouse home automation system.
//!
//! The component implements the Litehouse runner contract but does not yet connect
//! to an MQTT broker.

use crate::exports::litehouse::plugin::plugin::{Event, GuestRunner, Output, Subscription};

litehouse_plugin::generate!(MqttPlugin);

pub struct MqttPlugin;

impl GuestRunner for MqttPlugin {
    fn new(_nickname: String, _config: Option<String>) -> Self {
        Self
    }

    fn subscribe(&self) -> Result<Vec<Subscription>, u32> {
        Ok(vec![])
    }

    async fn update(&self, _events: Vec<Event>) -> Result<bool, u32> {
        Ok(true)
    }

    async fn outputs(&self) -> Result<Vec<Output>, u32> {
        Ok(vec![])
    }
}
