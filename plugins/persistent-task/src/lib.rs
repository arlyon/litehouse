//! Persistent task plugin for the Litehouse home automation system.
//!
//! This crate provides functionality for creating persistent tasks that can run in the background,
//! allowing for continuous operations or monitoring within the Litehouse system.

use std::time::Duration;

use crate::exports::litehouse::plugin::plugin::{
    Every, GuestRunner, Output, Subscription, TimeUnit,
};

litehouse_plugin::generate!(PersistentPlugin);

pub struct PersistentPlugin {
    nickname: String,
}

impl GuestRunner for PersistentPlugin {
    fn new(nickname: String, _config: Option<String>) -> Self {
        // let (commands_tx, commands_rx) = std::sync::mpsc::channel();
        // let (updates_tx, updates_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(1));
                tracing::info!("sending update");
            }
        });
        Self { nickname }
    }

    fn subscribe(&self) -> Result<Vec<Subscription>, u32> {
        Ok(vec![Subscription::Time(
            exports::litehouse::plugin::plugin::TimeSubscription::Every(Every {
                amount: 1,
                unit: TimeUnit::Minute,
            }),
        )])
    }

    fn update(&self, _events: Vec<exports::litehouse::plugin::plugin::Event>) -> Result<bool, u32> {
        Ok(true)
    }

    #[allow(async_fn_in_trait)]
    fn outputs(&self) -> Result<_rt::Vec<Output>, u32> {
        todo!()
    }
}
