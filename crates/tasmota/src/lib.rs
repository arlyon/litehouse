use std::sync::Mutex;

use wasi::http::{
    outgoing_handler,
    types::{Fields, OutgoingRequest, RequestOptions, Scheme},
};

use crate::exports::litehouse::plugin::plugin::{Every, GuestRunner, Subscription, TimeUnit};

plugin::generate!(TasmotaPlugin, TasmotaConfig);

pub struct TasmotaPlugin {
    state: Mutex<bool>,
    ip: String,
}

#[derive(plugin::JsonSchema, serde::Deserialize)]
pub struct TasmotaConfig {
    pub ip: String,
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
    fn new(config: Option<String>) -> Self {
        plugin::tracing_subscriber();
        let config: TasmotaConfig = serde_json::from_str(&config.unwrap_or_default()).unwrap();
        Self {
            ip: config.ip,
            state: Mutex::new(false),
        }
    }

    fn subscribe(&self) -> Result<Vec<Subscription>, u32> {
        Ok(vec![Subscription::Time(
            exports::litehouse::plugin::plugin::TimeSubscription::Every(Every {
                amount: 1,
                unit: TimeUnit::Second,
            }),
        )])
    }

    fn update(&self, events: Vec<exports::litehouse::plugin::plugin::Event>) -> Result<bool, u32> {
        let state = self.get_state();

        let headers = Fields::new();

        let req = OutgoingRequest::new(headers);
        req.set_path_with_query(Some(&format!(
            "/cm?cmnd=Power%20{}",
            if state { "OFF" } else { "ON" },
        )))
        .expect("ok");
        req.set_authority(Some(&format!("{}:80", self.ip)));
        req.set_scheme(Some(&Scheme::Http));

        let opts = RequestOptions::new();

        let x = outgoing_handler::handle(req, Some(opts)).unwrap();

        x.subscribe().block();
        let resp = x.get().unwrap().unwrap().unwrap();

        let body = resp
            .consume()
            .unwrap()
            .stream()
            .unwrap()
            .blocking_read(1024)
            .unwrap();

        println!("body: {:?}", String::from_utf8(body).unwrap());

        self.set_state(!state);

        Ok(true)
    }
}
