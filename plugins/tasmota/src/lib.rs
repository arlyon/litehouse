//! Tasmota plugin for the Litehouse home automation system.
//!
//! This crate provides integration with Tasmota devices, allowing for control and monitoring
//! within the Litehouse system.

use std::sync::Mutex;

use wasi::http::{
    outgoing_handler,
    types::{Fields, OutgoingRequest, RequestOptions, Scheme},
};

use crate::exports::litehouse::plugin::plugin::{Every, GuestRunner, Subscription, TimeUnit};

litehouse_plugin::generate!(TasmotaPlugin, TasmotaConfig);

pub struct TasmotaPlugin {
    nickname: String,
    state: Mutex<bool>,
    ip: (u8, u8, u8, u8),
}

#[derive(litehouse_plugin::JsonSchema, serde::Deserialize)]
pub struct TasmotaConfig {
    /// The ip address of the device to connect to.
    pub ip: (u8, u8, u8, u8),
}

impl TasmotaPlugin {
    fn get_state(&self) -> bool {
        *self.state.lock().unwrap()
    }

    fn set_state(&self, state: bool) {
        *self.state.lock().unwrap() = state;
    }
}

impl GuestRunner for TasmotaPlugin {
    fn new(nickname: String, config: Option<String>) -> Self {
        let TasmotaConfig { ip } = serde_json::from_str(&config.unwrap_or_default()).unwrap();
        Self {
            nickname,
            ip,
            state: Mutex::new(false),
        }
    }

    fn outputs(&self) -> Result<_rt::Vec<exports::litehouse::plugin::plugin::Output>, u32> {
        return Ok(vec![]);
    }

    fn subscribe(&self) -> Result<Vec<Subscription>, u32> {
        Ok(vec![Subscription::Time(
            exports::litehouse::plugin::plugin::TimeSubscription::Every(Every {
                amount: 5,
                unit: TimeUnit::Second,
            }),
        )])
    }

    fn update(&self, _events: Vec<exports::litehouse::plugin::plugin::Event>) -> Result<bool, u32> {
        tracing::debug!("UPDATE");
        let body = {
            let headers = Fields::new();

            let req = OutgoingRequest::new(headers);
            req.set_path_with_query(Some("/cm?cmnd=Status0"))
                .expect("ok");
            let (a, b, c, d) = self.ip;
            req.set_authority(Some(&format!("{}.{}.{}.{}:80", a, b, c, d)))
                .unwrap();
            req.set_scheme(Some(&Scheme::Http)).unwrap();

            let opts = RequestOptions::new();

            let x = outgoing_handler::handle(req, Some(opts)).unwrap();

            x.subscribe().block();
            let resp = x.get().unwrap().unwrap().unwrap();

            let body = resp.consume().unwrap();
            let stream = body.stream().unwrap();

            let mut body = vec![];
            loop {
                match stream.blocking_read(1024) {
                    Ok(data) => body.extend(data),
                    Err(wasi::io::streams::StreamError::Closed) => break,
                    Err(e) => {
                        tracing::error!("could not read data: {}", e);
                        return Err(1);
                    }
                }
            }

            String::from_utf8(body).unwrap()
        };

        let status: Status0 = serde_json::from_str(&body).unwrap();

        self.send_update(crate::litehouse::plugin::update::Update::OnOff(
            status.status_sts.power == "ON",
        ));
        self.send_update(crate::litehouse::plugin::update::Update::Current(
            status.status_sns.energy.current,
        ));
        self.send_update(crate::litehouse::plugin::update::Update::Voltage(
            status.status_sns.energy.voltage,
        ));
        self.send_update(crate::litehouse::plugin::update::Update::Power(
            status.status_sns.energy.power,
        ));

        tracing::debug!("sending update: {:?}", status);

        self.set_state(status.status_sts.power == "ON");

        Ok(true)
    }
}

impl TasmotaPlugin {
    fn send_update(&self, update: crate::litehouse::plugin::update::Update) {
        host::send_update(&self.nickname, update);
    }
}

#[derive(serde::Deserialize, Debug)]
struct Status0 {
    #[serde(rename = "StatusSNS")]
    status_sns: StatusSNS,
    #[serde(rename = "StatusSTS")]
    status_sts: StatusSTS,
}

#[derive(serde::Deserialize, Debug)]
struct StatusSNS {
    #[serde(rename = "ENERGY")]
    energy: Energy,
}

#[derive(serde::Deserialize, Debug)]
struct StatusSTS {
    #[serde(rename = "POWER")]
    power: String,
}

#[derive(serde::Deserialize, Debug)]
struct Energy {
    #[serde(rename = "Current")]
    current: f64,
    #[serde(rename = "Voltage")]
    voltage: u16,
    #[serde(rename = "Power")]
    power: u16,
}
