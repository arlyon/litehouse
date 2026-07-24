use esphome_client::{
    EspHomeClient,
    types::{EspHomeMessage, ListEntitiesRequest, SubscribeStatesRequest},
};
use esphome_native_api::{
    esphomeapi::EspHomeApi,
    parser::ProtoMessage,
    proto::{
        ListEntitiesDoneResponse, ListEntitiesSensorResponse, ListEntitiesSwitchResponse,
        SensorStateResponse, SwitchStateResponse,
    },
};
use tokio::{net::TcpListener, time::Duration};

const FIXTURE_KEY: &str = "AAECAwQFBgcICRAREhMUFRYXGBkgISIjJCUmJygpMDE=";

#[tokio::test]
async fn current_noise_server_is_readable_by_the_pinned_client() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let api = EspHomeApi::builder()
            .name("fixture".into())
            .encryption_key(FIXTURE_KEY.into())
            .api_version_major(1)
            .api_version_minor(14)
            .build();
        let (tx, mut rx) = api.start(stream).await.unwrap();
        while let Ok(message) = rx.recv().await {
            match message {
                ProtoMessage::ListEntitiesRequest(_) => {
                    tx.send(ProtoMessage::ListEntitiesSwitchResponse(switch_entity()))
                        .await
                        .unwrap();
                    tx.send(ProtoMessage::ListEntitiesSensorResponse(sensor_entity()))
                        .await
                        .unwrap();
                    tx.send(ProtoMessage::ListEntitiesDoneResponse(ListEntitiesDoneResponse {}))
                        .await
                        .unwrap();
                }
                ProtoMessage::SubscribeStatesRequest(_) => {
                    tx.send(ProtoMessage::SwitchStateResponse(SwitchStateResponse {
                        key: 11,
                        state: true,
                        device_id: 1,
                    }))
                    .await
                    .unwrap();
                    tx.send(ProtoMessage::SensorStateResponse(SensorStateResponse {
                        key: 21,
                        state: 21.25,
                        missing_state: false,
                        device_id: 1,
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
        .client_info("litehouse-fixture")
        .timeout(Duration::from_secs(2))
        .connect()
        .await
        .unwrap();
    client.try_write(ListEntitiesRequest {}).await.unwrap();
    client.try_write(SubscribeStatesRequest {}).await.unwrap();

    let mut switch_entity = false;
    let mut sensor_entity = false;
    let mut switch_state = false;
    let mut sensor_state = false;
    let mut done = false;
    while !(done && switch_state && sensor_state) {
        match client.try_read().await.unwrap() {
            EspHomeMessage::ListEntitiesSwitchResponse(entity) => {
                switch_entity = entity.key == 11 && entity.device_id == 1;
            }
            EspHomeMessage::ListEntitiesSensorResponse(entity) => {
                sensor_entity = entity.key == 21
                    && entity.device_id == 1
                    && entity.unit_of_measurement == "°C";
            }
            EspHomeMessage::ListEntitiesDoneResponse(_) => done = true,
            EspHomeMessage::SwitchStateResponse(state) => {
                switch_state = state.key == 11 && state.state;
            }
            EspHomeMessage::SensorStateResponse(state) => {
                sensor_state = state.key == 21 && state.state == 21.25;
            }
            _ => {}
        }
    }
    assert!(switch_entity && sensor_entity && switch_state && sensor_state);
    drop(client);
    server.abort();
}

fn switch_entity() -> ListEntitiesSwitchResponse {
    ListEntitiesSwitchResponse {
        object_id: "relay".into(),
        key: 11,
        name: "Relay".into(),
        icon: String::new(),
        assumed_state: false,
        disabled_by_default: false,
        entity_category: 0,
        device_class: "switch".into(),
        device_id: 1,
    }
}

#[allow(deprecated)]
fn sensor_entity() -> ListEntitiesSensorResponse {
    ListEntitiesSensorResponse {
        object_id: "temperature".into(),
        key: 21,
        name: "Temperature".into(),
        icon: String::new(),
        unit_of_measurement: "°C".into(),
        accuracy_decimals: 2,
        force_update: false,
        device_class: "temperature".into(),
        state_class: 0,
        legacy_last_reset_type: 0,
        disabled_by_default: false,
        entity_category: 0,
        device_id: 1,
    }
}
