use crate::{
    exports::litehouse::plugin::plugin::{Every, GuestRunner, Subscription, TimeUnit},
    wasi::http::{
        outgoing_handler,
        types::{Fields, OutgoingRequest, RequestOptions, Scheme},
    },
};
use base64::{engine::general_purpose::STANDARD, Engine};

plugin::generate!(SamsungPlugin, SamsungConfig);

impl std::io::Read for crate::wasi::io::streams::InputStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        tracing::trace!("reading");
        let d =
            crate::wasi::io::streams::InputStream::blocking_read(self, buf.len() as u64).unwrap();

        tracing::trace!("read {:?}", d);

        for x in 0..d.len() {
            buf[x as usize] = d[x as usize];
        }

        Ok(d.len())
    }
}

impl std::io::Write for crate::wasi::io::streams::OutputStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        tracing::trace!("writing");
        let d = crate::wasi::io::streams::OutputStream::check_write(self).map_err(|e| {
            tracing::error!("error checking write: {:?}", e);
            std::io::Error::new(std::io::ErrorKind::Other, "error checking write")
        })?;
        tracing::trace!("writing {} bytes from {:?}", d, buf);
        crate::wasi::io::streams::OutputStream::write(self, &buf[..d as usize]).unwrap();
        Ok(d as usize)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        tracing::trace!("flushing");
        crate::wasi::io::streams::OutputStream::flush(self).unwrap();
        Ok(())
    }
}

struct IoStream {
    input: crate::wasi::io::streams::InputStream,
    output: crate::wasi::io::streams::OutputStream,
}

impl std::io::Read for IoStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(&mut self.input, buf)
    }
}

impl std::io::Write for IoStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        std::io::Write::write(&mut self.output, buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        std::io::Write::flush(&mut self.output)
    }
}

pub struct SamsungPlugin {
    nickname: String,
    ip: String,
    connection_name: String,
    token: String,
}

/// Configuration for the Samsung plugin.
#[derive(plugin::JsonSchema, serde::Deserialize)]
pub struct SamsungConfig {
    /// The IP address of the Samsung TV.
    pub ip: String,
    /// The name to use for this connection.
    pub name: String,
    /// The connection token to use for this connection.
    pub token: String,
}

impl GuestRunner for SamsungPlugin {
    fn new(nickname: String, config: Option<String>) -> Self {
        plugin::tracing_subscriber();

        let SamsungConfig { ip, name, token } =
            serde_json::from_str(&config.unwrap_or_default()).unwrap();
        Self {
            nickname,
            ip,
            connection_name: name,
            token,
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
                exports::litehouse::plugin::plugin::Update::Time(_) => {
                    let headers = Fields::new();

                    let name_b64 = STANDARD.encode(&self.connection_name);

                    let req = OutgoingRequest::new(headers);
                    req.set_path_with_query(Some(&format!(
                        "/api/v2/channels/samsung.remote.control?name={}&token={}",
                        name_b64, self.token
                    )))
                    .expect("ok");
                    req.set_authority(Some(&format!("{}:8001", self.ip)))
                        .unwrap();
                    req.set_scheme(Some(&Scheme::Other("ws".to_string())))
                        .unwrap();

                    let req_body = req.body().unwrap();

                    let x = outgoing_handler::handle(req, Some(RequestOptions::new())).unwrap();

                    x.subscribe().block();
                    let resp = x.get().unwrap().unwrap().unwrap();

                    let status = resp.status();
                    tracing::info!("status {:?}", status);

                    let resp_body = resp.consume().unwrap();

                    let req_stream = req_body.write().unwrap();
                    let resp_stream = resp_body.stream().unwrap();

                    let stream = IoStream {
                        output: req_stream,
                        input: resp_stream,
                    };

                    let mut conn = websocket::sync::client::ClientBuilder::new(&format!(
                        "ws://{}:8001/api/v2/channels/samsung.remote.control?name={}",
                        self.ip, name_b64
                    ))
                    .unwrap()
                    .connect_on(stream)
                    .unwrap();

                    for message in conn.incoming_messages() {
                        tracing::info!("message {:?}", message);
                    }

                    tracing::info!("done");
                }
                _ => {}
            }
        }

        Ok(true)
    }
}
