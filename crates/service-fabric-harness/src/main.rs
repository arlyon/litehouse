use anyhow::{Context, Result, anyhow, ensure};
use litehouse_contracts::identity::Version;
use litehouse_service_fabric as fabric;
use litehouse_service_fabric::ContractSpec;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex, RwLock,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{Notify, mpsc, oneshot};
use wasmtime::component::{Component, HasSelf, Linker, ResourceAny, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

#[derive(Debug)]
pub struct V1ProxyResource {
    id: fabric::ProxyId,
}

#[derive(Debug)]
pub struct V2ProxyResource {
    id: fabric::ProxyId,
}

const _: [(); std::mem::size_of::<fabric::ProxyId>()] =
    [(); std::mem::size_of::<V1ProxyResource>()];
const _: [(); std::mem::size_of::<fabric::ProxyId>()] =
    [(); std::mem::size_of::<V2ProxyResource>()];

mod provider_component {
    wasmtime::component::bindgen!({
        path: "../litehouse-service-fabric/tests/fixtures/wit",
        world: "provider-world",
        imports: { default: async },
        exports: { default: async },
        require_store_data_send: true,
    });
}

mod consumer_component {
    wasmtime::component::bindgen!({
        path: "../litehouse-service-fabric/tests/fixtures/wit",
        world: "consumer-world",
        imports: { default: async },
        exports: { default: async },
        require_store_data_send: true,
        with: {
            "litehouse:service-fabric-fixture/fabric-v1.proxy": crate::V1ProxyResource,
            "litehouse:service-fabric-fixture/fabric-v2.proxy": crate::V2ProxyResource,
        },
    });
}

#[derive(Debug)]
struct FixtureV1;

#[derive(Debug)]
struct FixtureV2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixtureOperation {
    Query,
    Set,
}

impl fabric::TypedOperation for FixtureOperation {
    fn name(self) -> &'static str {
        match self {
            Self::Query => "query",
            Self::Set => "set",
        }
    }
}

impl fabric::ContractSpec for FixtureV1 {
    type Operation = FixtureOperation;

    fn requirement() -> fabric::ContractRequirement {
        fabric::ContractRequirement::exact(contract(1))
    }
}

impl fabric::ContractSpec for FixtureV2 {
    type Operation = FixtureOperation;

    fn requirement() -> fabric::ContractRequirement {
        fabric::ContractRequirement::exact(contract(2))
    }
}

fn contract(major: u32) -> fabric::ContractId {
    fabric::ContractId::new("litehouse", "fixture-on-off", major)
        .expect("fixture contract is valid")
}

fn instance(value: &str) -> fabric::InstanceId {
    fabric::InstanceId::new(value).expect("fixture instance is valid")
}

fn facet(value: &str) -> fabric::FacetKey {
    fabric::FacetKey::new(value).expect("fixture facet is valid")
}

fn operation(value: &str) -> fabric::OperationId {
    fabric::OperationId::new(value).expect("fixture operation is valid")
}

fn generation(value: u64) -> fabric::Generation {
    fabric::Generation::new(value).expect("fixture generation is non-zero")
}

fn incarnation(value: u64) -> fabric::Incarnation {
    fabric::Incarnation::new(value).expect("fixture incarnation is non-zero")
}

fn grant_revision(value: u64) -> fabric::GrantRevision {
    fabric::GrantRevision::new(value).expect("fixture grant revision is non-zero")
}

#[derive(Default)]
struct CompletionGate {
    second_completed: AtomicBool,
    release_first: Notify,
}

struct ProviderData {
    table: ResourceTable,
    wasi: WasiCtx,
    gate: Arc<CompletionGate>,
}

impl WasiView for ProviderData {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl provider_component::litehouse::service_fabric_fixture::fixture_gate::Host for ProviderData {}
impl provider_component::litehouse::errors::types::Host for ProviderData {}
impl provider_component::litehouse::service_fabric_fixture::model::Host for ProviderData {}

impl
    provider_component::litehouse::service_fabric_fixture::fixture_gate::HostWithStore<ProviderData>
    for HasSelf<ProviderData>
{
    async fn complete(accessor: &wasmtime::component::Accessor<ProviderData, Self>, ticket: u32) {
        if ticket != 1 {
            return;
        }
        let gate = accessor.with(|mut access| access.get().gate.clone());
        while !gate.second_completed.load(Ordering::Acquire) {
            gate.release_first.notified().await;
        }
    }

    async fn after_completion(
        accessor: &wasmtime::component::Accessor<ProviderData, Self>,
        ticket: u32,
    ) {
        let gate = accessor.with(|mut access| access.get().gate.clone());
        if ticket == 2 {
            gate.second_completed.store(true, Ordering::Release);
            gate.release_first.notify_waiters();
        } else if ticket == 1 {
            gate.second_completed.store(false, Ordering::Release);
        }
    }
}

#[derive(Debug)]
struct HostState {
    endpoint: String,
    provider_generation: u64,
    revision: u64,
    current: bool,
}

#[derive(Debug)]
struct HostReceipt {
    endpoint: String,
    ticket: u32,
    provider_generation: u64,
    revision: u64,
    completion_order: u32,
    current: bool,
    target: bool,
}

enum ProviderRequest {
    QueryV1 {
        dispatch: fabric::Dispatch<FixtureV1>,
        reply: oneshot::Sender<Result<HostState>>,
    },
    SetV1 {
        dispatch: fabric::Dispatch<FixtureV1>,
        value: bool,
        ticket: u32,
        reply: oneshot::Sender<Result<HostReceipt>>,
    },
    QueryV2 {
        dispatch: fabric::Dispatch<FixtureV2>,
        reply: oneshot::Sender<Result<HostState>>,
    },
    Trap {
        reply: oneshot::Sender<Result<()>>,
    },
}

#[derive(Clone)]
struct ProviderDispatcher {
    sender: mpsc::Sender<ProviderRequest>,
}

impl ProviderDispatcher {
    async fn query_v1(&self, dispatch: fabric::Dispatch<FixtureV1>) -> Result<HostState> {
        let (reply, result) = oneshot::channel();
        self.sender
            .send(ProviderRequest::QueryV1 { dispatch, reply })
            .await
            .map_err(|_| anyhow!("provider dispatcher stopped"))?;
        result.await.context("provider query reply was dropped")?
    }

    async fn set_v1(
        &self,
        dispatch: fabric::Dispatch<FixtureV1>,
        value: bool,
        ticket: u32,
    ) -> Result<HostReceipt> {
        let (reply, result) = oneshot::channel();
        self.sender
            .send(ProviderRequest::SetV1 {
                dispatch,
                value,
                ticket,
                reply,
            })
            .await
            .map_err(|_| anyhow!("provider dispatcher stopped"))?;
        result.await.context("provider set reply was dropped")?
    }

    async fn query_v2(&self, dispatch: fabric::Dispatch<FixtureV2>) -> Result<HostState> {
        let (reply, result) = oneshot::channel();
        self.sender
            .send(ProviderRequest::QueryV2 { dispatch, reply })
            .await
            .map_err(|_| anyhow!("provider dispatcher stopped"))?;
        result.await.context("provider query reply was dropped")?
    }

    async fn trap(&self) -> Result<()> {
        let (reply, result) = oneshot::channel();
        self.sender
            .send(ProviderRequest::Trap { reply })
            .await
            .map_err(|_| anyhow!("provider dispatcher stopped"))?;
        result.await.context("provider trap reply was dropped")?
    }
}

struct StartedProvider {
    dispatcher: ProviderDispatcher,
    endpoints: Vec<fabric::EndpointDescriptor>,
}

async fn start_provider(
    engine: &Engine,
    path: &Path,
    provider_generation: u64,
) -> Result<StartedProvider> {
    let component = Component::from_file(engine, path)
        .map_err(|error| anyhow!("load provider fixture {}: {error}", path.display()))?;
    let mut linker = Linker::new(engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    provider_component::ProviderWorld::add_to_linker::<_, HasSelf<_>>(&mut linker, |data| data)?;
    let mut store = Store::new(
        engine,
        ProviderData {
            table: ResourceTable::new(),
            wasi: WasiCtxBuilder::new().build(),
            gate: Arc::new(CompletionGate::default()),
        },
    );
    let bindings =
        provider_component::ProviderWorld::instantiate_async(&mut store, &component, &linker)
            .await?;
    let v1 = bindings.litehouse_service_fabric_fixture_provider_v1();
    let v2 = bindings.litehouse_service_fabric_fixture_provider_v2();
    let v1_handles = v1.call_discover(&mut store).await?;
    let v2_handles = v2.call_discover(&mut store).await?;

    let mut endpoints = Vec::new();
    let mut handles = BTreeMap::new();
    for handle in v1_handles {
        let key = v1.endpoint().call_key(&mut store, handle).await?;
        let facet_incarnation = v1.endpoint().call_incarnation(&mut store, handle).await?;
        endpoints.push(endpoint_descriptor(
            &key,
            1,
            provider_generation,
            facet_incarnation,
            ["query", "set"],
        )?);
        handles.insert((1, key), handle);
    }
    for handle in v2_handles {
        let key = v2.endpoint().call_key(&mut store, handle).await?;
        let facet_incarnation = v2.endpoint().call_incarnation(&mut store, handle).await?;
        endpoints.push(endpoint_descriptor(
            &key,
            2,
            provider_generation,
            facet_incarnation,
            ["query"],
        )?);
        handles.insert((2, key), handle);
    }

    let (sender, receiver) = mpsc::channel(16);
    tokio::spawn(run_provider(store, bindings, handles, receiver));
    Ok(StartedProvider {
        dispatcher: ProviderDispatcher { sender },
        endpoints,
    })
}

fn endpoint_descriptor<const N: usize>(
    key: &str,
    major: u32,
    provider_generation: u64,
    facet_incarnation: u64,
    operations: [&str; N],
) -> Result<fabric::EndpointDescriptor> {
    Ok(fabric::EndpointDescriptor::new(
        fabric::EndpointIdentity::new(instance("physical-switch"), facet(key), contract(major)),
        Version {
            major,
            minor: 0,
            patch: 0,
        },
        generation(provider_generation),
        incarnation(facet_incarnation),
        operations.into_iter().map(operation),
        [fabric::FeatureId::new("observable")?],
    )?)
}

async fn run_provider(
    mut store: Store<ProviderData>,
    bindings: provider_component::ProviderWorld,
    handles: BTreeMap<(u32, String), ResourceAny>,
    mut receiver: mpsc::Receiver<ProviderRequest>,
) {
    while let Some(request) = receiver.recv().await {
        match request {
            ProviderRequest::QueryV1 { dispatch, reply } => {
                let result = dispatch_query_v1(&mut store, &bindings, &handles, &dispatch).await;
                let _ = reply.send(result);
            }
            ProviderRequest::QueryV2 { dispatch, reply } => {
                let result = dispatch_query_v2(&mut store, &bindings, &handles, &dispatch).await;
                let _ = reply.send(result);
            }
            ProviderRequest::SetV1 {
                dispatch,
                value,
                ticket,
                reply,
            } => {
                let Some(ProviderRequest::SetV1 {
                    dispatch: second_dispatch,
                    value: second_value,
                    ticket: second_ticket,
                    reply: second_reply,
                }) = receiver.recv().await
                else {
                    let _ = reply.send(Err(anyhow!("concurrent fixture set was not paired")));
                    break;
                };
                let (first, second) = dispatch_set_pair(
                    &mut store,
                    &bindings,
                    &handles,
                    (&dispatch, value, ticket),
                    (&second_dispatch, second_value, second_ticket),
                )
                .await;
                let _ = reply.send(first);
                let _ = second_reply.send(second);
            }
            ProviderRequest::Trap { reply } => {
                let handle = provider_handle(
                    &handles,
                    1,
                    &fabric::EndpointIdentity::new(
                        instance("physical-switch"),
                        facet("relay-a"),
                        contract(1),
                    ),
                );
                let trapped = match handle {
                    Ok(handle) => matches!(
                        store
                            .run_concurrent(async |accessor| {
                                bindings
                                    .litehouse_service_fabric_fixture_provider_v1()
                                    .endpoint()
                                    .call_fail_for_host_test(accessor, handle)
                                    .await
                            })
                            .await,
                        Err(_) | Ok(Err(_))
                    ),
                    Err(_) => false,
                };
                let _ = reply.send(if trapped {
                    Ok(())
                } else {
                    Err(anyhow!("provider fixture did not trap"))
                });
                break;
            }
        }
    }
}

fn provider_handle(
    handles: &BTreeMap<(u32, String), ResourceAny>,
    major: u32,
    endpoint: &fabric::EndpointIdentity,
) -> Result<ResourceAny> {
    handles
        .get(&(major, endpoint.facet().as_str().to_owned()))
        .copied()
        .ok_or_else(|| anyhow!("provider-local endpoint resource is missing"))
}

async fn dispatch_query_v1(
    store: &mut Store<ProviderData>,
    bindings: &provider_component::ProviderWorld,
    handles: &BTreeMap<(u32, String), ResourceAny>,
    dispatch: &fabric::Dispatch<FixtureV1>,
) -> Result<HostState> {
    let handle = provider_handle(handles, 1, dispatch.endpoint())?;
    let state = store
        .run_concurrent(async |accessor| {
            bindings
                .litehouse_service_fabric_fixture_provider_v1()
                .endpoint()
                .call_query(accessor, handle)
                .await
        })
        .await???;
    Ok(HostState {
        endpoint: state.endpoint,
        provider_generation: dispatch.provider_generation().get(),
        revision: state.revision,
        current: state.current,
    })
}

async fn dispatch_query_v2(
    store: &mut Store<ProviderData>,
    bindings: &provider_component::ProviderWorld,
    handles: &BTreeMap<(u32, String), ResourceAny>,
    dispatch: &fabric::Dispatch<FixtureV2>,
) -> Result<HostState> {
    let handle = provider_handle(handles, 2, dispatch.endpoint())?;
    let state = store
        .run_concurrent(async |accessor| {
            bindings
                .litehouse_service_fabric_fixture_provider_v2()
                .endpoint()
                .call_query(accessor, handle)
                .await
        })
        .await???;
    Ok(HostState {
        endpoint: state.endpoint,
        provider_generation: dispatch.provider_generation().get(),
        revision: state.revision,
        current: state.current,
    })
}

async fn dispatch_set_pair(
    store: &mut Store<ProviderData>,
    bindings: &provider_component::ProviderWorld,
    handles: &BTreeMap<(u32, String), ResourceAny>,
    first: (&fabric::Dispatch<FixtureV1>, bool, u32),
    second: (&fabric::Dispatch<FixtureV1>, bool, u32),
) -> (Result<HostReceipt>, Result<HostReceipt>) {
    let run = async {
        let first_handle = provider_handle(handles, 1, first.0.endpoint())?;
        let second_handle = provider_handle(handles, 1, second.0.endpoint())?;
        let results = store
            .run_concurrent(async |accessor| {
                let provider = bindings.litehouse_service_fabric_fixture_provider_v1();
                let endpoint = provider.endpoint();
                tokio::join!(
                    endpoint.call_set(accessor, first_handle, first.1, first.2),
                    endpoint.call_set(accessor, second_handle, second.1, second.2)
                )
            })
            .await?;
        let first_receipt = results.0??;
        let second_receipt = results.1??;
        Ok::<_, anyhow::Error>((
            host_receipt(first_receipt, first.0.provider_generation()),
            host_receipt(second_receipt, second.0.provider_generation()),
        ))
    }
    .await;
    match run {
        Ok((first, second)) => (Ok(first), Ok(second)),
        Err(error) => {
            let message = error.to_string();
            (Err(anyhow!(message.clone())), Err(anyhow!(message)))
        }
    }
}

fn host_receipt(
    receipt: provider_component::litehouse::service_fabric_fixture::model::Receipt,
    provider_generation: fabric::Generation,
) -> HostReceipt {
    HostReceipt {
        endpoint: receipt.endpoint,
        ticket: receipt.ticket,
        provider_generation: provider_generation.get(),
        revision: receipt.revision,
        completion_order: receipt.completion_order,
        current: receipt.current,
        target: receipt.target,
    }
}

struct ConsumerData {
    table: ResourceTable,
    wasi: WasiCtx,
    fabric: Arc<Mutex<fabric::ServiceFabric>>,
    router: Arc<RwLock<ProviderDispatcher>>,
    consumer: fabric::InstanceGeneration,
}

impl WasiView for ConsumerData {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl consumer_component::litehouse::errors::types::Host for ConsumerData {}
impl consumer_component::litehouse::service_fabric_fixture::model::Host for ConsumerData {}

use consumer_component::litehouse::service_fabric_fixture::model as WireModel;

impl consumer_component::litehouse::service_fabric_fixture::fabric_v1::HostProxy for ConsumerData {
    async fn key(&mut self, proxy: wasmtime::component::Resource<V1ProxyResource>) -> String {
        let id = self
            .table
            .get(&proxy)
            .expect("generated v1 proxy belongs to the consumer store")
            .id;
        let mut fabric = self.fabric.lock().expect("service fabric mutex poisoned");
        let proxy = fabric.proxy::<FixtureV1>(id).expect("proxy ID is typed");
        fabric
            .endpoint_for_proxy(proxy)
            .expect("proxy identity remains available")
            .facet()
            .as_str()
            .to_owned()
    }

    async fn rebind(
        &mut self,
        proxy: wasmtime::component::Resource<V1ProxyResource>,
    ) -> Result<wasmtime::component::Resource<V1ProxyResource>, WireModel::ServiceError> {
        let id = self
            .table
            .get(&proxy)
            .expect("generated v1 proxy belongs to the consumer store")
            .id;
        let rebound = {
            let mut fabric = self.fabric.lock().expect("service fabric mutex poisoned");
            let typed = fabric.proxy::<FixtureV1>(id).map_err(wire_error)?;
            fabric.rebind(typed).map_err(wire_error)?
        };
        Ok(self
            .table
            .push(V1ProxyResource { id: rebound.id() })
            .expect("fixture consumer resource bound is respected"))
    }

    async fn drop(
        &mut self,
        proxy: wasmtime::component::Resource<V1ProxyResource>,
    ) -> wasmtime::Result<()> {
        let resource = self.table.delete(proxy)?;
        self.fabric
            .lock()
            .expect("service fabric mutex poisoned")
            .drop_proxy(resource.id);
        Ok(())
    }
}

impl consumer_component::litehouse::service_fabric_fixture::fabric_v1::Host for ConsumerData {
    async fn bind_many(
        &mut self,
    ) -> Result<Vec<wasmtime::component::Resource<V1ProxyResource>>, WireModel::ServiceError> {
        let binding = {
            let mut fabric = self.fabric.lock().expect("service fabric mutex poisoned");
            fabric
                .resolve(
                    fabric::ResolveRequest::<FixtureV1>::new(
                        self.consumer.clone(),
                        fabric::Selector::new(FixtureV1::requirement()),
                        [FixtureOperation::Query, FixtureOperation::Set],
                        fabric::Cardinality::Many,
                        fabric::RebindPolicy::SameIdentity,
                    )
                    .expect("fixture resolve request is valid"),
                )
                .map_err(wire_error)?
        };
        Ok(binding
            .into_proxies()
            .into_iter()
            .map(|proxy| {
                self.table
                    .push(V1ProxyResource { id: proxy.id() })
                    .expect("fixture consumer resource bound is respected")
            })
            .collect())
    }

    async fn bind_one(
        &mut self,
    ) -> Result<wasmtime::component::Resource<V1ProxyResource>, WireModel::ServiceError> {
        let result = self
            .fabric
            .lock()
            .expect("service fabric mutex poisoned")
            .resolve(
                fabric::ResolveRequest::<FixtureV1>::new(
                    self.consumer.clone(),
                    fabric::Selector::new(FixtureV1::requirement()),
                    [FixtureOperation::Query],
                    fabric::Cardinality::One,
                    fabric::RebindPolicy::Never,
                )
                .expect("fixture resolve request is valid"),
            )
            .map_err(wire_error)?;
        let proxy = result.proxies()[0];
        Ok(self
            .table
            .push(V1ProxyResource { id: proxy.id() })
            .expect("fixture consumer resource bound is respected"))
    }
}

impl consumer_component::litehouse::service_fabric_fixture::fabric_v2::HostProxy for ConsumerData {
    async fn key(&mut self, proxy: wasmtime::component::Resource<V2ProxyResource>) -> String {
        let id = self
            .table
            .get(&proxy)
            .expect("generated v2 proxy belongs to the consumer store")
            .id;
        let mut fabric = self.fabric.lock().expect("service fabric mutex poisoned");
        let proxy = fabric.proxy::<FixtureV2>(id).expect("proxy ID is typed");
        fabric
            .endpoint_for_proxy(proxy)
            .expect("proxy identity remains available")
            .facet()
            .as_str()
            .to_owned()
    }

    async fn rebind(
        &mut self,
        proxy: wasmtime::component::Resource<V2ProxyResource>,
    ) -> Result<wasmtime::component::Resource<V2ProxyResource>, WireModel::ServiceError> {
        let id = self
            .table
            .get(&proxy)
            .expect("generated v2 proxy belongs to the consumer store")
            .id;
        let rebound = {
            let mut fabric = self.fabric.lock().expect("service fabric mutex poisoned");
            let typed = fabric.proxy::<FixtureV2>(id).map_err(wire_error)?;
            fabric.rebind(typed).map_err(wire_error)?
        };
        Ok(self
            .table
            .push(V2ProxyResource { id: rebound.id() })
            .expect("fixture consumer resource bound is respected"))
    }

    async fn drop(
        &mut self,
        proxy: wasmtime::component::Resource<V2ProxyResource>,
    ) -> wasmtime::Result<()> {
        let resource = self.table.delete(proxy)?;
        self.fabric
            .lock()
            .expect("service fabric mutex poisoned")
            .drop_proxy(resource.id);
        Ok(())
    }
}

impl consumer_component::litehouse::service_fabric_fixture::fabric_v2::Host for ConsumerData {
    async fn bind_one(
        &mut self,
    ) -> Result<wasmtime::component::Resource<V2ProxyResource>, WireModel::ServiceError> {
        let result = self
            .fabric
            .lock()
            .expect("service fabric mutex poisoned")
            .resolve(
                fabric::ResolveRequest::<FixtureV2>::new(
                    self.consumer.clone(),
                    fabric::Selector::new(FixtureV2::requirement()).facet(facet("future-relay")),
                    [FixtureOperation::Query],
                    fabric::Cardinality::One,
                    fabric::RebindPolicy::Never,
                )
                .expect("fixture resolve request is valid"),
            )
            .map_err(wire_error)?;
        let proxy = result.proxies()[0];
        Ok(self
            .table
            .push(V2ProxyResource { id: proxy.id() })
            .expect("fixture consumer resource bound is respected"))
    }
}

impl
    consumer_component::litehouse::service_fabric_fixture::fabric_v1::HostProxyWithStore<
        ConsumerData,
    > for HasSelf<ConsumerData>
{
    async fn query(
        accessor: &wasmtime::component::Accessor<ConsumerData, Self>,
        proxy: wasmtime::component::Resource<V1ProxyResource>,
    ) -> Result<WireModel::State, WireModel::ServiceError> {
        let (fabric, router, id) = consumer_call_context(accessor, &proxy);
        let dispatch = {
            let mut fabric = fabric.lock().expect("service fabric mutex poisoned");
            let proxy = fabric.proxy::<FixtureV1>(id).map_err(wire_error)?;
            fabric
                .prepare_call(proxy, FixtureOperation::Query)
                .map_err(wire_error)?
        };
        router
            .query_v1(dispatch)
            .await
            .map(wire_state)
            .map_err(|_| wire_protocol_error())
    }

    async fn set(
        accessor: &wasmtime::component::Accessor<ConsumerData, Self>,
        proxy: wasmtime::component::Resource<V1ProxyResource>,
        value: bool,
        ticket: u32,
    ) -> Result<WireModel::Receipt, WireModel::ServiceError> {
        let (fabric, router, id) = consumer_call_context(accessor, &proxy);
        let dispatch = {
            let mut fabric = fabric.lock().expect("service fabric mutex poisoned");
            let proxy = fabric.proxy::<FixtureV1>(id).map_err(wire_error)?;
            fabric
                .prepare_call(proxy, FixtureOperation::Set)
                .map_err(wire_error)?
        };
        router
            .set_v1(dispatch, value, ticket)
            .await
            .map(wire_receipt)
            .map_err(|_| wire_protocol_error())
    }
}

impl
    consumer_component::litehouse::service_fabric_fixture::fabric_v2::HostProxyWithStore<
        ConsumerData,
    > for HasSelf<ConsumerData>
{
    async fn query(
        accessor: &wasmtime::component::Accessor<ConsumerData, Self>,
        proxy: wasmtime::component::Resource<V2ProxyResource>,
    ) -> Result<WireModel::State, WireModel::ServiceError> {
        let (fabric, router, id) = consumer_call_context(accessor, &proxy);
        let dispatch = {
            let mut fabric = fabric.lock().expect("service fabric mutex poisoned");
            let proxy = fabric.proxy::<FixtureV2>(id).map_err(wire_error)?;
            fabric
                .prepare_call(proxy, FixtureOperation::Query)
                .map_err(wire_error)?
        };
        router
            .query_v2(dispatch)
            .await
            .map(wire_state)
            .map_err(|_| wire_protocol_error())
    }
}

fn consumer_call_context<T>(
    accessor: &wasmtime::component::Accessor<ConsumerData, HasSelf<ConsumerData>>,
    proxy: &wasmtime::component::Resource<T>,
) -> (
    Arc<Mutex<fabric::ServiceFabric>>,
    ProviderDispatcher,
    fabric::ProxyId,
)
where
    T: Send + 'static,
{
    accessor.with(|mut access| {
        let data = access.get();
        let id = data
            .table
            .get(proxy)
            .expect("generated proxy belongs to the consumer store");
        let id = if let Some(proxy) = (id as &dyn std::any::Any).downcast_ref::<V1ProxyResource>() {
            proxy.id
        } else if let Some(proxy) = (id as &dyn std::any::Any).downcast_ref::<V2ProxyResource>() {
            proxy.id
        } else {
            unreachable!("only typed service-fabric proxies use this helper")
        };
        (
            data.fabric.clone(),
            data.router
                .read()
                .expect("provider router poisoned")
                .clone(),
            id,
        )
    })
}

fn wire_state(state: HostState) -> WireModel::State {
    WireModel::State {
        endpoint: state.endpoint,
        provider_generation: state.provider_generation,
        revision: state.revision,
        current: state.current,
    }
}

fn wire_receipt(receipt: HostReceipt) -> WireModel::Receipt {
    WireModel::Receipt {
        endpoint: receipt.endpoint,
        ticket: receipt.ticket,
        provider_generation: receipt.provider_generation,
        revision: receipt.revision,
        completion_order: receipt.completion_order,
        current: receipt.current,
        target: receipt.target,
    }
}

fn wire_protocol_error() -> WireModel::ServiceError {
    wire_error(fabric::ServiceError {
        inner: fabric::CoreError::Execution(fabric::ExecutionError::ProtocolFailed),
        retry: fabric::RetryAdvice::Never,
        message: None,
        error_id: "fabric-provider-protocol".into(),
    })
}

fn wire_error(error: fabric::ServiceError) -> WireModel::ServiceError {
    WireModel::ServiceError {
        inner: wire_core_error(error.inner),
        retry: wire_retry(error.retry),
        error_id: error.error_id,
    }
}

fn wire_core_error(
    error: fabric::CoreError,
) -> consumer_component::litehouse::errors::types::CoreError {
    use consumer_component::litehouse::errors::types as wire;
    match error {
        fabric::CoreError::Binding(error) => wire::CoreError::Binding(match error {
            fabric::BindingError::NotFound => wire::BindingError::NotFound,
            fabric::BindingError::Ambiguous => wire::BindingError::Ambiguous,
            fabric::BindingError::StaleHandle => wire::BindingError::StaleHandle,
            fabric::BindingError::EndpointRemoved => wire::BindingError::EndpointRemoved,
        }),
        fabric::CoreError::Authorization(error) => wire::CoreError::Authorization(match error {
            fabric::AuthorizationError::Denied => wire::AuthorizationError::Denied,
            fabric::AuthorizationError::ExpiredGrant => wire::AuthorizationError::ExpiredGrant,
            fabric::AuthorizationError::InsufficientScope => {
                wire::AuthorizationError::InsufficientScope
            }
        }),
        fabric::CoreError::Availability(error) => wire::CoreError::Availability(match error {
            fabric::AvailabilityError::Unavailable => wire::AvailabilityError::Unavailable,
            fabric::AvailabilityError::Reconnecting => wire::AvailabilityError::Reconnecting,
            fabric::AvailabilityError::ShuttingDown => wire::AvailabilityError::ShuttingDown,
        }),
        fabric::CoreError::Timing(error) => wire::CoreError::Timing(match error {
            fabric::TimingError::DeadlineExceeded {
                deadline_monotonic_nanoseconds,
            } => wire::TimingError::DeadlineExceeded(wire::DeadlineExceeded {
                deadline_monotonic_nanoseconds,
            }),
            fabric::TimingError::Cancelled(origin) => {
                wire::TimingError::Cancelled(wire_cancellation_origin(origin))
            }
        }),
        fabric::CoreError::Capacity(fabric::CapacityError::Overloaded {
            retry_after_milliseconds,
        }) => wire::CoreError::Capacity(wire::CapacityError::Overloaded(wire::Overloaded {
            retry_after_milliseconds,
        })),
        fabric::CoreError::Execution(error) => wire::CoreError::Execution(match error {
            fabric::ExecutionError::ProviderTrapped => wire::ExecutionError::ProviderTrapped,
            fabric::ExecutionError::ProtocolFailed => wire::ExecutionError::ProtocolFailed,
            fabric::ExecutionError::Unsupported => wire::ExecutionError::Unsupported,
        }),
        fabric::CoreError::OutcomeUnknown(error) => {
            wire::CoreError::OutcomeUnknown(wire::OutcomeUnknownError {
                cause: match error.cause {
                    fabric::UnknownOutcomeCause::DeadlineExceeded => {
                        wire::UnknownOutcomeCause::DeadlineExceeded
                    }
                    fabric::UnknownOutcomeCause::Cancelled(origin) => {
                        wire::UnknownOutcomeCause::Cancelled(wire_cancellation_origin(origin))
                    }
                    fabric::UnknownOutcomeCause::Disconnected => {
                        wire::UnknownOutcomeCause::Disconnected
                    }
                    fabric::UnknownOutcomeCause::ProviderFailed => {
                        wire::UnknownOutcomeCause::ProviderFailed
                    }
                },
                request_id: error.request_id,
                attempt: error.attempt,
                idempotency_key: error.idempotency_key,
            })
        }
        fabric::CoreError::ContractError => wire::CoreError::ContractError,
    }
}

fn wire_cancellation_origin(
    origin: fabric::CancellationOrigin,
) -> consumer_component::litehouse::errors::types::CancellationOrigin {
    use consumer_component::litehouse::errors::types::CancellationOrigin as Wire;
    match origin {
        fabric::CancellationOrigin::Caller => Wire::Caller,
        fabric::CancellationOrigin::Provider => Wire::Provider,
        fabric::CancellationOrigin::Host => Wire::Host,
        fabric::CancellationOrigin::Shutdown => Wire::Shutdown,
    }
}

fn wire_retry(
    retry: fabric::RetryAdvice,
) -> consumer_component::litehouse::errors::types::RetryAdvice {
    use consumer_component::litehouse::errors::types as wire;
    match retry {
        fabric::RetryAdvice::Never => wire::RetryAdvice::Never,
        fabric::RetryAdvice::Reconnect => wire::RetryAdvice::Reconnect,
        fabric::RetryAdvice::Rebind => wire::RetryAdvice::Rebind,
        fabric::RetryAdvice::RetryAfter(milliseconds) => wire::RetryAdvice::RetryAfter(
            milliseconds.map(|milliseconds| wire::RetryAfter { milliseconds }),
        ),
    }
}

struct ConsumerRuntime {
    store: Store<ConsumerData>,
    bindings: consumer_component::ConsumerWorld,
    session: ResourceAny,
}

impl ConsumerRuntime {
    async fn start(
        engine: &Engine,
        component: &Component,
        fabric: Arc<Mutex<fabric::ServiceFabric>>,
        router: Arc<RwLock<ProviderDispatcher>>,
        consumer: fabric::InstanceGeneration,
    ) -> Result<Self> {
        let mut linker = Linker::new(engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        consumer_component::ConsumerWorld::add_to_linker::<_, HasSelf<_>>(&mut linker, |data| {
            data
        })?;
        let mut store = Store::new(
            engine,
            ConsumerData {
                table: ResourceTable::new(),
                wasi: WasiCtxBuilder::new().build(),
                fabric,
                router,
                consumer,
            },
        );
        let bindings =
            consumer_component::ConsumerWorld::instantiate_async(&mut store, component, &linker)
                .await?;
        let session = bindings
            .litehouse_service_fabric_fixture_scenario()
            .call_open(&mut store)
            .await?
            .map_err(|error| anyhow!("consumer open failed: {error:?}"))?;
        Ok(Self {
            store,
            bindings,
            session,
        })
    }

    async fn initial(&mut self) -> Result<WireModel::InitialReport> {
        let session = self.session;
        self.store
            .run_concurrent(async |accessor| {
                self.bindings
                    .litehouse_service_fabric_fixture_scenario()
                    .session()
                    .call_initial(accessor, session)
                    .await
            })
            .await??
            .map_err(|error| anyhow!("consumer initial scenario failed: {error:?}"))
    }

    async fn after_restart(&mut self) -> Result<WireModel::RestartReport> {
        let session = self.session;
        self.store
            .run_concurrent(async |accessor| {
                self.bindings
                    .litehouse_service_fabric_fixture_scenario()
                    .session()
                    .call_after_restart(accessor, session)
                    .await
            })
            .await??
            .map_err(|error| anyhow!("consumer restart scenario failed: {error:?}"))
    }
}

fn configure_engine() -> Result<Engine> {
    let mut config = Config::new();
    config
        .wasm_component_model(true)
        .wasm_component_model_async(true)
        .concurrency_support(true);
    Ok(Engine::new(&config)?)
}

fn service_grants() -> Vec<fabric::ServiceGrant> {
    vec![
        fabric::ServiceGrant::new(
            contract(1),
            None,
            None,
            fabric::AuthorizationAction::ALL,
            [operation("query"), operation("set")],
        )
        .expect("fixture grant is valid"),
        fabric::ServiceGrant::new(
            contract(2),
            None,
            None,
            fabric::AuthorizationAction::ALL,
            [operation("query")],
        )
        .expect("fixture grant is valid"),
    ]
}

fn activate_consumer(
    fabric: &mut fabric::ServiceFabric,
    name: &str,
) -> Result<fabric::InstanceGeneration> {
    let consumer = fabric::InstanceGeneration::new(instance(name), generation(1));
    fabric.activate_consumer(consumer.clone())?;
    fabric.set_grants(consumer.instance(), grant_revision(1), service_grants())?;
    Ok(consumer)
}

fn assert_initial(report: &WireModel::InitialReport, expected_current: [bool; 2]) -> Result<()> {
    use consumer_component::litehouse::errors::types as wire;
    ensure!(
        report.v1_keys == ["relay-a", "relay-b"],
        "non-canonical v1 ordering: {:?}",
        report.v1_keys
    );
    ensure!(
        report.v1_current == expected_current,
        "v1 typed query was not forwarded: {:?}",
        report.v1_current
    );
    ensure!(report.v1_provider_generations == [1, 1]);
    ensure!(
        report.v2_key == "future-relay",
        "v2 exact-major route failed"
    );
    ensure!(!report.v2_current, "v2 typed query was not forwarded");
    ensure!(report.v2_provider_generation == 1);
    ensure!(
        matches!(
            report.ambiguity,
            wire::CoreError::Binding(wire::BindingError::Ambiguous)
        ),
        "one cardinality did not return typed ambiguity: {:?}",
        report.ambiguity
    );
    ensure!(report.completion_tickets == [1, 2]);
    ensure!(
        report.completion_order.len() == 2
            && report.completion_order[1] < report.completion_order[0],
        "provider calls did not complete out of admission order: {:?}",
        report.completion_order
    );
    Ok(())
}

fn assert_restart(report: &WireModel::RestartReport) -> Result<()> {
    use consumer_component::litehouse::errors::types as wire;
    ensure!(
        matches!(
            report.stale,
            wire::CoreError::Binding(wire::BindingError::StaleHandle)
        ),
        "old proxy did not become stale: {:?}",
        report.stale
    );
    ensure!(
        matches!(
            report.never_rebind,
            wire::CoreError::Binding(wire::BindingError::StaleHandle)
        ),
        "never policy did not reject rebind: {:?}",
        report.never_rebind
    );
    ensure!(report.rebound_provider_generation == 2);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1).map(PathBuf::from);
    let provider_path = arguments
        .next()
        .ok_or_else(|| anyhow!("usage: service-fabric-harness PROVIDER.wasm CONSUMER.wasm"))?;
    let consumer_path = arguments
        .next()
        .ok_or_else(|| anyhow!("usage: service-fabric-harness PROVIDER.wasm CONSUMER.wasm"))?;
    ensure!(arguments.next().is_none(), "unexpected harness argument");

    let engine = configure_engine()?;
    let fabric = Arc::new(Mutex::new(fabric::ServiceFabric::default()));
    let provider_one = fabric::InstanceGeneration::new(instance("physical-switch"), generation(1));
    fabric
        .lock()
        .expect("service fabric mutex poisoned")
        .activate_provider(provider_one.clone())?;
    let first_provider = start_provider(&engine, &provider_path, 1).await?;
    {
        let mut fabric = fabric.lock().expect("service fabric mutex poisoned");
        for endpoint in first_provider.endpoints {
            fabric.publish_endpoint(endpoint)?;
        }
    }
    let router = Arc::new(RwLock::new(first_provider.dispatcher));
    let consumer_component = Component::from_file(&engine, &consumer_path)
        .map_err(|error| anyhow!("load consumer fixture {}: {error}", consumer_path.display()))?;
    let first_consumer = {
        let mut fabric = fabric.lock().expect("service fabric mutex poisoned");
        activate_consumer(&mut fabric, "consumer-a")?
    };
    let second_consumer = {
        let mut fabric = fabric.lock().expect("service fabric mutex poisoned");
        activate_consumer(&mut fabric, "consumer-b")?
    };
    let mut first = ConsumerRuntime::start(
        &engine,
        &consumer_component,
        fabric.clone(),
        router.clone(),
        first_consumer,
    )
    .await?;
    let mut second = ConsumerRuntime::start(
        &engine,
        &consumer_component,
        fabric.clone(),
        router.clone(),
        second_consumer,
    )
    .await?;
    assert_initial(&first.initial().await?, [false, true])?;
    assert_initial(&second.initial().await?, [true, false])?;

    router
        .read()
        .expect("provider router poisoned")
        .clone()
        .trap()
        .await?;
    {
        let mut fabric = fabric.lock().expect("service fabric mutex poisoned");
        fabric.trap_provider(&provider_one)?;
        fabric.activate_provider(fabric::InstanceGeneration::new(
            instance("physical-switch"),
            generation(2),
        ))?;
    }
    let second_provider = start_provider(&engine, &provider_path, 2).await?;
    {
        let mut fabric = fabric.lock().expect("service fabric mutex poisoned");
        for endpoint in second_provider.endpoints {
            fabric.publish_endpoint(endpoint)?;
        }
    }
    *router.write().expect("provider router poisoned") = second_provider.dispatcher;
    assert_restart(&first.after_restart().await?)?;
    assert_restart(&second.after_restart().await?)?;

    println!(
        "service-fabric harness passed: 2 stores, repeated v1 facets, v1/v2 coexistence, typed live forwarding, out-of-order completion, provider trap/restart staleness, same-identity rebind"
    );
    Ok(())
}
