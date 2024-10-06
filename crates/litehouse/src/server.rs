use std::sync::Arc;

use anyhow::Result;
use reqwest::Url;
use reqwest_eventsource::{Event, EventSource};
use tokio::time::Duration;
use tokio_stream::StreamExt;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

// do_signaling exchanges all state of the local PeerConnection and is called
// every time a video is added or removed
async fn do_signaling(offer: RTCSessionDescription) -> RTCSessionDescription {
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
        let pc = match api.new_peer_connection(RTCConfiguration::default()).await {
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

        // Send the current time via a DataChannel to the remote peer every 3 seconds
        peer_connection.on_data_channel(Box::new(|d: Arc<RTCDataChannel>| {
            Box::pin(async move {
                let d2 = Arc::clone(&d);
                d.on_open(Box::new(move || {
                    Box::pin(async move {
                        while d2
                            .send_text(format!("{:?}", tokio::time::Instant::now()))
                            .await
                            .is_ok()
                        {
                            tokio::time::sleep(Duration::from_secs(3)).await;
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

    peer_connection.local_description().await.unwrap()
}

/// Attempts to establish direct webrtc connections via the provided
/// `broker`. As soon as a connection is brokered, it starts a new
/// task to handle it, and then immediately starts polling again.
pub async fn facilicate_connections(broker: Url) -> Result<()> {
    // let broker = broker.to_string(); // for some reason the url is not cloneable
    let client = reqwest::Client::new();
    loop {
        let conn = open_connection(&client, broker.clone()).await;
        tokio::task::spawn(async move {
            // do stuff
        });
    }
}

async fn open_connection(client: &reqwest::Client, broker: Url) {
    let post = client.post(broker.clone()).bearer_auth("1234");

    let mut es = EventSource::new(post).unwrap();
    tracing::trace!("connected to broker, waiting for events");
    let answer = loop {
        match es.next().await {
            Some(Ok(Event::Open)) => {
                tracing::trace!("connection opened");
            }
            Some(Ok(Event::Message(msg))) => {
                let offer: RTCSessionDescription = serde_json::from_str(&msg.data).unwrap();
                let answer = do_signaling(offer.clone()).await;
                tracing::info!("{}: {:?} -> {:?}", msg.id, offer, answer);
                let id = msg.id.parse().unwrap();
                break Some((id, answer));
            }
            Some(Err(reqwest_eventsource::Error::StreamEnded)) | None => break None,
            Some(Err(reqwest_eventsource::Error::Transport(e))) if e.is_connect() => {
                tracing::trace!("could not connect to broker, retrying in 1 minute");
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
            Some(Err(e)) => {
                tracing::error!("failed to get event: {e:?}");
            }
        }
    };

    tracing::info!("answer: {:?}", answer);

    let Some(answer) = answer else {
        return;
    };

    send_answer(client, broker, answer).await;
}

#[derive(serde::Serialize)]
struct Finalize {
    id: u64,
    offer: RTCSessionDescription,
}

async fn send_answer(client: &reqwest::Client, broker: Url, answer: (u64, RTCSessionDescription)) {
    let post = client
        .put(broker)
        .bearer_auth("1234")
        .json(&Finalize {
            id: answer.0,
            offer: answer.1,
        })
        .send()
        .await;

    let response = post.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    tracing::info!("response: {:?} {}", status, text);
}
