use crate::{
    exports::litehouse::plugin::plugin::{Every, GuestRunner, Subscription, TimeUnit, Update},
    wasi::http::{
        outgoing_handler,
        types::{Fields, OutgoingRequest, RequestOptions, Scheme},
    },
};

plugin::generate!(WeatherPlugin, WeatherConfig);

pub struct WeatherPlugin {
    lat: f64,
    lon: f64,
}

#[derive(plugin::JsonSchema, serde::Deserialize)]
pub struct WeatherConfig {
    pub lat: f64,
    pub lon: f64,
}

impl GuestRunner for WeatherPlugin {
    fn new(config: Option<String>) -> Self {
        let config: WeatherConfig = serde_json::from_str(&config.unwrap_or_default()).unwrap();
        Self {
            lat: config.lat,
            lon: config.lon,
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
            match event.inner {
                Update::Time(_) => {
                    let headers = Fields::new();

                    let req = OutgoingRequest::new(headers);
                    req.set_path_with_query(Some(
                        &format!("/v1/forecast?latitude={}&longitude={}&current=temperature_2m,wind_speed_10m", self.lat, self.lon),
                    ))
                    .expect("ok");
                    req.set_authority(Some("api.open-meteo.com"));
                    req.set_scheme(Some(&Scheme::Https));

                    let opts = RequestOptions::new();

                    let x = outgoing_handler::handle(req, Some(opts)).unwrap();

                    x.subscribe().block();
                    let resp = x.get().unwrap();

                    if let Ok(Ok(resp)) = resp {
                        let body = resp.consume().unwrap();
                        let stream = body.stream().unwrap();
                        let data = stream.blocking_read(1024).unwrap();
                        let parsed = serde_json::from_slice::<WeatherResponse>(&data).unwrap();

                        update(Event {
                            id: 0,
                            timestamp: 0,
                            inner: litehouse::plugin::plugin::Update::Temperature(
                                parsed.current.temperature_2m,
                            ),
                        });
                        update(Event {
                            id: 0,
                            timestamp: 0,
                            inner: litehouse::plugin::plugin::Update::WindSpeed(
                                parsed.current.wind_speed_10m,
                            ),
                        });
                    }
                }
                _ => {}
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
