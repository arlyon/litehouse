//! Test-only consumer component for the service-fabric integration harness.

wit_bindgen::generate!({
    path: "../litehouse-service-fabric/tests/fixtures/wit",
    world: "consumer-world",
    generate_all,
});

use exports::litehouse::service_fabric_fixture::scenario;
use litehouse::errors::types::{CoreError, RetryAdvice};
use litehouse::service_fabric_fixture::{fabric_v1, fabric_v2, model};

struct Consumer;

struct Session {
    v1: Vec<fabric_v1::Proxy>,
    v2: fabric_v2::Proxy,
}

impl scenario::GuestSession for Session {
    async fn initial(&self) -> Result<model::InitialReport, model::ServiceError> {
        let v1_keys = self.v1.iter().map(fabric_v1::Proxy::key).collect();
        let first_state = self.v1[0].query().await?;
        let second_state = self.v1[1].query().await?;
        let v2_key = self.v2.key();
        let v2_state = self.v2.query().await?;
        let ambiguity = match fabric_v1::bind_one() {
            Err(error) => error.inner,
            Ok(_) => return Err(unexpected("v1 one selector was not ambiguous")),
        };

        let first = self.v1[0].set(true, 1);
        let second = self.v1[1].set(false, 2);
        let (first, second) = futures::join!(first, second);
        let first = first?;
        let second = second?;
        Ok(model::InitialReport {
            v1_keys,
            v1_current: vec![first_state.current, second_state.current],
            v1_provider_generations: vec![
                first_state.provider_generation,
                second_state.provider_generation,
            ],
            v2_key,
            v2_current: v2_state.current,
            v2_provider_generation: v2_state.provider_generation,
            ambiguity,
            completion_tickets: vec![first.ticket, second.ticket],
            completion_order: vec![first.completion_order, second.completion_order],
        })
    }

    async fn after_restart(&self) -> Result<model::RestartReport, model::ServiceError> {
        let stale = match self.v1[0].query().await {
            Err(error) => error.inner,
            Ok(_) => return Err(unexpected("old v1 proxy remained live after restart")),
        };
        let rebound = self.v1[0].rebind()?;
        let rebound_state = rebound.query().await?;
        let never_rebind = match self.v2.rebind() {
            Err(error) => error.inner,
            Ok(_) => return Err(unexpected("never policy created a v2 replacement")),
        };
        Ok(model::RestartReport {
            stale,
            never_rebind,
            rebound_provider_generation: rebound_state.provider_generation,
        })
    }
}

impl scenario::Guest for Consumer {
    type Session = Session;

    fn open() -> Result<scenario::Session, model::ServiceError> {
        let v1 = fabric_v1::bind_many()?;
        let v2 = fabric_v2::bind_one()?;
        Ok(scenario::Session::new(Session { v1, v2 }))
    }
}

fn unexpected(message: &str) -> model::ServiceError {
    model::ServiceError {
        inner: CoreError::ContractError,
        retry: RetryAdvice::Never,
        error_id: format!("fixture:{message}"),
    }
}

export!(Consumer);
