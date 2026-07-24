use esphome_native_api::{
    esphomeapi::EspHomeApi,
    parser::ProtoMessage,
    proto::{
        BinarySensorStateResponse, ListEntitiesBinarySensorResponse, ListEntitiesDoneResponse,
        ListEntitiesSwitchResponse, SwitchStateResponse,
    },
};
use litehouse_provider_host::{
    CallContext, FacetValue, ProviderConfig, ReceiptConfidence, start_provider,
};
use litehouse_service_fabric as fabric;
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::net::TcpListener;
use wasmtime::component::Component;
use wasmtime::{Config, Engine};

const FIXTURE_KEY: &str = "AAECAwQFBgcICRAREhMUFRYXGBkgISIjJCUmJygpMDE=";

fn engine() -> Engine {
    let mut config = Config::new();
    config
        .wasm_component_model(true)
        .wasm_component_model_async(true)
        .concurrency_support(true)
        .epoch_interruption(true);
    Engine::new(&config).unwrap()
}

fn context(request_id: &str, idempotency_key: Option<&str>) -> CallContext {
    CallContext {
        request_id: request_id.into(),
        correlation_id: Some("esphome-component-test".into()),
        deadline: tokio::time::Instant::now() + Duration::from_secs(5),
        idempotency_key: idempotency_key.map(str::to_owned),
        attempt: 1,
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires LITEHOUSE_ESPHOME_COMPONENT built for wasm32-wasip2"]
async fn real_component_uses_one_noise_session_for_discovery_query_and_absolute_set() {
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
                    tx.send(ProtoMessage::SwitchStateResponse(SwitchStateResponse {
                        key: command.key,
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

    let engine = engine();
    let epoch_engine = engine.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(10));
        loop {
            ticker.tick().await;
            epoch_engine.increment_epoch();
        }
    });
    let component = Component::from_file(
        &engine,
        std::env::var("LITEHOUSE_ESPHOME_COMPONENT").unwrap(),
    )
    .unwrap();
    let fabric = Arc::new(Mutex::new(fabric::ServiceFabric::default()));
    let generation = fabric::Generation::new(3).unwrap();
    let handle = start_provider(
        &engine,
        &component,
        ProviderConfig {
            instance: fabric::InstanceId::new("esphome-fixture").unwrap(),
            generation,
            config_json: format!(r#"{{"address":"{address}","noise_key":"{FIXTURE_KEY}"}}"#),
            allowed_tcp_addresses: BTreeSet::from([address]),
            allowed_http_origins: BTreeSet::new(),
            startup_timeout: Duration::from_secs(10),
        },
        fabric,
    )
    .await
    .unwrap();
    assert_eq!(handle.descriptors().len(), 2);

    let relay = handle
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.label() == Some("Relay 1"))
        .unwrap()
        .endpoint()
        .identity()
        .clone();
    let before = handle
        .query(&relay, generation, context("query-before", None))
        .await
        .unwrap();
    assert_eq!(before.value, FacetValue::OnOff(Some(false)));

    let receipt = handle
        .set_on_off(
            &relay,
            generation,
            true,
            context("set-relay", Some("0123456789abcdef0123456789abcdef")),
        )
        .await
        .unwrap();
    assert_eq!(receipt.confidence, ReceiptConfidence::AcceptedForDelivery);
    let after = handle
        .query(&relay, generation, context("query-after", None))
        .await
        .unwrap();
    assert_eq!(after.value, FacetValue::OnOff(Some(true)));

    drop(handle);
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
