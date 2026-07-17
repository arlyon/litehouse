use esphome_client::{
    EspHomeClient,
    types::{EspHomeMessage, ListEntitiesRequest, SubscribeStatesRequest, SwitchCommandRequest},
};
use esphome_native_api::{
    esphomeapi::EspHomeApi,
    parser::ProtoMessage,
    proto::{
        BinarySensorStateResponse, ListEntitiesBinarySensorResponse, ListEntitiesDoneResponse,
        ListEntitiesSwitchResponse, SwitchStateResponse,
    },
};
use tokio::{net::TcpListener, sync::oneshot, time::Duration};

const FIXTURE_KEY: &str = "AAECAwQFBgcICRAREhMUFRYXGBkgISIjJCUmJygpMDE=";

#[tokio::test]
async fn pinned_noise_stack_discovers_state_and_delivers_absolute_switch_command() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (command_tx, command_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let api = EspHomeApi::builder()
            .name("fixture".into())
            .encryption_key(FIXTURE_KEY.into())
            .api_version_major(1)
            .api_version_minor(14)
            .build();
        let (tx, mut rx) = api.start(stream).await.unwrap();
        let mut command_tx = Some(command_tx);
        while let Ok(message) = rx.recv().await {
            match message {
                ProtoMessage::ListEntitiesRequest(_) => {
                    tx.send(ProtoMessage::ListEntitiesSwitchResponse(switch_entity()))
                        .await
                        .unwrap();
                    tx.send(ProtoMessage::ListEntitiesBinarySensorResponse(
                        binary_entity(),
                    ))
                    .await
                    .unwrap();
                    tx.send(ProtoMessage::ListEntitiesDoneResponse(
                        ListEntitiesDoneResponse {},
                    ))
                    .await
                    .unwrap();
                }
                ProtoMessage::SubscribeStatesRequest(_) => {
                    tx.send(ProtoMessage::SwitchStateResponse(SwitchStateResponse {
                        key: 11,
                        state: false,
                        device_id: 1,
                    }))
                    .await
                    .unwrap();
                    tx.send(ProtoMessage::BinarySensorStateResponse(
                        BinarySensorStateResponse {
                            key: 21,
                            state: true,
                            missing_state: false,
                            device_id: 1,
                        },
                    ))
                    .await
                    .unwrap();
                }
                ProtoMessage::SwitchCommandRequest(command) => {
                    let absolute = command.key == 11 && command.device_id == 1 && command.state;
                    if let Some(command_tx) = command_tx.take() {
                        let _ = command_tx.send(absolute);
                    }
                    tx.send(ProtoMessage::SwitchStateResponse(SwitchStateResponse {
                        key: 11,
                        state: command.state,
                        device_id: command.device_id,
                    }))
                    .await
                    .unwrap();
                }
                _ => {}
            }
        }
    });

    let mut client = EspHomeClient::builder()
        .address(&address.to_string())
        .key(FIXTURE_KEY)
        .client_info("litehouse-esphome-fixture")
        .timeout(Duration::from_secs(2))
        .connect()
        .await
        .unwrap();
    client.try_write(ListEntitiesRequest {}).await.unwrap();
    client.try_write(SubscribeStatesRequest {}).await.unwrap();

    let mut listed_switch = false;
    let mut listed_binary = false;
    let mut initial_switch = false;
    let mut initial_binary = false;
    while !(listed_switch && listed_binary && initial_switch && initial_binary) {
        match tokio::time::timeout(Duration::from_secs(2), client.try_read())
            .await
            .unwrap()
            .unwrap()
        {
            EspHomeMessage::ListEntitiesSwitchResponse(entity) => {
                listed_switch = entity.key == 11 && entity.device_id == 1;
            }
            EspHomeMessage::ListEntitiesBinarySensorResponse(entity) => {
                listed_binary = entity.key == 21 && entity.device_id == 1;
            }
            EspHomeMessage::SwitchStateResponse(state) => {
                initial_switch = state.key == 11 && !state.state;
            }
            EspHomeMessage::BinarySensorStateResponse(state) => {
                initial_binary = state.key == 21 && state.state;
            }
            _ => {}
        }
    }

    client
        .try_write(SwitchCommandRequest {
            key: 11,
            state: true,
            device_id: 1,
        })
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), command_rx)
            .await
            .unwrap()
            .unwrap()
    );
    loop {
        if let EspHomeMessage::SwitchStateResponse(state) =
            tokio::time::timeout(Duration::from_secs(2), client.try_read())
                .await
                .unwrap()
                .unwrap()
            && state.key == 11
            && state.state
        {
            break;
        }
    }
    drop(client);
    server.abort();
}

fn switch_entity() -> ListEntitiesSwitchResponse {
    ListEntitiesSwitchResponse {
        object_id: "relay_1".into(),
        key: 11,
        name: "Relay 1".into(),
        icon: String::new(),
        assumed_state: false,
        disabled_by_default: false,
        entity_category: 0,
        device_class: "switch".into(),
        device_id: 1,
    }
}

fn binary_entity() -> ListEntitiesBinarySensorResponse {
    ListEntitiesBinarySensorResponse {
        object_id: "button_1".into(),
        key: 21,
        name: "Button 1".into(),
        device_class: String::new(),
        is_status_binary_sensor: false,
        disabled_by_default: false,
        icon: String::new(),
        entity_category: 0,
        device_id: 1,
    }
}
