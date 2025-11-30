//! Samsung plugin for the Litehouse home automation system.
//!
//! This crate provides functionality for controlling Samsung TVs via the network, allowing for integration into the Litehouse system for tasks such as launching apps or controlling volume.

use std::{sync::Arc, time::Duration};

use crate::{
    exports::litehouse::plugin::plugin::{Every, GuestRunner, Output, Subscription, TimeUnit},
    wasi::sockets::{
        instance_network::instance_network,
        network::{self, IpSocketAddress},
        tcp_create_socket::create_tcp_socket,
    },
};
use base64::{Engine, engine::general_purpose::STANDARD};
use rustls::{ClientConfig, ClientConnection, client::danger::ServerCertVerifier};
use serde::Serialize;
use tungstenite::Message;
use url::Url;

litehouse_plugin::generate!(SamsungPlugin, SamsungConfig);

impl std::io::Read for crate::wasi::io::streams::InputStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        tracing::trace!("reading at most {} bytes", buf.len());

        let d = loop {
            tracing::trace!("waiting to read..");
            self.subscribe().block();
            let data = crate::wasi::io::streams::InputStream::read(self, buf.len() as u64).unwrap();
            if !data.is_empty() {
                break data;
            } else {
                std::thread::sleep(Duration::from_millis(100));
            }
        };

        tracing::trace!("read {:?}", d);

        buf[..d.len()].copy_from_slice(&d[..]);

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
        let bytes = (d as usize).min(buf.len());
        tracing::trace!("writing {} bytes from {:?}", bytes, buf);
        crate::wasi::io::streams::OutputStream::write(self, &buf[..bytes]).unwrap();
        Ok(bytes)
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
    ip: (u8, u8, u8, u8),
    connection_name: String,
    token: String,
}

/// Configuration for the Samsung plugin.
#[derive(litehouse_plugin::JsonSchema, serde::Deserialize)]
pub struct SamsungConfig {
    /// The IP address of the Samsung TV.
    pub ip: (u8, u8, u8, u8),
    /// The name to use for this connection.
    pub name: String,
    /// The connection token to use for this connection.
    pub token: String,
}

impl GuestRunner for SamsungPlugin {
    fn new(nickname: String, config: Option<String>) -> Self {
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
        let base_url = format!(
            "wss://{}.{}.{}.{}:8002/api/v2",
            self.ip.0, self.ip.1, self.ip.2, self.ip.3
        );

        let name_b64 = STANDARD.encode(&self.connection_name);

        let remote_url = format!(
            "{}/channels/samsung.remote.control?name={}&token={}",
            base_url, name_b64, self.token
        );
        let _remote_url = Url::parse(&remote_url).unwrap();

        let app_url = format!("{}?name={}", base_url, name_b64);
        let app_url = Url::parse(&app_url).unwrap();

        let network = instance_network();
        let socket = create_tcp_socket(network::IpAddressFamily::Ipv4).unwrap();
        socket
            .start_connect(
                &network,
                IpSocketAddress::Ipv4(network::Ipv4SocketAddress {
                    address: self.ip,
                    port: 8002,
                }),
            )
            .unwrap();

        let (input, output) = loop {
            tracing::trace!("trying to finish connecting..");
            socket.subscribe().block();
            if let Ok((input, output)) = socket.finish_connect() {
                break (input, output);
            }
        };

        let mut stream = IoStream { output, input };

        let config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(Ignore))
            .with_no_client_auth();

        let stream = rustls::Stream {
            sock: &mut stream,
            conn: &mut ClientConnection::new(
                Arc::new(config),
                format!("{}.{}.{}.{}", self.ip.0, self.ip.1, self.ip.2, self.ip.3)
                    .try_into()
                    .unwrap(),
            )
            .unwrap(),
        };

        for event in events {
            if let exports::litehouse::plugin::plugin::Update::Time(_) = event.inner {
                let (mut socket, _) = tungstenite::client(app_url, stream).unwrap();

                loop {
                    let message = socket.read().unwrap();
                    tracing::info!("message {:?}", message);

                    let message = TvMessage::MsChannelEmit {
                        params: TvEvent::Launch {
                            to: "host".to_string(),
                            data: LaunchData {
                                app_id: "MCmYXNxgcu.DisneyPlus".to_string(),
                                action_type: ActionType::NativeLaunch,
                            },
                        },
                    };

                    // let message = TvMessage::MsRemoteControl {
                    //     params: RemoteControlEvent::press_button(RemoteControlButton::VolumeUp),
                    // };

                    let message = serde_json::to_string(&message).unwrap();
                    tracing::info!("sending message {}", message);
                    socket.send(Message::Text(message)).unwrap();
                }
            }
        }

        Ok(true)
    }

    fn outputs(&self) -> Result<_rt::Vec<Output>, u32> {
        todo!()
    }
}

#[derive(Serialize)]
#[serde(tag = "method")]
enum TvMessage {
    #[serde(rename = "ms.channel.emit")]
    MsChannelEmit { params: TvEvent },
    #[serde(rename = "ms.remote.control")]
    MsRemoteControl { params: RemoteControlEvent },
}

#[derive(Serialize)]
#[serde(tag = "event")]
enum TvEvent {
    #[serde(rename = "ed.apps.launch")]
    Launch { to: String, data: LaunchData },
    #[serde(rename = "ed.installedApp.get")]
    InstalledApps { to: String },
}

#[derive(Serialize)]
struct RemoteControlEvent {
    #[serde(rename = "Cmd")]
    cmd: String,
    #[serde(rename = "DataOfCmd")]
    data: RemoteControlButton,
    #[serde(rename = "Option")]
    option: String,
    #[serde(rename = "TypeOfRemote")]
    type_of_remote: String,
}

#[derive(Serialize)]
enum RemoteControlButton {
    #[serde(rename = "KEY_VOLUP")]
    VolumeUp,
    #[serde(rename = "KEY_VOLDOWN")]
    VolumeDown,
}

impl RemoteControlEvent {
    fn press_button(data: RemoteControlButton) -> Self {
        Self {
            cmd: "Click".to_string(),
            data,
            option: "false".to_string(),
            type_of_remote: "SendRemoteKey".to_string(),
        }
    }
}

#[derive(Serialize)]
struct LaunchData {
    #[serde(rename = "appId")]
    app_id: String,
    action_type: ActionType,
}

#[derive(Serialize)]
enum ActionType {
    #[serde(rename = "DEEP_LINK")]
    DeepLink,
    #[serde(rename = "NATIVE_LAUNCH")]
    NativeLaunch,
}

#[derive(Debug)]
struct Ignore;

impl ServerCertVerifier for Ignore {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
        ]
    }
}
