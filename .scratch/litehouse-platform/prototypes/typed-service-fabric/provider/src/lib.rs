wit_bindgen::generate!({
    path: "../wit",
    world: "provider-world",
    generate_all,
});

use exports::litehouse::service_spike::{provider_v1, provider_v2};
use litehouse::service_spike::model::{Operation, Receipt, ServiceError};
use std::sync::atomic::{AtomicBool, Ordering};

struct Provider;

struct EndpointV1 {
    key: &'static str,
    incarnation: u64,
    current: AtomicBool,
}

impl provider_v1::GuestEndpoint for EndpointV1 {
    fn key(&self) -> String {
        self.key.into()
    }

    fn incarnation(&self) -> u64 {
        self.incarnation
    }

    fn features(&self) -> Vec<String> {
        vec!["absolute-set".into(), "observable".into()]
    }

    async fn invoke(
        &self,
        operation: Operation,
        value: Option<bool>,
    ) -> Result<Receipt, ServiceError> {
        match operation {
            Operation::Read => {}
            Operation::Set => {
                let value = value.ok_or_else(|| ServiceError::InvalidInput("set needs a value".into()))?;
                self.current.store(value, Ordering::Relaxed);
            }
        }
        let current = self.current.load(Ordering::Relaxed);
        Ok(Receipt {
            endpoint: self.key.into(),
            accepted: true,
            current,
            target: (operation == Operation::Set).then_some(current),
        })
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
            }),
            provider_v1::Endpoint::new(EndpointV1 {
                key: "relay-b",
                incarnation: 9,
                current: AtomicBool::new(true),
            }),
        ]
    }
}

struct EndpointV2;

impl provider_v2::GuestEndpoint for EndpointV2 {
    fn key(&self) -> String {
        "future-relay".into()
    }

    fn incarnation(&self) -> u64 {
        1
    }

    async fn invoke(
        &self,
        operation: Operation,
        value: Option<bool>,
    ) -> Result<Receipt, ServiceError> {
        let current = value.unwrap_or(false);
        Ok(Receipt {
            endpoint: "future-relay".into(),
            accepted: true,
            current,
            target: (operation == Operation::Set).then_some(current),
        })
    }
}

impl provider_v2::Guest for Provider {
    type Endpoint = EndpointV2;

    fn discover() -> Vec<provider_v2::Endpoint> {
        vec![provider_v2::Endpoint::new(EndpointV2)]
    }
}

export!(Provider);
