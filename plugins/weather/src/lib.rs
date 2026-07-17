//! Weather plugin for the Litehouse home automation system.
//!
//! This crate provides functionality for fetching and displaying weather information
//! from various sources, allowing for weather-based automation within the Litehouse system.

use exports::litehouse::plugin::plugin::Output;

use crate::{
    exports::litehouse::plugin::plugin::{Every, GuestRunner, Subscription, TimeUnit},
    litehouse::plugin::{host, update::Update},
    wasi::http::{
        client,
        types::{Fields, Request, Response, Scheme},
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

#[derive(litehouse_plugin::JsonSchema, litehouse_plugin::Config, serde::Deserialize)]
pub struct WeatherConfig {
    /// The latitude to fetch the weather for.
    pub lat: f64,
    /// The longitude to fetch the weather for.
    pub lon: f64,
}

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

    async fn outputs(&self) -> Result<Vec<Output>, u32> {
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
        events: Vec<exports::litehouse::plugin::plugin::Event>,
    ) -> Result<bool, u32> {
        tracing::info!("weather update");
        for event in events {
            if let exports::litehouse::plugin::plugin::Update::Time(_) = event.inner {
                let headers = Fields::new();
                let (request_trailers, request_trailers_reader) = wit_future::new(|| Ok(None));
                let (req, request_result) =
                    Request::new(headers, None, request_trailers_reader, None);
                if req.set_path_with_query(Some(&self.request_path)).is_err() {
                    tracing::error!("unable to set weather request path");
                    continue;
                }
                if req.set_authority(Some("api.open-meteo.com")).is_err() {
                    tracing::error!("unable to set weather request authority");
                    continue;
                }
                if req.set_scheme(Some(&Scheme::Https)).is_err() {
                    tracing::error!("unable to set weather request scheme");
                    continue;
                }
                drop(request_trailers);

                let Ok(resp) = client::send(req).await else {
                    tracing::error!("unable to send weather request");
                    continue;
                };
                if let Err(error) = request_result.await {
                    tracing::error!(?error, "unable to stream weather request");
                    continue;
                }

                let (body_result, body_result_reader) = wit_future::new(|| Ok(()));
                let (stream, trailers) = Response::consume_body(resp, body_result_reader);
                let data = stream.collect().await;
                if let Err(error) = trailers.await {
                    tracing::error!(?error, "unable to read weather response");
                    continue;
                }
                drop(body_result);

                let parsed = match serde_json::from_slice::<WeatherResponse>(&data) {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        tracing::error!(?error, "unable to parse weather response");
                        continue;
                    }
                };

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
