//! Test-only provider component for the service-fabric integration harness.

wit_bindgen::generate!({
    path: "../litehouse-service-fabric/tests/fixtures/wit",
    world: "provider-world",
    generate_all,
});

use exports::litehouse::service_fabric_fixture::{provider_v1, provider_v2};
use litehouse::service_fabric_fixture::{fixture_gate, model};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

static COMPLETION_ORDER: AtomicU32 = AtomicU32::new(0);

struct Provider;

struct EndpointV1 {
    key: &'static str,
    incarnation: u64,
    current: AtomicBool,
    revision: AtomicU64,
}

impl provider_v1::GuestEndpoint for EndpointV1 {
    fn key(&self) -> String {
        self.key.into()
    }

    fn incarnation(&self) -> u64 {
        self.incarnation
    }

    async fn query(&self) -> Result<model::State, model::ServiceError> {
        Ok(model::State {
            endpoint: self.key.into(),
            provider_generation: 0,
            revision: self.revision.load(Ordering::Relaxed),
            current: self.current.load(Ordering::Relaxed),
        })
    }

    async fn set(&self, value: bool, ticket: u32) -> Result<model::Receipt, model::ServiceError> {
        fixture_gate::complete(ticket).await;
        self.current.store(value, Ordering::Relaxed);
        let revision = self.revision.fetch_add(1, Ordering::Relaxed) + 1;
        let completion_order = COMPLETION_ORDER.fetch_add(1, Ordering::Relaxed) + 1;
        fixture_gate::after_completion(ticket).await;
        Ok(model::Receipt {
            endpoint: self.key.into(),
            ticket,
            provider_generation: 0,
            revision,
            completion_order,
            current: value,
            target: value,
        })
    }

    async fn fail_for_host_test(&self) {
        panic!("intentional provider fixture trap")
    }
}

impl provider_v1::Guest for Provider {
    type Endpoint = EndpointV1;

    fn discover() -> Vec<provider_v1::Endpoint> {
        vec![
            provider_v1::Endpoint::new(EndpointV1 {
                key: "relay-a",
                incarnation: 4,
                current: AtomicBool::new(false),
                revision: AtomicU64::new(1),
            }),
            provider_v1::Endpoint::new(EndpointV1 {
                key: "relay-b",
                incarnation: 9,
                current: AtomicBool::new(true),
                revision: AtomicU64::new(1),
            }),
        ]
    }
}

struct EndpointV2 {
    current: AtomicBool,
    revision: AtomicU64,
}

impl provider_v2::GuestEndpoint for EndpointV2 {
    fn key(&self) -> String {
        "future-relay".into()
    }

    fn incarnation(&self) -> u64 {
        1
    }

    async fn query(&self) -> Result<model::State, model::ServiceError> {
        Ok(model::State {
            endpoint: "future-relay".into(),
            provider_generation: 0,
            revision: self.revision.load(Ordering::Relaxed),
            current: self.current.load(Ordering::Relaxed),
        })
    }
}

impl provider_v2::Guest for Provider {
    type Endpoint = EndpointV2;

    fn discover() -> Vec<provider_v2::Endpoint> {
        vec![provider_v2::Endpoint::new(EndpointV2 {
            current: AtomicBool::new(false),
            revision: AtomicU64::new(1),
        })]
    }
}

export!(Provider);
