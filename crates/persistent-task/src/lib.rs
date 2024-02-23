use std::{sync::Arc, time::Duration};

use crate::{
    exports::litehouse::plugin::plugin::{Every, GuestRunner, Subscription, TimeUnit},
    wasi::sockets::{
        instance_network::instance_network,
        network::{self, IpSocketAddress},
        tcp_create_socket::create_tcp_socket,
    },
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;

plugin::generate!(PersistentPlugin);

pub struct PersistentPlugin {
    nickname: String,
}

impl GuestRunner for PersistentPlugin {
    fn new(nickname: String, _config: Option<String>) -> Self {
        plugin::tracing_subscriber();
        // let (commands_tx, commands_rx) = std::sync::mpsc::channel();
        // let (updates_tx, updates_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(1));
            tracing::info!("sending update");
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

    fn update(&self, events: Vec<exports::litehouse::plugin::plugin::Event>) -> Result<bool, u32> {
        Ok(true)
    }
}
