//! Tasmota plugin for the Litehouse home automation system.
//!
//! This crate provides integration with Tasmota devices, allowing for control and monitoring
//! within the Litehouse system.

use std::sync::Mutex;

use wasi::http::{
    client,
    types::{Fields, Request, RequestOptions, Response, Scheme},
};

use crate::{
    exports::litehouse::plugin::plugin::{Every, GuestRunner, Subscription, TimeUnit},
    litehouse::plugin::host,
};

litehouse_plugin::generate!(TasmotaPlugin, TasmotaConfig);

pub struct TasmotaPlugin {
    nickname: String,
    state: Mutex<bool>,
    ip: (u8, u8, u8, u8),
}

#[derive(litehouse_plugin::JsonSchema, litehouse_plugin::Config, serde::Deserialize)]
pub struct TasmotaConfig {
    /// The ip address of the device to connect to.
    #[litehouse(private_local = "omitted")]
    pub ip: (u8, u8, u8, u8),
}

impl TasmotaPlugin {
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

    async fn outputs(&self) -> Result<_rt::Vec<exports::litehouse::plugin::plugin::Output>, u32> {
        Ok(vec![])
    }

    fn subscribe(&self) -> Result<Vec<Subscription>, u32> {
        Ok(vec![Subscription::Time(
            exports::litehouse::plugin::plugin::TimeSubscription::Every(Every {
                amount: 5,
                unit: TimeUnit::Second,
            }),
        )])
    }

    async fn update(
        &self,
        _events: Vec<exports::litehouse::plugin::plugin::Event>,
    ) -> Result<bool, u32> {
        tracing::debug!("UPDATE");
        let body = {
            let headers = Fields::new();
            let (request_trailers, request_trailers_reader) = wit_future::new(|| Ok(None));
            let opts = RequestOptions::new();
            let (req, request_result) =
                Request::new(headers, None, request_trailers_reader, Some(opts));
            if req.set_path_with_query(Some("/cm?cmnd=Status0")).is_err() {
                tracing::error!("unable to set Tasmota request path");
                return Err(1);
            }
            let (a, b, c, d) = self.ip;
            if req
                .set_authority(Some(&format!("{}.{}.{}.{}:80", a, b, c, d)))
                .is_err()
            {
                tracing::error!("unable to set Tasmota request authority");
                return Err(1);
            }
            if req.set_scheme(Some(&Scheme::Http)).is_err() {
                tracing::error!("unable to set Tasmota request scheme");
                return Err(1);
            }
            drop(request_trailers);

            let resp = match client::send(req).await {
                Ok(response) => response,
                Err(error) => {
                    tracing::error!(?error, "unable to send Tasmota request");
                    return Err(1);
                }
            };
            if let Err(error) = request_result.await {
                tracing::error!(?error, "unable to stream Tasmota request");
                return Err(1);
            }

            let (body_result, body_result_reader) = wit_future::new(|| Ok(()));
            let (stream, trailers) = Response::consume_body(resp, body_result_reader);
            let body = stream.collect().await;
            if let Err(error) = trailers.await {
                tracing::error!(?error, "unable to read Tasmota response");
                return Err(1);
            }
            drop(body_result);

            match String::from_utf8(body) {
                Ok(body) => body,
                Err(error) => {
                    tracing::error!(?error, "Tasmota response was not UTF-8");
                    return Err(1);
                }
            }
        };

        let status: Status0 = match serde_json::from_str(&body) {
            Ok(status) => status,
            Err(error) => {
                tracing::error!(?error, "unable to parse Tasmota response");
                return Err(1);
            }
        };

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
