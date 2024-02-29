use crate::{
    exports::litehouse::plugin::plugin::{Every, GuestRunner, Subscription, TimeUnit},
    wasi::http::{
        outgoing_handler,
        types::{Fields, OutgoingRequest, RequestOptions, Scheme},
    },
};
use alloc::borrow::ToOwned;
use core::fmt::Write;
use plugin::wit_bindgen::rt::{string::String, vec::Vec};

#[macro_use]
extern crate alloc;

plugin::generate!(WeatherPlugin, WeatherConfig);

pub struct WeatherPlugin {
    nickname: String,
    request_path: String,
}

#[derive(plugin::JsonSchema, serde::Deserialize)]
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

    fn subscribe(&self) -> Result<Vec<Subscription>, u32> {
        Ok(vec![Subscription::Time(
            exports::litehouse::plugin::plugin::TimeSubscription::Every(Every {
                amount: 1,
                unit: TimeUnit::Minute,
            }),
        )])
    }

    fn update(&self, events: Vec<exports::litehouse::plugin::plugin::Event>) -> Result<bool, u32> {
        for event in events {
            if let exports::litehouse::plugin::plugin::Update::Time(_) = event.inner {
                let headers = Fields::new();

                let req = OutgoingRequest::new(headers);
                req.set_path_with_query(Some(&self.request_path))
                    .expect("ok");
                req.set_authority(Some("api.open-meteo.com")).unwrap();
                req.set_scheme(Some(&Scheme::Https)).unwrap();

                let opts = RequestOptions::new();

                let x = outgoing_handler::handle(req, Some(opts)).unwrap();

                x.subscribe().block();
                let resp = x.get().unwrap();

                if let Ok(Ok(resp)) = resp {
                    let body = resp.consume().unwrap();
                    let stream = body.stream().unwrap();
                    let data = stream.blocking_read(1024).unwrap();
                    let parsed = serde_json::from_slice::<WeatherResponse>(&data).unwrap();

                    send_update(
                        &self.nickname,
                        litehouse::plugin::plugin::Update::Temperature(
                            parsed.current.temperature_2m,
                        ),
                    );
                    send_update(
                        &self.nickname,
                        litehouse::plugin::plugin::Update::WindSpeed(parsed.current.wind_speed_10m),
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
