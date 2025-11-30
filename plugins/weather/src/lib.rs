//! Weather plugin for the Litehouse home automation system.
//!
//! This crate provides functionality for fetching and displaying weather information
//! from various sources, allowing for weather-based automation within the Litehouse system.

use exports::litehouse::plugin::plugin::Output;
use host::Update;

use crate::{
    exports::litehouse::plugin::plugin::{Every, GuestRunner, Subscription, TimeUnit},
    wasi::http::{
        outgoing_handler,
        types::{Fields, OutgoingRequest, Scheme},
    },
};
use core::fmt::Write;

#[macro_use]
extern crate alloc;

litehouse_plugin::generate!(WeatherPlugin, WeatherConfig);

pub struct WeatherPlugin {
    nickname: String,
    request_path: String,
}

#[derive(litehouse_plugin::JsonSchema, serde::Deserialize)]
pub struct WeatherConfig {
    /// The latitude to fetch the weather for.
    pub lat: f64,
    /// The longitude to fetch the weather for.
    pub lon: f64,
}

struct MyNotify;
// impl GuestNotify for MyNotify {
//     fn notify(&self, message: String) -> Result<bool, u32> {
//         tracing::info!("got message: {}", message);
//         Ok(true)
//     }

//     fn new() -> Self {
//         MyNotify
//     }
// }

impl GuestRunner for WeatherPlugin {
    fn new(nickname: String, config: Option<String>) -> Self {
        let WeatherConfig { lat, lon } = serde_json::from_str(&config.unwrap_or_default()).unwrap();
        let mut request_path = String::new();
        write!(
            &mut request_path,
            "/v1/forecast?latitude={}&longitude={}&current=temperature_2m,wind_speed_10m",
            lat, lon
        )
        .unwrap();
        Self {
            nickname,
            request_path,
        }
    }

    fn outputs(&self) -> Result<Vec<Output>, u32> {
        // let not = MyNotify::new();
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

    fn update(&self, events: Vec<exports::litehouse::plugin::plugin::Event>) -> Result<bool, u32> {
        tracing::info!("weather update");
        for event in events {
            if let exports::litehouse::plugin::plugin::Update::Time(_) = event.inner {
                let headers = Fields::new();

                let req = OutgoingRequest::new(headers);
                req.set_path_with_query(Some(&self.request_path))
                    .expect("ok");
                req.set_authority(Some("api.open-meteo.com")).unwrap();
                req.set_scheme(Some(&Scheme::Https)).unwrap();

                let Ok(result) = outgoing_handler::handle(req, None) else {
                    tracing::error!("unable to send request");
                    continue;
                };

                result.subscribe().block();
                let resp = result.get().expect("this is called only once");

                if let Ok(Ok(resp)) = resp {
                    let body = resp.consume().unwrap();
                    let stream = body.stream().unwrap();
                    let data = stream.blocking_read(1024).unwrap();
                    let parsed = serde_json::from_slice::<WeatherResponse>(&data).unwrap();

                    host::send_update(
                        &self.nickname,
                        Update::Temperature(parsed.current.temperature_2m),
                    );
                    host::send_update(
                        &self.nickname,
                        Update::WindSpeed(parsed.current.wind_speed_10m),
                    );
                }
            }
        }

        Ok(true)
    }
}

#[derive(serde::Deserialize, Debug)]
struct WeatherResponse {
    current: Current,
}

#[derive(serde::Deserialize, Debug)]
struct Current {
    temperature_2m: f64,
    wind_speed_10m: f64,
}
