//! Samsung plugin for the Litehouse home automation system.
//!
//! This crate provides functionality for controlling Samsung TVs via the network, allowing for integration into the Litehouse system for tasks such as launching apps or controlling volume.

use std::sync::Arc;

use crate::{
    exports::litehouse::plugin::plugin::{Every, GuestRunner, Output, Subscription, TimeUnit},
    wasi::sockets::types::{
        ErrorCode, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, TcpSocket,
    },
};
use base64::{Engine, engine::general_purpose::STANDARD};
use litehouse_plugin::wit_bindgen::rt::async_support::{
    FutureReader, StreamReader, StreamResult, StreamWriter, block_on,
};
use rustls::{ClientConfig, ClientConnection, client::danger::ServerCertVerifier};
use serde::{Deserialize, Serialize};
use tungstenite::Message;
use url::Url;

litehouse_plugin::generate!(SamsungPlugin, SamsungConfig);

fn samsung_websocket_url(
    ip: (u8, u8, u8, u8),
    connection_name: &str,
    token: &str,
) -> Result<Url, url::ParseError> {
    let endpoint = format!(
        "wss://{}.{}.{}.{}:8002/api/v2/channels/samsung.remote.control",
        ip.0, ip.1, ip.2, ip.3
    );
    let mut url = Url::parse(&endpoint)?;
    url.query_pairs_mut()
        .append_pair("name", &STANDARD.encode(connection_name))
        .append_pair("token", token);
    Ok(url)
}

#[derive(Deserialize)]
struct SamsungConnectionResponse {
    event: String,
}

#[derive(Debug)]
enum SamsungConnectionError {
    InvalidJson,
    NotAuthorized,
    UnexpectedFrame,
}

fn validate_connection_response(message: &Message) -> Result<(), SamsungConnectionError> {
    let response: SamsungConnectionResponse = match message {
        Message::Text(payload) => serde_json::from_str(payload),
        Message::Binary(payload) => serde_json::from_slice(payload),
        Message::Ping(_) | Message::Pong(_) | Message::Close(_) | Message::Frame(_) => {
            return Err(SamsungConnectionError::UnexpectedFrame);
        }
    }
    .map_err(|_| SamsungConnectionError::InvalidJson)?;

    if response.event == "ms.channel.connect" {
        Ok(())
    } else {
        Err(SamsungConnectionError::NotAuthorized)
    }
}

struct IoStream {
    input: StreamReader<u8>,
    output: StreamWriter<u8>,
    receive_result: FutureReader<Result<(), ErrorCode>>,
    send_result: FutureReader<Result<(), ErrorCode>>,
    socket: TcpSocket,
}

impl IoStream {
    async fn finish(self) -> Result<(), ErrorCode> {
        let Self {
            input,
            output,
            receive_result,
            send_result,
            socket,
        } = self;

        // Closing the component streams signals both halves of the TCP socket
        // before awaiting their transport completion results.
        drop(output);
        drop(input);
        let send_result = send_result.await;
        let receive_result = receive_result.await;
        drop(socket);

        send_result?;
        receive_result
    }
}

impl std::io::Read for IoStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        tracing::trace!("reading at most {} bytes", buf.len());
        loop {
            let (status, data) = block_on(self.input.read(Vec::with_capacity(buf.len())));
            match status {
                StreamResult::Complete(0) => continue,
                StreamResult::Complete(_) => {
                    tracing::trace!("read {:?}", data);
                    buf[..data.len()].copy_from_slice(&data);
                    return Ok(data.len());
                }
                StreamResult::Dropped => return Ok(0),
                StreamResult::Cancelled => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "WASI receive stream read was cancelled",
                    ));
                }
            }
        }
    }
}

impl std::io::Write for IoStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        tracing::trace!("writing {} bytes from {:?}", buf.len(), buf);
        let remaining = block_on(self.output.write_all(buf.to_vec()));
        let written = buf.len() - remaining.len();
        if written == 0 {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "WASI send stream was dropped",
            ))
        } else {
            Ok(written)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
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

    async fn update(
        &self,
        events: Vec<exports::litehouse::plugin::plugin::Event>,
    ) -> Result<bool, u32> {
        let Some(_) = events.into_iter().find(|event| {
            matches!(
                event.inner,
                exports::litehouse::plugin::plugin::Update::Time(_)
            )
        }) else {
            return Ok(true);
        };

        let websocket_url = match samsung_websocket_url(self.ip, &self.connection_name, &self.token)
        {
            Ok(url) => url,
            Err(error) => {
                tracing::error!(?error, plugin = %self.nickname, "invalid Samsung TV URL");
                return Err(1);
            }
        };

        let socket = match TcpSocket::create(IpAddressFamily::Ipv4) {
            Ok(socket) => socket,
            Err(error) => {
                tracing::error!(?error, plugin = %self.nickname, "unable to create Samsung TV socket");
                return Err(1);
            }
        };
        if let Err(error) = socket
            .connect(IpSocketAddress::Ipv4(Ipv4SocketAddress {
                address: self.ip,
                port: 8002,
            }))
            .await
        {
            tracing::error!(?error, plugin = %self.nickname, "unable to connect to Samsung TV");
            return Err(1);
        }

        let (input, receive_result) = socket.receive();
        let (output, send_stream) = wit_stream::new::<u8>();
        let send_result = socket.send(send_stream);
        let mut io_stream = IoStream {
            input,
            output,
            receive_result,
            send_result,
            socket,
        };

        let config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyServerCertificate))
            .with_no_client_auth();

        let server_name = match format!("{}.{}.{}.{}", self.ip.0, self.ip.1, self.ip.2, self.ip.3)
            .try_into()
        {
            Ok(server_name) => server_name,
            Err(error) => {
                tracing::error!(?error, plugin = %self.nickname, "invalid Samsung TV server name");
                return Err(1);
            }
        };
        let mut connection = match ClientConnection::new(Arc::new(config), server_name) {
            Ok(connection) => connection,
            Err(error) => {
                tracing::error!(?error, plugin = %self.nickname, "unable to create Samsung TLS session");
                return Err(1);
            }
        };
        let tls_stream = rustls::Stream {
            sock: &mut io_stream,
            conn: &mut connection,
        };

        let (mut socket, _) = match tungstenite::client(websocket_url, tls_stream) {
            Ok(connection) => connection,
            Err(error) => {
                tracing::error!(?error, plugin = %self.nickname, "Samsung WebSocket handshake failed");
                return Err(1);
            }
        };

        let message = match socket.read() {
            Ok(message) => message,
            Err(error) => {
                tracing::error!(?error, plugin = %self.nickname, "unable to read Samsung TV response");
                return Err(1);
            }
        };
        if let Err(error) = validate_connection_response(&message) {
            tracing::error!(?error, plugin = %self.nickname, "Samsung TV rejected the session");
            return Err(1);
        }
        tracing::debug!(plugin = %self.nickname, "Samsung TV session established");

        let message = TvMessage::MsChannelEmit {
            params: TvEvent::Launch {
                to: "host".to_string(),
                data: LaunchData {
                    app_id: "MCmYXNxgcu.DisneyPlus".to_string(),
                    action_type: ActionType::NativeLaunch,
                },
            },
        };

        let message = match serde_json::to_string(&message) {
            Ok(message) => message,
            Err(error) => {
                tracing::error!(?error, plugin = %self.nickname, "unable to encode Samsung TV command");
                return Err(1);
            }
        };
        if let Err(error) = socket.send(Message::Text(message)) {
            tracing::error!(?error, plugin = %self.nickname, "unable to send Samsung TV command");
            return Err(1);
        }
        drop(socket);

        if let Err(error) = io_stream.finish().await {
            tracing::error!(?error, plugin = %self.nickname, "Samsung TV transport failed");
            return Err(1);
        }

        Ok(true)
    }

    async fn outputs(&self) -> Result<_rt::Vec<Output>, u32> {
        Ok(vec![])
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

/// Accepts any server certificate because Samsung TVs use self-signed TLS.
///
/// SECURITY: this permits a LAN attacker to impersonate the TV and observe the
/// token-bearing WebSocket request. Keep this explicit until certificate
/// enrollment or pinning is added.
#[derive(Debug)]
struct AcceptAnyServerCertificate;

impl ServerCertVerifier for AcceptAnyServerCertificate {
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

#[cfg(test)]
mod tests {
    use tungstenite::Message;

    use super::{SamsungConnectionError, samsung_websocket_url, validate_connection_response};

    #[test]
    fn websocket_url_uses_remote_control_channel_and_encodes_query() -> Result<(), url::ParseError>
    {
        let url = samsung_websocket_url((127, 0, 0, 1), "smoke", "a&b")?;

        assert_eq!(url.path(), "/api/v2/channels/samsung.remote.control");
        assert_eq!(
            url.as_str(),
            "wss://127.0.0.1:8002/api/v2/channels/samsung.remote.control?name=c21va2U%3D&token=a%26b"
        );
        Ok(())
    }

    #[test]
    fn connection_response_accepts_the_authorized_event() {
        let response = Message::Text(
            r#"{"event":"ms.channel.connect","data":{"token":"test-token"}}"#.to_string(),
        );

        assert!(validate_connection_response(&response).is_ok());
    }

    #[test]
    fn connection_response_rejects_an_unauthorized_event() {
        let response = Message::Text(r#"{"event":"ms.channel.unauthorized"}"#.to_string());

        assert!(matches!(
            validate_connection_response(&response),
            Err(SamsungConnectionError::NotAuthorized)
        ));
    }
}
