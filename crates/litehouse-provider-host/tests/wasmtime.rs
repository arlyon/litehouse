use litehouse_provider_host::{
    CallContext, FacetValue, MeasurementValue, NotificationPriority, NotificationRequest,
    ProviderConfig, ProviderHostError, ReceiptConfidence, start_provider,
};
use litehouse_service_fabric as fabric;
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
    time::Duration,
};
use wasmtime::component::Component;
use wasmtime::{Config, Engine};

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
        correlation_id: Some("provider-host-test".into()),
        deadline: tokio::time::Instant::now() + Duration::from_secs(5),
        idempotency_key: idempotency_key.map(str::to_owned),
        attempt: 1,
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires LITEHOUSE_PROVIDER_FIXTURE_WASM built for wasm32-wasip2"]
async fn typed_handles_stay_in_actor_while_host_routes_query_set_send_and_withdrawal() {
    let engine = engine();
    let epoch_engine = engine.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(10));
        loop {
            ticker.tick().await;
            epoch_engine.increment_epoch();
        }
    });
    let fixture = std::env::var("LITEHOUSE_PROVIDER_FIXTURE_WASM")
        .expect("fixture component path is required for the ignored integration test");
    let component = Component::from_file(&engine, fixture).unwrap();
    let fabric = Arc::new(Mutex::new(fabric::ServiceFabric::default()));
    let generation = fabric::Generation::new(7).unwrap();
    let handle = start_provider(
        &engine,
        &component,
        ProviderConfig {
            instance: fabric::InstanceId::new("fixture-node").unwrap(),
            generation,
            config_json: "fixture".into(),
            allowed_tcp_addresses: BTreeSet::new(),
            allowed_http_origins: BTreeSet::new(),
            startup_timeout: Duration::from_secs(5),
        },
        fabric.clone(),
    )
    .await
    .unwrap();

    assert_eq!(handle.descriptors().len(), 5);
    assert_eq!(fabric.lock().unwrap().registry().len(), 5);
    let relay = handle
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.endpoint().identity().facet().as_str() == "relay-1")
        .unwrap()
        .endpoint()
        .identity()
        .clone();

    let before = handle
        .query(&relay, generation, context("query-before", None))
        .await
        .unwrap();
    assert_eq!(before.generation, generation);
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
    assert_eq!(receipt.provider_generation, generation);
    assert_eq!(receipt.confidence, ReceiptConfidence::AcceptedForDelivery);

    let after = handle
        .query(&relay, generation, context("query-after", None))
        .await
        .unwrap();
    assert_eq!(after.value, FacetValue::OnOff(Some(true)));
    assert!(after.revision > before.revision);

    let measurement = handle
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.endpoint().identity().facet().as_str() == "temperature-1")
        .unwrap()
        .endpoint()
        .identity()
        .clone();
    let measured = handle
        .query(&measurement, generation, context("query-temperature", None))
        .await
        .unwrap();
    assert_eq!(
        measured.value,
        FacetValue::Measurement(Some(MeasurementValue {
            profile: "temperature".into(),
            magnitude: "21.5".into(),
            unit: "Cel".into(),
            native_magnitude: Some("21.5".into()),
            native_unit: Some("°C".into()),
        }))
    );

    let notification = handle
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.endpoint().identity().facet().as_str() == "notification")
        .unwrap()
        .endpoint()
        .identity()
        .clone();
    let notification_receipt = handle
        .send_notification(
            &notification,
            generation,
            NotificationRequest {
                body: "fixture body".into(),
                title: Some("Fixture".into()),
                priority: Some(NotificationPriority::Normal),
                expires_at_unix_milliseconds: None,
            },
            context("send-notification", None),
        )
        .await
        .unwrap();
    assert_eq!(
        notification_receipt.provider_message_id.as_deref(),
        Some("fixture-message")
    );
    assert_eq!(
        notification_receipt.confidence,
        ReceiptConfidence::AcceptedForDelivery
    );

    let stale = handle
        .query(
            &relay,
            fabric::Generation::new(6).unwrap(),
            context("query-stale", None),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        stale,
        ProviderHostError::Service(fabric::ServiceError {
            inner: fabric::CoreError::Binding(fabric::BindingError::StaleHandle),
            ..
        })
    ));

    drop(handle);
    for _ in 0..20 {
        if fabric.lock().unwrap().registry().is_empty() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("provider endpoints were not withdrawn after actor shutdown");
}
