use litehouse_contracts::identity::Version;
use litehouse_service_fabric::*;
use std::mem::size_of;

#[derive(Debug)]
struct OnOffV1;

#[derive(Debug)]
struct OnOffV2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
    Query,
    Set,
}

impl TypedOperation for Operation {
    fn name(self) -> &'static str {
        match self {
            Self::Query => "query",
            Self::Set => "set",
        }
    }
}

impl ContractSpec for OnOffV1 {
    type Operation = Operation;

    fn requirement() -> ContractRequirement {
        ContractRequirement::exact(contract(1))
    }
}

impl ContractSpec for OnOffV2 {
    type Operation = Operation;

    fn requirement() -> ContractRequirement {
        ContractRequirement::exact(contract(2))
    }
}

fn id(value: &str) -> InstanceId {
    InstanceId::new(value).unwrap()
}

fn facet(value: &str) -> FacetKey {
    FacetKey::new(value).unwrap()
}

fn operation(value: &str) -> OperationId {
    OperationId::new(value).unwrap()
}

fn generation(value: u64) -> Generation {
    Generation::new(value).unwrap()
}

fn incarnation(value: u64) -> Incarnation {
    Incarnation::new(value).unwrap()
}

fn revision(value: u64) -> GrantRevision {
    GrantRevision::new(value).unwrap()
}

fn contract(major: u32) -> ContractId {
    ContractId::new("litehouse", "fixture-on-off", major).unwrap()
}

fn endpoint(
    provider: &str,
    key: &str,
    major: u32,
    generation: u64,
    incarnation: u64,
) -> EndpointDescriptor {
    endpoint_with(
        provider,
        key,
        major,
        generation,
        incarnation,
        &["query", "set"],
        &["observable"],
    )
}

fn endpoint_with(
    provider: &str,
    key: &str,
    major: u32,
    generation: u64,
    incarnation: u64,
    operations: &[&str],
    features: &[&str],
) -> EndpointDescriptor {
    EndpointDescriptor::new(
        EndpointIdentity::new(id(provider), facet(key), contract(major)),
        Version {
            major,
            minor: 0,
            patch: 0,
        },
        self::generation(generation),
        self::incarnation(incarnation),
        operations.iter().map(|name| operation(name)),
        features.iter().map(|name| FeatureId::new(*name).unwrap()),
    )
    .unwrap()
}

fn consumer(name: &str, generation: u64) -> InstanceGeneration {
    InstanceGeneration::new(id(name), self::generation(generation))
}

fn provider(name: &str, generation: u64) -> InstanceGeneration {
    InstanceGeneration::new(id(name), self::generation(generation))
}

fn wildcard_grant(major: u32, operations: &[&str]) -> ServiceGrant {
    ServiceGrant::new(
        contract(major),
        None,
        None,
        AuthorizationAction::ALL,
        operations
            .iter()
            .map(|operation_name| operation(operation_name)),
    )
    .unwrap()
}

fn exact_grant(provider: &str, key: &str, major: u32, operations: &[&str]) -> ServiceGrant {
    ServiceGrant::new(
        contract(major),
        Some(id(provider)),
        Some(facet(key)),
        AuthorizationAction::ALL,
        operations
            .iter()
            .map(|operation_name| operation(operation_name)),
    )
    .unwrap()
}

fn activate_consumer(
    fabric: &mut ServiceFabric,
    name: &str,
    grants: Vec<ServiceGrant>,
) -> InstanceGeneration {
    let consumer = consumer(name, 1);
    fabric.activate_consumer(consumer.clone()).unwrap();
    fabric
        .set_grants(consumer.instance(), revision(1), grants)
        .unwrap();
    consumer
}

fn resolve<C: ContractSpec<Operation = Operation>>(
    fabric: &mut ServiceFabric,
    consumer: InstanceGeneration,
    selector: Selector,
    cardinality: Cardinality,
    rebind: RebindPolicy,
    operations: &[Operation],
) -> Result<Binding<C>, ServiceError> {
    fabric.resolve(
        ResolveRequest::<C>::new(
            consumer,
            selector,
            operations.iter().copied(),
            cardinality,
            rebind,
        )
        .unwrap(),
    )
}

fn assert_core(error: ServiceError, expected: CoreError) {
    assert_eq!(error.inner, expected, "unexpected error: {error}");
    assert!(error.message.is_none());
    assert!(error.error_id.starts_with("fabric-"));
}

#[test]
fn repeated_facets_exact_majors_and_canonical_cardinality_are_deterministic() {
    let mut fabric = ServiceFabric::default();
    fabric.activate_provider(provider("provider-z", 1)).unwrap();
    fabric.activate_provider(provider("provider-a", 1)).unwrap();
    for descriptor in [
        endpoint("provider-z", "relay-b", 1, 1, 9),
        endpoint("provider-a", "relay-c", 1, 1, 1),
        endpoint("provider-z", "relay-a", 1, 1, 4),
        endpoint("provider-z", "relay-a", 2, 1, 1),
    ] {
        fabric.publish_endpoint(descriptor).unwrap();
    }
    let consumer = activate_consumer(
        &mut fabric,
        "consumer",
        vec![wildcard_grant(1, &["query"]), wildcard_grant(2, &["query"])],
    );

    let many = resolve::<OnOffV1>(
        &mut fabric,
        consumer.clone(),
        Selector::new(OnOffV1::requirement()),
        Cardinality::Many,
        RebindPolicy::SameIdentity,
        &[Operation::Query],
    )
    .unwrap();
    let identities = many
        .proxies()
        .iter()
        .map(|proxy| {
            let endpoint = fabric.endpoint_for_proxy(*proxy).unwrap();
            format!("{}/{}", endpoint.provider(), endpoint.facet())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        identities,
        [
            "provider-a/relay-c",
            "provider-z/relay-a",
            "provider-z/relay-b"
        ]
    );

    assert_core(
        resolve::<OnOffV1>(
            &mut fabric,
            consumer.clone(),
            Selector::new(OnOffV1::requirement()),
            Cardinality::One,
            RebindPolicy::Never,
            &[Operation::Query],
        )
        .unwrap_err(),
        CoreError::Binding(BindingError::Ambiguous),
    );
    assert_core(
        resolve::<OnOffV1>(
            &mut fabric,
            consumer.clone(),
            Selector::new(OnOffV1::requirement()),
            Cardinality::Optional,
            RebindPolicy::Never,
            &[Operation::Query],
        )
        .unwrap_err(),
        CoreError::Binding(BindingError::Ambiguous),
    );

    let v2 = resolve::<OnOffV2>(
        &mut fabric,
        consumer,
        Selector::new(OnOffV2::requirement()).facet(facet("relay-a")),
        Cardinality::One,
        RebindPolicy::Never,
        &[Operation::Query],
    )
    .unwrap();
    assert_eq!(v2.proxies().len(), 1);
    assert_eq!(
        fabric
            .endpoint_for_proxy(v2.proxies()[0])
            .unwrap()
            .contract()
            .major(),
        2
    );
}

#[test]
fn optional_not_found_is_empty_but_one_is_typed_not_found() {
    let mut fabric = ServiceFabric::default();
    let consumer = activate_consumer(&mut fabric, "consumer", vec![wildcard_grant(1, &["query"])]);
    let selector = Selector::new(OnOffV1::requirement()).facet(facet("missing"));
    for cardinality in [Cardinality::Optional, Cardinality::Many] {
        for _ in 0..128 {
            let empty = resolve::<OnOffV1>(
                &mut fabric,
                consumer.clone(),
                selector.clone(),
                cardinality,
                RebindPolicy::Never,
                &[Operation::Query],
            )
            .unwrap();
            assert!(empty.proxies().is_empty());
            assert_eq!(empty.id(), None);
        }
    }
    assert_eq!(fabric.active_binding_count(), 0);
    assert_core(
        resolve::<OnOffV1>(
            &mut fabric,
            consumer,
            selector,
            Cardinality::One,
            RebindPolicy::Never,
            &[Operation::Query],
        )
        .unwrap_err(),
        CoreError::Binding(BindingError::NotFound),
    );
}

#[test]
fn independent_consumers_receive_distinct_host_ids_and_authority() {
    let mut fabric = ServiceFabric::default();
    fabric.activate_provider(provider("provider", 1)).unwrap();
    fabric
        .publish_endpoint(endpoint("provider", "relay", 1, 1, 1))
        .unwrap();
    let reader = activate_consumer(
        &mut fabric,
        "reader",
        vec![exact_grant("provider", "relay", 1, &["query"])],
    );
    let operator = activate_consumer(
        &mut fabric,
        "operator",
        vec![exact_grant("provider", "relay", 1, &["query", "set"])],
    );
    let selector = Selector::new(OnOffV1::requirement())
        .provider(id("provider"))
        .facet(facet("relay"));
    let reader_proxy = resolve::<OnOffV1>(
        &mut fabric,
        reader,
        selector.clone(),
        Cardinality::One,
        RebindPolicy::Never,
        &[Operation::Query],
    )
    .unwrap()
    .proxies()[0];
    let operator_proxy = resolve::<OnOffV1>(
        &mut fabric,
        operator,
        selector,
        Cardinality::One,
        RebindPolicy::Never,
        &[Operation::Query, Operation::Set],
    )
    .unwrap()
    .proxies()[0];
    assert_ne!(reader_proxy.id(), operator_proxy.id());
    assert!(fabric.prepare_call(reader_proxy, Operation::Query).is_ok());
    assert_core(
        fabric
            .prepare_call(reader_proxy, Operation::Set)
            .unwrap_err(),
        CoreError::Authorization(AuthorizationError::InsufficientScope),
    );
    assert!(fabric.prepare_call(operator_proxy, Operation::Set).is_ok());
}

#[test]
fn public_authorizer_enforces_principal_and_action_and_defaults_to_deny() {
    let authorizer = ServiceAuthorizer;
    let principal = id("consumer");
    let other_principal = id("other-consumer");
    let endpoint = endpoint("provider", "relay", 1, 1, 1).identity().clone();
    let query = operation("query");
    let grants = GrantSet::new(
        principal.clone(),
        revision(1),
        vec![
            ServiceGrant::new(
                contract(1),
                None,
                None,
                [AuthorizationAction::Invoke],
                [query.clone()],
            )
            .unwrap(),
        ],
    );

    let request = AuthorizationRequest {
        principal: &principal,
        action: AuthorizationAction::Invoke,
        endpoint: &endpoint,
        operation: &query,
        grant_revision: revision(1),
    };
    assert_eq!(
        authorizer.authorize(None, &request),
        Err(AuthorizationError::Denied)
    );
    assert!(authorizer.authorize(Some(&grants), &request).is_ok());

    let wrong_principal = AuthorizationRequest {
        principal: &other_principal,
        ..request
    };
    assert_eq!(
        authorizer.authorize(Some(&grants), &wrong_principal),
        Err(AuthorizationError::Denied)
    );
    let wrong_action = AuthorizationRequest {
        principal: &principal,
        action: AuthorizationAction::Bind,
        ..request
    };
    assert_eq!(
        authorizer.authorize(Some(&grants), &wrong_action),
        Err(AuthorizationError::Denied)
    );
}

#[test]
fn wildcard_grants_require_explicit_reresolution_and_grant_changes_fence_proxies() {
    let mut fabric = ServiceFabric::default();
    fabric.activate_provider(provider("provider", 1)).unwrap();
    fabric
        .publish_endpoint(endpoint("provider", "relay-a", 1, 1, 1))
        .unwrap();
    let consumer = activate_consumer(&mut fabric, "consumer", vec![wildcard_grant(1, &["query"])]);
    let first = resolve::<OnOffV1>(
        &mut fabric,
        consumer.clone(),
        Selector::new(OnOffV1::requirement()),
        Cardinality::Many,
        RebindPolicy::SameIdentity,
        &[Operation::Query],
    )
    .unwrap();
    fabric
        .publish_endpoint(endpoint("provider", "relay-b", 1, 1, 1))
        .unwrap();
    assert_eq!(first.proxies().len(), 1, "bindings are immutable snapshots");
    let second = resolve::<OnOffV1>(
        &mut fabric,
        consumer.clone(),
        Selector::new(OnOffV1::requirement()),
        Cardinality::Many,
        RebindPolicy::SameIdentity,
        &[Operation::Query],
    )
    .unwrap();
    assert_eq!(second.proxies().len(), 2);

    fabric
        .set_grants(
            consumer.instance(),
            revision(2),
            vec![wildcard_grant(1, &["query"])],
        )
        .unwrap();
    assert_core(
        fabric
            .prepare_call(first.proxies()[0], Operation::Query)
            .unwrap_err(),
        CoreError::Authorization(AuthorizationError::ExpiredGrant),
    );
    let rebound = fabric.rebind(first.proxies()[0]).unwrap();
    assert!(fabric.prepare_call(rebound, Operation::Query).is_ok());
}

#[test]
fn facet_incarnation_and_provider_generation_have_distinct_stale_blast_radii() {
    let mut fabric = ServiceFabric::default();
    let provider_one = provider("provider", 1);
    fabric.activate_provider(provider_one.clone()).unwrap();
    fabric
        .publish_endpoint(endpoint("provider", "relay-a", 1, 1, 1))
        .unwrap();
    fabric
        .publish_endpoint(endpoint("provider", "relay-b", 1, 1, 1))
        .unwrap();
    let consumer = activate_consumer(&mut fabric, "consumer", vec![wildcard_grant(1, &["query"])]);
    let binding = resolve::<OnOffV1>(
        &mut fabric,
        consumer,
        Selector::new(OnOffV1::requirement()),
        Cardinality::Many,
        RebindPolicy::SameIdentity,
        &[Operation::Query],
    )
    .unwrap();
    let first = binding.proxies()[0];
    let second = binding.proxies()[1];

    fabric
        .publish_endpoint(endpoint("provider", "relay-a", 1, 1, 2))
        .unwrap();
    assert_core(
        fabric.prepare_call(first, Operation::Query).unwrap_err(),
        CoreError::Binding(BindingError::StaleHandle),
    );
    assert!(fabric.prepare_call(second, Operation::Query).is_ok());

    fabric.trap_provider(&provider_one).unwrap();
    assert_core(
        fabric.prepare_call(second, Operation::Query).unwrap_err(),
        CoreError::Execution(ExecutionError::ProviderTrapped),
    );
    fabric.activate_provider(provider("provider", 2)).unwrap();
    fabric
        .publish_endpoint(endpoint("provider", "relay-a", 1, 2, 1))
        .unwrap();
    fabric
        .publish_endpoint(endpoint("provider", "relay-b", 1, 2, 1))
        .unwrap();
    assert_core(
        fabric.prepare_call(second, Operation::Query).unwrap_err(),
        CoreError::Binding(BindingError::StaleHandle),
    );
}

#[test]
fn rebinding_never_retargets_and_same_identity_rechecks_current_grant() {
    let mut fabric = ServiceFabric::default();
    let provider_one = provider("provider", 1);
    fabric.activate_provider(provider_one.clone()).unwrap();
    fabric
        .publish_endpoint(endpoint("provider", "relay", 1, 1, 1))
        .unwrap();
    fabric
        .activate_provider(provider("alternative", 1))
        .unwrap();
    fabric
        .publish_endpoint(endpoint("alternative", "relay", 1, 1, 1))
        .unwrap();
    let consumer = activate_consumer(&mut fabric, "consumer", vec![wildcard_grant(1, &["query"])]);
    let selector = Selector::new(OnOffV1::requirement())
        .provider(id("provider"))
        .facet(facet("relay"));
    let never = resolve::<OnOffV1>(
        &mut fabric,
        consumer.clone(),
        selector.clone(),
        Cardinality::One,
        RebindPolicy::Never,
        &[Operation::Query],
    )
    .unwrap()
    .proxies()[0];
    let same = resolve::<OnOffV1>(
        &mut fabric,
        consumer.clone(),
        selector,
        Cardinality::One,
        RebindPolicy::SameIdentity,
        &[Operation::Query],
    )
    .unwrap()
    .proxies()[0];

    fabric.trap_provider(&provider_one).unwrap();
    fabric.activate_provider(provider("provider", 2)).unwrap();
    fabric
        .publish_endpoint(endpoint("provider", "relay", 1, 2, 1))
        .unwrap();
    assert_core(
        fabric.rebind(never).unwrap_err(),
        CoreError::Binding(BindingError::StaleHandle),
    );
    let rebound = fabric.rebind(same).unwrap();
    let endpoint = fabric.endpoint_for_proxy(rebound).unwrap();
    assert_eq!(endpoint.provider().as_str(), "provider");
    assert_eq!(endpoint.facet().as_str(), "relay");

    assert!(
        fabric
            .withdraw_endpoint(&provider("provider", 2), &endpoint, incarnation(1))
            .unwrap()
    );
    assert_core(
        fabric.rebind(rebound).unwrap_err(),
        CoreError::Binding(BindingError::EndpointRemoved),
    );
    assert_eq!(
        fabric.registry().len(),
        1,
        "the matching alternative is never selected"
    );

    fabric
        .set_grants(
            consumer.instance(),
            revision(2),
            vec![exact_grant("alternative", "relay", 1, &["query"])],
        )
        .unwrap();
    assert_core(
        fabric.rebind(same).unwrap_err(),
        CoreError::Binding(BindingError::EndpointRemoved),
    );
}

#[test]
fn rebind_rejects_an_endpoint_that_lost_a_required_feature() {
    let mut fabric = ServiceFabric::default();
    fabric.activate_provider(provider("provider", 1)).unwrap();
    fabric
        .publish_endpoint(endpoint("provider", "relay", 1, 1, 1))
        .unwrap();
    let consumer = activate_consumer(&mut fabric, "consumer", vec![wildcard_grant(1, &["query"])]);
    let proxy = resolve::<OnOffV1>(
        &mut fabric,
        consumer,
        Selector::new(OnOffV1::requirement())
            .provider(id("provider"))
            .facet(facet("relay"))
            .require_feature(FeatureId::new("observable").unwrap()),
        Cardinality::One,
        RebindPolicy::SameIdentity,
        &[Operation::Query],
    )
    .unwrap()
    .proxies()[0];

    fabric
        .publish_endpoint(endpoint_with(
            "provider",
            "relay",
            1,
            1,
            2,
            &["query", "set"],
            &[],
        ))
        .unwrap();
    assert_core(
        fabric.rebind(proxy).unwrap_err(),
        CoreError::Binding(BindingError::EndpointRemoved),
    );
}

#[test]
fn rebind_rejects_an_endpoint_that_lost_a_bound_operation() {
    let mut fabric = ServiceFabric::default();
    fabric.activate_provider(provider("provider", 1)).unwrap();
    fabric
        .publish_endpoint(endpoint("provider", "relay", 1, 1, 1))
        .unwrap();
    let consumer = activate_consumer(&mut fabric, "consumer", vec![wildcard_grant(1, &["query"])]);
    let proxy = resolve::<OnOffV1>(
        &mut fabric,
        consumer,
        Selector::new(OnOffV1::requirement())
            .provider(id("provider"))
            .facet(facet("relay")),
        Cardinality::One,
        RebindPolicy::SameIdentity,
        &[Operation::Query],
    )
    .unwrap()
    .proxies()[0];

    fabric
        .publish_endpoint(endpoint_with(
            "provider",
            "relay",
            1,
            1,
            2,
            &["set"],
            &["observable"],
        ))
        .unwrap();
    assert_core(
        fabric.rebind(proxy).unwrap_err(),
        CoreError::Execution(ExecutionError::Unsupported),
    );
}

#[test]
fn withdrawal_rejects_stale_incarnation_and_generation_without_removing_current() {
    let mut fabric = ServiceFabric::default();
    let provider_one = provider("provider", 1);
    fabric.activate_provider(provider_one.clone()).unwrap();
    let identity = endpoint("provider", "relay", 1, 1, 1).identity().clone();
    fabric
        .publish_endpoint(endpoint("provider", "relay", 1, 1, 1))
        .unwrap();
    fabric
        .publish_endpoint(endpoint("provider", "relay", 1, 1, 2))
        .unwrap();

    assert!(matches!(
        fabric.withdraw_endpoint(&provider_one, &identity, incarnation(1)),
        Err(LifecycleError::EndpointNotCurrent)
    ));
    assert_eq!(
        fabric.registry().get(&identity).unwrap().incarnation(),
        incarnation(2)
    );

    fabric.activate_provider(provider("provider", 2)).unwrap();
    fabric
        .publish_endpoint(endpoint("provider", "relay", 1, 2, 1))
        .unwrap();
    assert!(matches!(
        fabric.withdraw_endpoint(&provider_one, &identity, incarnation(1)),
        Err(LifecycleError::ProviderNotActive)
    ));
    assert_eq!(
        fabric.registry().get(&identity).unwrap().generation(),
        generation(2)
    );
}

#[test]
fn cycles_are_rejected_atomically_and_release_removes_the_edge() {
    let mut fabric = ServiceFabric::default();
    fabric.activate_provider(provider("a", 1)).unwrap();
    fabric.activate_provider(provider("b", 1)).unwrap();
    fabric
        .publish_endpoint(endpoint("a", "relay", 1, 1, 1))
        .unwrap();
    fabric
        .publish_endpoint(endpoint("b", "relay", 1, 1, 1))
        .unwrap();
    let a = activate_consumer(
        &mut fabric,
        "a",
        vec![exact_grant("b", "relay", 1, &["query"])],
    );
    let b = activate_consumer(
        &mut fabric,
        "b",
        vec![exact_grant("a", "relay", 1, &["query"])],
    );
    let a_to_b = resolve::<OnOffV1>(
        &mut fabric,
        a,
        Selector::new(OnOffV1::requirement())
            .provider(id("b"))
            .facet(facet("relay")),
        Cardinality::One,
        RebindPolicy::Never,
        &[Operation::Query],
    )
    .unwrap();
    assert_core(
        resolve::<OnOffV1>(
            &mut fabric,
            b.clone(),
            Selector::new(OnOffV1::requirement())
                .provider(id("a"))
                .facet(facet("relay")),
            Cardinality::One,
            RebindPolicy::Never,
            &[Operation::Query],
        )
        .unwrap_err(),
        CoreError::Execution(ExecutionError::Unsupported),
    );
    assert!(fabric.release_binding(a_to_b.id().unwrap()));
    assert!(
        resolve::<OnOffV1>(
            &mut fabric,
            b,
            Selector::new(OnOffV1::requirement())
                .provider(id("a"))
                .facet(facet("relay")),
            Cardinality::One,
            RebindPolicy::Never,
            &[Operation::Query],
        )
        .is_ok()
    );
}

#[test]
fn dropping_many_proxies_removes_only_each_providers_last_live_edge() {
    let mut fabric = ServiceFabric::default();
    for name in ["a", "b", "c"] {
        fabric.activate_provider(provider(name, 1)).unwrap();
    }
    for descriptor in [
        endpoint("b", "relay-1", 1, 1, 1),
        endpoint("b", "relay-2", 1, 1, 1),
        endpoint("c", "relay", 1, 1, 1),
    ] {
        fabric.publish_endpoint(descriptor).unwrap();
    }
    let a = activate_consumer(&mut fabric, "a", vec![wildcard_grant(1, &["query"])]);
    let many = resolve::<OnOffV1>(
        &mut fabric,
        a,
        Selector::new(OnOffV1::requirement()),
        Cardinality::Many,
        RebindPolicy::Never,
        &[Operation::Query],
    )
    .unwrap();
    let mut b_proxies = Vec::new();
    let mut c_proxy = None;
    for proxy in many.proxies() {
        match fabric
            .endpoint_for_proxy(*proxy)
            .unwrap()
            .provider()
            .as_str()
        {
            "b" => b_proxies.push(*proxy),
            "c" => c_proxy = Some(*proxy),
            provider => panic!("unexpected provider {provider}"),
        }
    }
    assert_eq!(b_proxies.len(), 2);
    let c_proxy = c_proxy.unwrap();

    fabric
        .publish_endpoint(endpoint("a", "relay", 1, 1, 1))
        .unwrap();
    let b = activate_consumer(
        &mut fabric,
        "b",
        vec![exact_grant("a", "relay", 1, &["query"])],
    );
    let b_to_a = || {
        ResolveRequest::<OnOffV1>::new(
            b.clone(),
            Selector::new(OnOffV1::requirement())
                .provider(id("a"))
                .facet(facet("relay")),
            [Operation::Query],
            Cardinality::One,
            RebindPolicy::Never,
        )
        .unwrap()
    };

    assert_core(
        fabric.resolve(b_to_a()).unwrap_err(),
        CoreError::Execution(ExecutionError::Unsupported),
    );
    assert!(fabric.drop_proxy(b_proxies[0].id()));
    assert_core(
        fabric.resolve(b_to_a()).unwrap_err(),
        CoreError::Execution(ExecutionError::Unsupported),
    );
    assert!(fabric.drop_proxy(b_proxies[1].id()));
    assert!(fabric.resolve(b_to_a()).is_ok());
    assert!(fabric.prepare_call(c_proxy, Operation::Query).is_ok());
}

#[test]
fn deep_cycle_detection_uses_bounded_stack_space() {
    const DEPTH: usize = 4_096;

    let mut fabric = ServiceFabric::default();
    let names = (0..=DEPTH)
        .map(|index| format!("node-{index}"))
        .collect::<Vec<_>>();
    for name in &names {
        fabric.activate_provider(provider(name, 1)).unwrap();
        fabric
            .publish_endpoint(endpoint(name, "relay", 1, 1, 1))
            .unwrap();
    }
    let consumers = names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let target = if index == DEPTH {
                &names[0]
            } else {
                &names[index + 1]
            };
            activate_consumer(
                &mut fabric,
                name,
                vec![exact_grant(target, "relay", 1, &["query"])],
            )
        })
        .collect::<Vec<_>>();

    for index in 0..DEPTH {
        resolve::<OnOffV1>(
            &mut fabric,
            consumers[index].clone(),
            Selector::new(OnOffV1::requirement())
                .provider(id(&names[index + 1]))
                .facet(facet("relay")),
            Cardinality::One,
            RebindPolicy::Never,
            &[Operation::Query],
        )
        .unwrap();
    }
    assert_core(
        resolve::<OnOffV1>(
            &mut fabric,
            consumers[DEPTH].clone(),
            Selector::new(OnOffV1::requirement())
                .provider(id(&names[0]))
                .facet(facet("relay")),
            Cardinality::One,
            RebindPolicy::Never,
            &[Operation::Query],
        )
        .unwrap_err(),
        CoreError::Execution(ExecutionError::Unsupported),
    );
}

#[test]
fn state_and_event_identity_are_endpoint_and_generation_scoped() {
    let mut fabric = ServiceFabric::default();
    fabric.activate_provider(provider("provider", 1)).unwrap();
    let descriptor = endpoint("provider", "relay", 1, 1, 1);
    let identity = descriptor.identity().clone();
    fabric.publish_endpoint(descriptor).unwrap();

    let first = fabric.state_identity(&identity, 7).unwrap();
    let second = fabric.state_identity(&identity, 8).unwrap();
    assert_eq!(first.endpoint, second.endpoint);
    assert_eq!(first.generation, second.generation);
    assert!(first.revision < second.revision);

    let event_id = EventId::new("event-42").unwrap();
    let source = SourceEventId::new("device-sequence-9").unwrap();
    let event = fabric
        .event_identity(&identity, event_id.clone(), Some(source.clone()))
        .unwrap();
    let retransmission = fabric
        .event_identity(&identity, event_id, Some(source))
        .unwrap();
    assert_eq!(event, retransmission);
}

#[test]
fn consumer_proxy_runtime_representation_is_only_the_host_id() {
    assert_eq!(size_of::<TypedProxy<OnOffV1>>(), size_of::<ProxyId>());
    assert_ne!(
        std::any::TypeId::of::<TypedProxy<OnOffV1>>(),
        std::any::TypeId::of::<TypedProxy<OnOffV2>>()
    );
}

#[test]
fn registry_requires_new_incarnation_for_metadata_changes_and_same_major_minimum() {
    let mut registry = EndpointRegistry::default();
    let original = endpoint("provider", "relay", 1, 1, 1);
    registry.publish(original.clone()).unwrap();
    let changed = EndpointDescriptor::new(
        original.identity().clone(),
        Version {
            major: 1,
            minor: 1,
            patch: 0,
        },
        generation(1),
        incarnation(1),
        [operation("query")],
        [],
    )
    .unwrap();
    assert_eq!(
        registry.publish(changed),
        Err(RegistryError::ChangedWithoutIncarnation)
    );

    let selector = Selector::new(
        ContractRequirement::new(
            contract(1),
            Version {
                major: 1,
                minor: 1,
                patch: 0,
            },
        )
        .unwrap(),
    );
    assert!(registry.resolve(&selector).is_empty());
}
