use std::sync::Arc;

use anyhow::Result;
use itertools::Itertools;
use reqwest::Url;
use reqwest_eventsource::{Event, EventSource};
use tokio::sync::broadcast::Receiver;
use tokio::time::Duration;
use tokio_stream::StreamExt;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

use crate::LogMessage;

// do_signaling exchanges all state of the local PeerConnection and is called
// every time a video is added or removed
async fn do_signaling(
    offer: RTCSessionDescription,
    receiver: Receiver<LogMessage>,
) -> Arc<RTCPeerConnection> {
    let peer_connection = {
        // Create a MediaEngine object to configure the supported codec
        let mut m = MediaEngine::default();

        match m.register_default_codecs() {
            Ok(_) => {}
            Err(err) => panic!("{}", err),
        };

        // Create a InterceptorRegistry. This is the user configurable RTP/RTCP Pipeline.
        // This provides NACKs, RTCP Reports and other features. If you use `webrtc.NewPeerConnection`
        // this is enabled by default. If you are manually managing You MUST create a InterceptorRegistry
        // for each PeerConnection.
        let mut registry = Registry::new();

        // Use the default set of Interceptors
        registry = match register_default_interceptors(registry, &mut m) {
            Ok(r) => r,
            Err(err) => panic!("{}", err),
        };

        // Create the API object with the MediaEngine
        let api = APIBuilder::new()
            .with_media_engine(m)
            .with_interceptor_registry(registry)
            .build();

        // Create a new RTCPeerConnection
        let pc = match api
            .new_peer_connection(RTCConfiguration {
                ice_servers: vec![
                    RTCIceServer {
                        urls: vec!["stun:stun.l.google.com:19302".to_string()],
                        ..Default::default()
                    },
                    RTCIceServer {
                        urls: vec!["stun:stun1.l.google.com:19302".to_string()],
                        ..Default::default()
                    },
                    RTCIceServer {
                        urls: vec!["stun:stun2.l.google.com:19302".to_string()],
                        ..Default::default()
                    },
                    RTCIceServer {
                        urls: vec!["stun:stun3.l.google.com:19302".to_string()],
                        ..Default::default()
                    },
                    RTCIceServer {
                        urls: vec!["stun:stun4.l.google.com:19302".to_string()],
                        ..Default::default()
                    },
                ],

                ..Default::default()
            })
            .await
        {
            Ok(p) => p,
            Err(err) => panic!("{}", err),
        };
        let peer_connection = Arc::new(pc);

        // Set the handler for ICE connection state
        // This will notify you when the peer has connected/disconnected
        peer_connection.on_ice_connection_state_change(Box::new(
            |connection_state: RTCIceConnectionState| {
                tracing::info!("ICE Connection State has changed: {connection_state}");
                Box::pin(async {})
            },
        ));

        let receiver = Arc::new(receiver);

        // Send the current time via a DataChannel to the remote peer every 3 seconds
        peer_connection.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
            let mut r_inner = receiver.resubscribe();
            Box::pin(async move {
                let d2 = Arc::clone(&d);
                d.on_open(Box::new(move || {
                    Box::pin(async move {
                        loop {
                            while let Ok(log_message) = r_inner.recv().await {
                                let log_message = serde_json::to_string(&log_message).unwrap();
                                if let Err(_) = d2.send_text(log_message).await {
                                    break;
                                }
                            }
                        }
                    })
                }));
            })
        }));

        peer_connection
    };

    if let Err(err) = peer_connection.set_remote_description(offer).await {
        panic!("{}", err);
    }

    // Create channel that is blocked until ICE Gathering is complete
    let mut gather_complete = peer_connection.gathering_complete_promise().await;

    // Create an answer
    let answer = match peer_connection.create_answer(None).await {
        Ok(answer) => answer,
        Err(err) => panic!("{}", err),
    };

    // Sets the LocalDescription, and starts our UDP listeners
    if let Err(err) = peer_connection.set_local_description(answer).await {
        panic!("{}", err);
    }

    // Block until ICE Gathering is complete, disabling trickle ICE
    // we do this because we only can exchange one signaling message
    // in a production application you should exchange ICE Candidates via OnICECandidate
    let _ = gather_complete.recv().await;

    peer_connection
}

/// A jwt certificate signed by some authority that can be used to prove
/// the identity of the instance
///
/// Contains the server-assigned node id and account id of the owner
#[derive(Clone, Debug)]
pub struct Authed(pub String);

#[derive(serde::Deserialize)]
struct AuthedClaims {
    node_id: String,
    account: String,
}

impl Authed {
    pub fn node_id(&self) -> &str {
        // parse jwt
        &self.0
    }

    pub fn account(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub enum Credentials {
    Authed(Authed),
    Unauthed { password: [u8; 6] },
}

/// Attempts to establish direct webrtc connections via the provided
/// `broker`. As soon as a connection is brokered, it starts a new
/// task to handle it, and then immediately starts polling again.
pub async fn facilicate_connections(
    broker: Url,
    credentials: Credentials,
    mut logs_rx: Receiver<LogMessage>,
) -> Result<()> {
    // let broker = broker.to_string(); // for some reason the url is not cloneable
    let client = reqwest::Client::new();
    loop {
        let receiver = logs_rx;
        logs_rx = receiver.resubscribe();

        let Some(conn) =
            open_connection(&client, broker.clone(), credentials.clone(), receiver).await
        else {
            continue;
        };

        tokio::task::spawn(async move {
            let _conn = conn;
            tokio::time::sleep(Duration::MAX).await;
        });
    }
}

#[derive(serde::Deserialize)]
struct UnauthConnection {
    offer: RTCSessionDescription,
    seed: String,
    password: String,
}

async fn open_connection(
    client: &reqwest::Client,
    broker: Url,
    credentials: Credentials,
    receiver: Receiver<LogMessage>,
) -> Option<Arc<RTCPeerConnection>> {
    let post = if let Credentials::Authed(cert) = &credentials {
        let mut url = broker.clone();
        url.set_path(&format!("/litehouse/{}", cert.node_id()));
        tracing::debug!("opening broker connection at {}", url);
        client.get(url).bearer_auth(&cert.account())
    } else {
        let mut url = broker.clone();
        url.set_path("/litehouse");
        tracing::debug!("opening broker connection at {}", url);
        client.get(url)
    };

    let mut es = EventSource::new(post).unwrap();
    tracing::trace!("connected to broker, waiting for events");
    let (response, conn) = loop {
        match (&credentials, es.next().await) {
            (_, Some(Ok(Event::Open))) => {
                tracing::trace!("connection opened");
            }
            (Credentials::Authed(cert), Some(Ok(Event::Message(msg)))) => {
                let offer: RTCSessionDescription = serde_json::from_str(&msg.data).unwrap();
                tracing::info!("building answer {}", cert.node_id());
                let conn = do_signaling(offer.clone(), receiver).await;
                let id = msg.id.parse().unwrap();

                let mut url = broker.clone();
                url.set_path(&format!("/litehouse"));
                tracing::info!("sending answer to {}", url);
                break (
                    client
                        .post(url)
                        .bearer_auth(&cert.account())
                        .json(&Finalize {
                            id,
                            offer: conn.local_description().await.unwrap(),
                        })
                        .send()
                        .await,
                    conn,
                );
            }
            (Credentials::Unauthed { password }, Some(Ok(Event::Message(msg)))) => {
                let offer: UnauthConnection = serde_json::from_str(&msg.data).unwrap();
                let password_exp = password.iter().map(|c| c.to_string()).join("");
                let mut url = broker.clone();
                url.set_path(&format!("/litehouse"));
                let id = msg.id.parse().unwrap();

                if offer.password != password_exp {
                    tracing::error!("password mismatch");
                    client
                        .delete(url)
                        .json(&Reject { id })
                        .send()
                        .await
                        .unwrap();
                    return None;
                }

                let conn = do_signaling(offer.offer.clone(), receiver).await;
                tracing::info!("sending answer to {}", url);
                break (
                    client
                        .post(url)
                        .json(&Finalize {
                            id,
                            offer: conn.local_description().await.unwrap(),
                        })
                        .send()
                        .await,
                    conn,
                );
            }
            (_, Some(Err(reqwest_eventsource::Error::StreamEnded)) | None) => return None,
            (_, Some(Err(reqwest_eventsource::Error::Transport(e)))) if e.is_connect() => {
                tracing::trace!("could not connect to broker, retrying in 1 minute");
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
            (_, Some(Err(e))) => {
                tracing::error!("failed to get event: {e:?}");
            }
        }
    };

    tracing::info!("got response");

    let response = response.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    tracing::info!("response: {:?} {}", status, text.trim());
    Some(conn)
}

#[derive(serde::Serialize)]
struct Finalize {
    id: u64,
    offer: RTCSessionDescription,
}

#[derive(serde::Serialize)]
struct Reject {
    id: u64,
}
