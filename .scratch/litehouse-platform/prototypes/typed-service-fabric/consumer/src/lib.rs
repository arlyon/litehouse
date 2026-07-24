wit_bindgen::generate!({
    path: "../wit",
    world: "consumer-world",
    generate_all,
});

use exports::litehouse::service_spike::scenario;
use litehouse::service_spike::{fabric_v1, fabric_v2};
use litehouse::service_spike::model::{Operation, Receipt};

struct Consumer;

impl scenario::Guest for Consumer {
    async fn run() -> Result<Vec<Receipt>, String> {
        let proxies = fabric_v1::bind_many();
        if proxies.len() != 2 {
            return Err(format!("expected two v1 facets, got {}", proxies.len()));
        }

        let first = proxies[0]
            .invoke(Operation::Read, None)
            .await
            .map_err(|error| format!("v1 read failed: {error:?}"))?;
        let second = proxies[1]
            .invoke(Operation::Set, Some(false))
            .await
            .map_err(|error| format!("v1 set failed: {error:?}"))?;

        let v2 = fabric_v2::bind_one().ok_or("missing v2 facet")?;
        let third = v2
            .invoke(Operation::Read, None)
            .await
            .map_err(|error| format!("v2 read failed: {error:?}"))?;

        Ok(vec![first, second, third])
    }
}

export!(Consumer);
