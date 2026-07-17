use std::{collections::BTreeMap, marker::PhantomData};

#[allow(dead_code)]
mod provider_component {
    wasmtime::component::bindgen!({
        path: "../wit",
        world: "provider-world",
        exports: { default: async },
        require_store_data_send: true,
    });
}

#[allow(dead_code)]
mod consumer_component {
    wasmtime::component::bindgen!({
        path: "../wit",
        world: "consumer-world",
        imports: { default: async },
        exports: { default: async },
        require_store_data_send: true,
    });
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    Read,
    Set,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub endpoint: String,
    pub accepted: bool,
    pub current: bool,
    pub target: Option<bool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceError {
    Denied,
    Stale,
    Unavailable,
    InvalidInput,
}

pub enum V1 {}
pub enum V2 {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proxy<M> {
    endpoint: String,
    provider_instance: String,
    store_generation: u64,
    facet_incarnation: u64,
    allow_read: bool,
    allow_set: bool,
    _major: PhantomData<M>,
}

#[derive(Clone, Debug)]
struct Facet {
    incarnation: u64,
    current: bool,
}

#[derive(Debug)]
pub struct ProviderStore {
    instance: String,
    generation: u64,
    v1: BTreeMap<String, Facet>,
    v2: BTreeMap<String, Facet>,
}

impl ProviderStore {
    pub fn fixture() -> Self {
        Self {
            instance: "physical-switch".into(),
            generation: 7,
            v1: BTreeMap::from([
                ("relay-a".into(), Facet { incarnation: 4, current: false }),
                ("relay-b".into(), Facet { incarnation: 9, current: true }),
            ]),
            v2: BTreeMap::from([(
                "future-relay".into(),
                Facet { incarnation: 1, current: false },
            )]),
        }
    }

    pub fn bind_v1_wildcard(&self, allow_read: bool, allow_set: bool) -> Vec<Proxy<V1>> {
        self.v1
            .iter()
            .map(|(endpoint, facet)| Proxy {
                endpoint: endpoint.clone(),
                provider_instance: self.instance.clone(),
                store_generation: self.generation,
                facet_incarnation: facet.incarnation,
                allow_read,
                allow_set,
                _major: PhantomData,
            })
            .collect()
    }

    pub fn bind_one_v2(&self) -> Option<Proxy<V2>> {
        self.v2.iter().next().map(|(endpoint, facet)| Proxy {
            endpoint: endpoint.clone(),
            provider_instance: self.instance.clone(),
            store_generation: self.generation,
            facet_incarnation: facet.incarnation,
            allow_read: true,
            allow_set: false,
            _major: PhantomData,
        })
    }

    pub fn invoke_v1(
        &mut self,
        proxy: &Proxy<V1>,
        operation: Operation,
        value: Option<bool>,
    ) -> Result<Receipt, ServiceError> {
        self.validate_proxy(proxy, operation)?;
        let facet = self.v1.get_mut(&proxy.endpoint).ok_or(ServiceError::Stale)?;
        apply(&proxy.endpoint, facet, operation, value)
    }

    pub fn invoke_v2(
        &mut self,
        proxy: &Proxy<V2>,
        operation: Operation,
        value: Option<bool>,
    ) -> Result<Receipt, ServiceError> {
        self.validate_proxy(proxy, operation)?;
        let facet = self.v2.get_mut(&proxy.endpoint).ok_or(ServiceError::Stale)?;
        apply(&proxy.endpoint, facet, operation, value)
    }

    fn validate_proxy<M>(
        &self,
        proxy: &Proxy<M>,
        operation: Operation,
    ) -> Result<(), ServiceError> {
        if proxy.provider_instance != self.instance || proxy.store_generation != self.generation {
            return Err(ServiceError::Stale);
        }
        if (operation == Operation::Read && !proxy.allow_read)
            || (operation == Operation::Set && !proxy.allow_set)
        {
            return Err(ServiceError::Denied);
        }
        let incarnation = self
            .v1
            .get(&proxy.endpoint)
            .or_else(|| self.v2.get(&proxy.endpoint))
            .map(|facet| facet.incarnation)
            .ok_or(ServiceError::Stale)?;
        (incarnation == proxy.facet_incarnation)
            .then_some(())
            .ok_or(ServiceError::Stale)
    }

    pub fn add_v1_facet(&mut self, endpoint: &str) {
        self.v1.insert(endpoint.into(), Facet { incarnation: 1, current: false });
    }

    pub fn replace_v1_facet(&mut self, endpoint: &str) {
        if let Some(facet) = self.v1.get_mut(endpoint) {
            facet.incarnation += 1;
        }
    }

    pub fn trap_and_restart(&mut self) {
        self.generation += 1;
    }
}

fn apply(
    endpoint: &str,
    facet: &mut Facet,
    operation: Operation,
    value: Option<bool>,
) -> Result<Receipt, ServiceError> {
    if operation == Operation::Set {
        facet.current = value.ok_or(ServiceError::InvalidInput)?;
    }
    Ok(Receipt {
        endpoint: endpoint.into(),
        accepted: true,
        current: facet.current,
        target: (operation == Operation::Set).then_some(facet.current),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::{mpsc, oneshot};

    #[test]
    fn binding_is_deterministic_and_snapshot_based() {
        let mut provider = ProviderStore::fixture();
        let snapshot = provider.bind_v1_wildcard(true, true);
        assert_eq!(snapshot.iter().map(|proxy| proxy.endpoint.as_str()).collect::<Vec<_>>(), ["relay-a", "relay-b"]);

        provider.add_v1_facet("relay-c");
        assert_eq!(snapshot.len(), 2, "an active binding does not grow implicitly");
        assert_eq!(provider.bind_v1_wildcard(true, true).len(), 3, "the wildcard grant covers explicit re-resolution");
    }

    #[test]
    fn operations_and_exact_majors_are_typed() {
        let mut provider = ProviderStore::fixture();
        let read_only = provider.bind_v1_wildcard(true, false).remove(0);
        assert!(provider.invoke_v1(&read_only, Operation::Read, None).is_ok());
        assert_eq!(provider.invoke_v1(&read_only, Operation::Set, Some(true)), Err(ServiceError::Denied));

        let v2: Proxy<V2> = provider.bind_one_v2().unwrap();
        assert!(provider.invoke_v2(&v2, Operation::Read, None).is_ok());
        // `invoke_v1(&v2, ...)` cannot compile: exact majors are different proxy types.
    }

    #[test]
    fn facet_and_store_staleness_have_different_blast_radii() {
        let mut provider = ProviderStore::fixture();
        let proxies = provider.bind_v1_wildcard(true, true);
        provider.replace_v1_facet("relay-a");
        assert_eq!(provider.invoke_v1(&proxies[0], Operation::Read, None), Err(ServiceError::Stale));
        assert!(provider.invoke_v1(&proxies[1], Operation::Read, None).is_ok());

        provider.trap_and_restart();
        assert_eq!(provider.invoke_v1(&proxies[1], Operation::Read, None), Err(ServiceError::Stale));
    }

    #[tokio::test]
    async fn host_admits_in_order_while_provider_controls_completion() {
        let (tx, mut rx) = mpsc::channel::<(u8, oneshot::Sender<u8>)>(2);
        let provider = tokio::spawn(async move {
            let first = rx.recv().await.unwrap();
            let second = rx.recv().await.unwrap();
            second.1.send(second.0).unwrap();
            first.1.send(first.0).unwrap();
        });

        let (done_one_tx, done_one_rx) = oneshot::channel();
        let (done_two_tx, done_two_rx) = oneshot::channel();
        tx.send((1, done_one_tx)).await.unwrap();
        tx.send((2, done_two_tx)).await.unwrap();

        assert_eq!(done_two_rx.await.unwrap(), 2);
        assert_eq!(done_one_rx.await.unwrap(), 1);
        provider.await.unwrap();
    }
}
