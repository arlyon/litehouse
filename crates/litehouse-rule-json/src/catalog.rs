use litehouse_contracts::management::management_manifest;
use serde::Serialize;
use serde_json::Value;

use crate::model::{
    CapabilityOperationKind, ContractEventRef, ContractRef, ContractTypeRef, CoveringAxis,
    DesiredProjectionPath, QuantityProfileRef, ReceiptConfidence, StateFieldRef,
    StatefulContractKind, TriggerContextKind, TypeRef,
};

pub(crate) const PORT_CATALOG_URL: &str =
    "https://schemas.litehouse.dev/rule-ir/v1.0.0/node-port-catalog.json";

#[derive(Serialize)]
pub(crate) struct PortCatalog<'a> {
    #[serde(rename = "$schema")]
    pub schema: &'a str,
    #[serde(rename = "$id")]
    pub id: &'a str,
    pub schema_version: &'a str,
    pub entries: Vec<CatalogEntry>,
    pub capability_operations: Vec<OperationTemplate>,
    pub contract_events: Vec<ContractEventMapping>,
    pub state_fields: Vec<StateFieldMapping>,
    pub desired_patch_paths: Vec<DesiredPathMapping>,
    pub execution_state: ExecutionStatePolicy,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EntryClass {
    Trigger,
    Node,
}

#[derive(Serialize)]
pub(crate) struct CatalogEntry {
    pub class: EntryClass,
    pub kind: &'static str,
    pub version: u8,
    pub ports: Vec<PortDeclaration>,
    pub dynamic_ports: Vec<DynamicPortDeclaration>,
    pub runtime_state: RuntimeStateDeclaration,
}

#[derive(Serialize)]
pub(crate) struct PortDeclaration {
    pub name: &'static str,
    pub direction: PortDirection,
    pub type_ref: TypeExpression,
    pub cardinality: PortCardinality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PortDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PortCardinality {
    ZeroOrOne,
    ExactlyOne,
    OneOrMore,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum TypeExpression {
    Fixed {
        type_ref: TypeRef,
    },
    StateField {
        field_property: &'static str,
    },
    QuantityInput {
        profile_property: &'static str,
        optional: bool,
    },
    QuantityResult {
        profile_property: &'static str,
        operation_property: &'static str,
    },
    CapabilityReceipt {
        operation_property: &'static str,
    },
    ReferencedEffectReceipt {
        effect_node_property: &'static str,
    },
    TriggerContext {
        trigger_property: &'static str,
        optional: bool,
    },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum DynamicPortDeclaration {
    ConditionInputs {
        source_property: &'static str,
        name_property: &'static str,
        type_property: &'static str,
        direction: PortDirection,
        cardinality: PortCardinality,
    },
    ForkJoinPhases {
        phase_property: &'static str,
        source_property: &'static str,
        name_property: &'static str,
        fork_input: &'static str,
        fork_input_direction: PortDirection,
        fork_input_cardinality: PortCardinality,
        fork_output_prefix: &'static str,
        fork_output_direction: PortDirection,
        fork_output_cardinality: PortCardinality,
        join_input_prefix: &'static str,
        join_input_direction: PortDirection,
        join_input_cardinality: PortCardinality,
        join_output: &'static str,
        join_output_direction: PortDirection,
        join_output_cardinality: PortCardinality,
        type_ref: TypeRef,
    },
}

#[derive(Serialize)]
pub(crate) struct RuntimeStateDeclaration {
    pub scope: RuntimeStateScope,
    pub restart: RestartDisposition,
    pub initial: RuntimeInitialState,
    pub fields: Vec<RuntimeStateField>,
    pub max_encoded_bytes: u32,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimeStateScope {
    Stateless,
    ProcessEpochLocal,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RestartDisposition {
    None,
    Discard,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimeInitialState {
    Null,
    Empty,
}

#[derive(Serialize)]
pub(crate) struct RuntimeStateField {
    pub name: &'static str,
    pub type_ref: TypeExpression,
    pub maximum_items: Option<u16>,
}

#[derive(Serialize)]
pub(crate) struct OperationTemplate {
    pub operation: CapabilityOperationKind,
    pub selector_contract: ContractRef,
    pub service_binding: ServiceBinding,
    pub arguments: Vec<ArgumentTemplate>,
    pub required_receipt: ReceiptRequirementRule,
    pub returned_receipts: Vec<ReceiptConfidence>,
    pub retry: RetryEligibility,
}

#[derive(Serialize)]
pub(crate) struct ServiceBinding {
    pub package: String,
    pub interface: String,
    pub operation: String,
    pub projection: String,
}

#[derive(Serialize)]
pub(crate) struct ArgumentTemplate {
    pub name: &'static str,
    pub type_ref: TypeRef,
    pub required: bool,
    pub canonical_schema: Value,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReceiptRequirementRule {
    AcceptedForDeliveryOnly,
    NotificationAdvertisedMaximum,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RetryEligibility {
    Never,
    DefinitelyPreDispatchWithMatchingProviderDeduplication,
}

#[derive(Serialize)]
pub(crate) struct ContractEventMapping {
    pub event: ContractEventRef,
    pub selector_contract: ContractRef,
    pub payload_type: TypeRef,
    pub wit_event_kind: &'static str,
}

#[derive(Serialize)]
pub(crate) struct StateFieldMapping {
    pub selector_contract: ContractRef,
    pub field: StateFieldRef,
    pub type_ref: TypeRef,
    pub wit_evidence: WitFieldEvidence,
    pub observed_only: bool,
}

#[derive(Serialize)]
pub(crate) struct WitFieldEvidence {
    pub package: &'static str,
    pub state_field: &'static str,
    pub state_record_path: &'static str,
}

#[derive(Serialize)]
pub(crate) struct DesiredPathMapping {
    pub path: DesiredProjectionPath,
    pub selector_contract: ContractRef,
    pub type_ref: TypeRef,
    pub mutability: DesiredPathMutability,
    pub operations: Vec<PatchOperationKind>,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DesiredPathMutability {
    Required,
    Optional,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchOperationKind {
    Test,
    Add,
    Replace,
    Remove,
}

#[derive(Serialize)]
pub(crate) struct ExecutionStatePolicy {
    pub process_epoch_local: bool,
    pub restart_discards: Vec<&'static str>,
    pub restart_rehydrates_dependencies_before_arming: bool,
    pub schedule_restart_policy: &'static str,
    pub effect_continuation_policy: &'static str,
    pub retry_deadline_policy: &'static str,
    pub activation_binding_boundary: &'static str,
}

pub(crate) fn catalog(schema_version: &'static str) -> PortCatalog<'static> {
    PortCatalog {
        schema: "https://json-schema.org/draft/2020-12/schema",
        id: PORT_CATALOG_URL,
        schema_version,
        entries: entries(),
        capability_operations: operation_templates(),
        contract_events: contract_event_mappings(),
        state_fields: state_field_mappings(),
        desired_patch_paths: desired_path_mappings(),
        execution_state: ExecutionStatePolicy {
            process_epoch_local: true,
            restart_discards: vec![
                "runs",
                "delays",
                "debounce_throttle_windows",
                "trigger_queues",
                "pending_effects",
                "receipt_continuations",
                "timers",
                "event_deduplication",
                "cursors",
            ],
            restart_rehydrates_dependencies_before_arming: true,
            schedule_restart_policy: "next_strictly_future_occurrence_without_catch_up",
            effect_continuation_policy: "suspend_after_intent_and_resume_only_from_the_exact_typed_receipt",
            retry_deadline_policy: "all_attempts_and_backoff_share_one_absolute_effect_deadline",
            activation_binding_boundary: "provider_generations_facet_revisions_and_grant_revisions_are_resolved_activation_record_fields_not_authored_rule_json",
        },
    }
}

fn entries() -> Vec<CatalogEntry> {
    vec![
        entry(
            EntryClass::Trigger,
            "state_observation",
            vec![
                optional_output("next", fixed(TypeRef::Control)),
                optional_output(
                    "context",
                    fixed(TypeRef::TriggerContext {
                        trigger: TriggerContextKind::StateObservation,
                    }),
                ),
            ],
            vec![],
            process_state(
                vec![state_field(
                    "queued_inputs",
                    fixed(TypeRef::List {
                        item: Box::new(TypeRef::TriggerContext {
                            trigger: TriggerContextKind::StateObservation,
                        }),
                        max_items: 16,
                    }),
                    Some(16),
                )],
                65_536,
            ),
        ),
        entry(
            EntryClass::Trigger,
            "discrete_event",
            vec![
                optional_output("next", fixed(TypeRef::Control)),
                optional_output(
                    "context",
                    fixed(TypeRef::TriggerContext {
                        trigger: TriggerContextKind::DiscreteEvent,
                    }),
                ),
            ],
            vec![],
            process_state(
                vec![
                    state_field(
                        "recent_event_ids",
                        fixed(TypeRef::List {
                            item: Box::new(TypeRef::String),
                            max_items: 1_024,
                        }),
                        Some(1_024),
                    ),
                    state_field(
                        "queued_inputs",
                        fixed(TypeRef::List {
                            item: Box::new(TypeRef::TriggerContext {
                                trigger: TriggerContextKind::DiscreteEvent,
                            }),
                            max_items: 16,
                        }),
                        Some(16),
                    ),
                ],
                65_536,
            ),
        ),
        entry(
            EntryClass::Trigger,
            "time_schedule",
            vec![
                optional_output("next", fixed(TypeRef::Control)),
                optional_output(
                    "context",
                    fixed(TypeRef::TriggerContext {
                        trigger: TriggerContextKind::TimeSchedule,
                    }),
                ),
            ],
            vec![],
            process_state(
                vec![
                    state_field("next_occurrence", fixed(TypeRef::Timestamp), None),
                    state_field("occurrence_id", fixed(TypeRef::String), None),
                ],
                1_024,
            ),
        ),
        entry(
            EntryClass::Node,
            "snapshot",
            vec![
                control_input("in"),
                optional_output("next", fixed(TypeRef::Control)),
                optional_output(
                    "value",
                    TypeExpression::StateField {
                        field_property: "field",
                    },
                ),
            ],
            vec![],
            stateless(),
        ),
        entry(
            EntryClass::Node,
            "condition",
            vec![
                control_input("in"),
                optional_output("next", fixed(TypeRef::Control)),
                optional_output("result", fixed(TypeRef::PredicateOutcome)),
            ],
            vec![DynamicPortDeclaration::ConditionInputs {
                source_property: "expression.inputs",
                name_property: "name",
                type_property: "type_ref",
                direction: PortDirection::Input,
                cardinality: PortCardinality::ExactlyOne,
            }],
            stateless(),
        ),
        entry(
            EntryClass::Node,
            "quantity",
            vec![
                control_input("in"),
                input(
                    "value",
                    TypeExpression::QuantityInput {
                        profile_property: "input_profile",
                        optional: true,
                    },
                    PortCardinality::ExactlyOne,
                ),
                optional_output("next", fixed(TypeRef::Control)),
                optional_output(
                    "result",
                    TypeExpression::QuantityResult {
                        profile_property: "input_profile",
                        operation_property: "operation",
                    },
                ),
            ],
            vec![],
            stateless(),
        ),
        entry(
            EntryClass::Node,
            "branch",
            vec![
                control_input("in"),
                input(
                    "predicate",
                    fixed(TypeRef::PredicateOutcome),
                    PortCardinality::ExactlyOne,
                ),
                optional_output("true", fixed(TypeRef::Control)),
                optional_output("false", fixed(TypeRef::Control)),
                optional_output("unknown", fixed(TypeRef::Control)),
                optional_output("error", fixed(TypeRef::Control)),
            ],
            vec![],
            stateless(),
        ),
        entry(
            EntryClass::Node,
            "desired_patch",
            vec![
                control_input("in"),
                optional_output("next", fixed(TypeRef::Control)),
                optional_output("patch", fixed(TypeRef::DesiredPatch)),
            ],
            vec![],
            stateless(),
        ),
        entry(
            EntryClass::Node,
            "effect_intent",
            vec![
                control_input("in"),
                optional_output("intent", fixed(TypeRef::EffectIntent)),
                optional_output(
                    "receipt",
                    TypeExpression::CapabilityReceipt {
                        operation_property: "operation",
                    },
                ),
            ],
            vec![],
            process_state(
                vec![
                    state_field("pending_intent_id", fixed(TypeRef::String), None),
                    state_field("continuation_node_id", fixed(TypeRef::String), None),
                ],
                4_096,
            ),
        ),
        entry(
            EntryClass::Node,
            "delay",
            vec![
                control_input("in"),
                optional_output("next", fixed(TypeRef::Control)),
            ],
            vec![],
            process_state(
                vec![
                    state_field("scheduled_at", fixed(TypeRef::Timestamp), None),
                    state_field("continuation_node_id", fixed(TypeRef::String), None),
                ],
                2_048,
            ),
        ),
        entry(
            EntryClass::Node,
            "debounce_throttle",
            vec![
                control_input("in"),
                optional_output("admitted", fixed(TypeRef::Control)),
                optional_output("dropped", fixed(TypeRef::Control)),
            ],
            vec![],
            process_state(
                vec![
                    state_field("window_ends_at", fixed(TypeRef::Timestamp), None),
                    state_field(
                        "winning_input",
                        TypeExpression::TriggerContext {
                            trigger_property: "trigger",
                            optional: true,
                        },
                        None,
                    ),
                    state_field("dropped_count", fixed(TypeRef::U64), None),
                    state_field("coalesced_count", fixed(TypeRef::U64), None),
                ],
                65_536,
            ),
        ),
        entry(
            EntryClass::Node,
            "fork_join",
            vec![],
            vec![DynamicPortDeclaration::ForkJoinPhases {
                phase_property: "phase.kind",
                source_property: "phase.branches",
                name_property: "id",
                fork_input: "in",
                fork_input_direction: PortDirection::Input,
                fork_input_cardinality: PortCardinality::OneOrMore,
                fork_output_prefix: "branch:",
                fork_output_direction: PortDirection::Output,
                fork_output_cardinality: PortCardinality::ZeroOrOne,
                join_input_prefix: "arrival:",
                join_input_direction: PortDirection::Input,
                join_input_cardinality: PortCardinality::ExactlyOne,
                join_output: "next",
                join_output_direction: PortDirection::Output,
                join_output_cardinality: PortCardinality::ZeroOrOne,
                type_ref: TypeRef::Control,
            }],
            process_state(
                vec![state_field(
                    "arrivals",
                    fixed(TypeRef::List {
                        item: Box::new(TypeRef::ForkToken),
                        max_items: 4,
                    }),
                    Some(4),
                )],
                4_096,
            ),
        ),
        entry(
            EntryClass::Node,
            "receipt_branch",
            vec![
                input(
                    "receipt",
                    TypeExpression::ReferencedEffectReceipt {
                        effect_node_property: "effect_node_id",
                    },
                    PortCardinality::ExactlyOne,
                ),
                optional_output("accepted_for_delivery", fixed(TypeRef::Control)),
                optional_output("protocol_or_target_acknowledged", fixed(TypeRef::Control)),
                optional_output("completed", fixed(TypeRef::Control)),
                optional_output("observed_completed", fixed(TypeRef::Control)),
                optional_output("rejected", fixed(TypeRef::Control)),
                optional_output("error", fixed(TypeRef::Control)),
                optional_output("cancelled", fixed(TypeRef::Control)),
                optional_output("outcome_unknown", fixed(TypeRef::Control)),
            ],
            vec![],
            process_state(
                vec![state_field(
                    "pending_receipt",
                    TypeExpression::ReferencedEffectReceipt {
                        effect_node_property: "effect_node_id",
                    },
                    None,
                )],
                4_096,
            ),
        ),
        entry(
            EntryClass::Node,
            "end",
            vec![control_input("in")],
            vec![],
            stateless(),
        ),
    ]
}

fn operation_templates() -> Vec<OperationTemplate> {
    let manifest = management_manifest().expect("authoritative management manifest must validate");
    CapabilityOperationKind::ALL
        .into_iter()
        .map(|operation| {
            let manifest_operation = manifest
                .operations
                .iter()
                .find(|candidate| candidate.id == operation.operation_id())
                .expect("closed operation must exist in management manifest");
            let properties = manifest_operation.schemas.input.schema["properties"]
                .as_object()
                .expect("capability input properties");
            let required = manifest_operation.schemas.input.schema["required"]
                .as_array()
                .expect("capability input required fields");
            let service = manifest_operation
                .service_binding
                .as_ref()
                .expect("capability operation service binding");
            let arguments = authored_argument_types(operation)
                .into_iter()
                .map(|(name, type_ref)| ArgumentTemplate {
                    name,
                    type_ref,
                    required: required.iter().any(|value| value == name),
                    canonical_schema: properties
                        .get(name)
                        .unwrap_or_else(|| {
                            panic!("missing {name} schema for {}", operation.operation_id())
                        })
                        .clone(),
                })
                .collect();
            OperationTemplate {
                operation,
                selector_contract: ContractRef::litehouse(operation.package(), "capability", 1),
                service_binding: ServiceBinding {
                    package: service.package.clone(),
                    interface: service.interface.clone(),
                    operation: service.operation.clone(),
                    projection: service.projection.clone(),
                },
                arguments,
                required_receipt: if operation == CapabilityOperationKind::NotificationSend {
                    ReceiptRequirementRule::NotificationAdvertisedMaximum
                } else {
                    ReceiptRequirementRule::AcceptedForDeliveryOnly
                },
                returned_receipts: all_receipts(),
                retry: if operation.retry_eligible() {
                    RetryEligibility::DefinitelyPreDispatchWithMatchingProviderDeduplication
                } else {
                    RetryEligibility::Never
                },
            }
        })
        .collect()
}

fn authored_argument_types(operation: CapabilityOperationKind) -> Vec<(&'static str, TypeRef)> {
    use CapabilityOperationKind as O;
    let decimal = || TypeRef::Contract {
        type_ref: ContractTypeRef::CanonicalDecimal,
    };
    let color = || TypeRef::Contract {
        type_ref: ContractTypeRef::ColorValue,
    };
    match operation {
        O::OnOffSet => vec![("on", TypeRef::Bool)],
        O::OnOffToggle | O::LevelStop | O::ColorStop | O::MomentaryInvoke => vec![],
        O::LevelSet => vec![("level", decimal())],
        O::LevelStep => vec![("step", decimal())],
        O::LevelMove => vec![
            ("direction", TypeRef::String),
            ("rate_per_second", decimal()),
        ],
        O::LevelTransition => vec![("level", decimal()), ("duration_ms", TypeRef::U64)],
        O::LevelSetCoupled => vec![("level", decimal()), ("on_when_nonzero", TypeRef::Bool)],
        O::LevelStepCoupled => vec![("step", decimal()), ("on_when_nonzero", TypeRef::Bool)],
        O::LevelMoveCoupled => vec![
            ("direction", TypeRef::String),
            ("rate_per_second", decimal()),
            ("on_when_nonzero", TypeRef::Bool),
        ],
        O::LevelTransitionCoupled => vec![
            ("level", decimal()),
            ("duration_ms", TypeRef::U64),
            ("on_when_nonzero", TypeRef::Bool),
        ],
        O::ColorSet => vec![("color", color())],
        O::ColorStep | O::ColorMove => {
            vec![("color", color()), ("direction", TypeRef::String)]
        }
        O::ColorTransition => vec![("color", color()), ("duration_ms", TypeRef::U64)],
        O::ThermostatSetMode => vec![(
            "mode",
            TypeRef::Contract {
                type_ref: ContractTypeRef::ThermostatMode,
            },
        )],
        O::ThermostatSetSingleTarget => vec![("target_celsius", decimal())],
        O::ThermostatSetDualTarget => vec![
            ("heat_target_celsius", decimal()),
            ("cool_target_celsius", decimal()),
        ],
        O::ThermostatSetFan => vec![("fan_choice", TypeRef::String)],
        O::ThermostatSetSwing => vec![("swing_choice", TypeRef::String)],
        O::ThermostatSetPreset => vec![("preset_choice", TypeRef::String)],
        O::CoveringOpen | O::CoveringClose | O::CoveringStop => {
            vec![("axis", TypeRef::String)]
        }
        O::CoveringSetLift | O::CoveringSetTilt => vec![("position", decimal())],
        O::NotificationSend => vec![
            ("notification_template_id", TypeRef::String),
            ("priority", TypeRef::String),
            (
                "expires_in_ms",
                TypeRef::Optional {
                    item: Box::new(TypeRef::U64),
                },
            ),
        ],
    }
}

fn all_receipts() -> Vec<ReceiptConfidence> {
    vec![
        ReceiptConfidence::AcceptedForDelivery,
        ReceiptConfidence::ProtocolOrTargetAcknowledged,
        ReceiptConfidence::Completed,
        ReceiptConfidence::ObservedCompleted,
    ]
}

fn contract_event_mappings() -> Vec<ContractEventMapping> {
    use crate::model::{
        BooleanStateEventKind, MeasurementEventKind, OnOffEventKind, StateObservationEventKind,
    };
    vec![
        ContractEventRef::OnOff {
            event: OnOffEventKind::StateObservation,
        },
        ContractEventRef::BooleanState {
            event: BooleanStateEventKind::StateObservation,
        },
        ContractEventRef::BooleanState {
            event: BooleanStateEventKind::Occurrence,
        },
        ContractEventRef::LevelControl {
            event: StateObservationEventKind::StateObservation,
        },
        ContractEventRef::ColorControl {
            event: StateObservationEventKind::StateObservation,
        },
        ContractEventRef::Measurement {
            event: MeasurementEventKind::Observation,
        },
        ContractEventRef::Thermostat {
            event: StateObservationEventKind::StateObservation,
        },
        ContractEventRef::WindowCovering {
            event: StateObservationEventKind::StateObservation,
        },
    ]
    .into_iter()
    .map(|event| ContractEventMapping {
        selector_contract: event.selector_contract(),
        payload_type: TypeRef::Contract {
            type_ref: event.payload_type(),
        },
        wit_event_kind: event.event_name(),
        event,
    })
    .collect()
}

fn state_field_mappings() -> Vec<StateFieldMapping> {
    use StateFieldRef as F;
    let mut fields = vec![
        field(F::OnOffCurrent, "on-off", "current", "current"),
        field(F::BooleanCurrent, "boolean-state", "current", "current"),
        field(
            F::LevelCurrentLevel,
            "level-control",
            "current-level",
            "current",
        ),
        field(
            F::LevelTargetLevel,
            "level-control",
            "target-level",
            "target",
        ),
        field(
            F::LevelTransitionRemaining,
            "level-control",
            "transition-remaining",
            "transition-remaining-milliseconds",
        ),
        field(
            F::ColorCurrentColor,
            "color-control",
            "current-color",
            "current",
        ),
        field(
            F::ColorTargetColor,
            "color-control",
            "target-color",
            "target",
        ),
        field(
            F::ColorActiveModel,
            "color-control",
            "active-model",
            "active-model",
        ),
        field(
            F::MeasurementNativeRepresentation,
            "measurement",
            "native-representation",
            "native",
        ),
        field(
            F::ThermostatCurrentMode,
            "thermostat",
            "current-mode",
            "current-mode",
        ),
        field(
            F::ThermostatTargetMode,
            "thermostat",
            "target-mode",
            "target-mode",
        ),
        field(
            F::ThermostatAction,
            "thermostat",
            "action",
            "current-action",
        ),
        field(F::ThermostatRunning, "thermostat", "running", "running"),
        field(F::ThermostatTargets, "thermostat", "targets", "targets"),
        field(F::ThermostatFan, "thermostat", "fan", "fan"),
        field(F::ThermostatSwing, "thermostat", "swing", "swing"),
        field(F::ThermostatPreset, "thermostat", "preset", "preset"),
        field(
            F::CoveringCurrentPosition {
                axis: CoveringAxis::Lift,
            },
            "window-covering",
            "current-position",
            "axes[lift].current",
        ),
        field(
            F::CoveringCurrentPosition {
                axis: CoveringAxis::Tilt,
            },
            "window-covering",
            "current-position",
            "axes[tilt].current",
        ),
        field(
            F::CoveringTargetPosition {
                axis: CoveringAxis::Lift,
            },
            "window-covering",
            "target-position",
            "axes[lift].target",
        ),
        field(
            F::CoveringTargetPosition {
                axis: CoveringAxis::Tilt,
            },
            "window-covering",
            "target-position",
            "axes[tilt].target",
        ),
        field(
            F::CoveringMovement {
                axis: CoveringAxis::Lift,
            },
            "window-covering",
            "movement",
            "axes[lift].movement",
        ),
        field(
            F::CoveringMovement {
                axis: CoveringAxis::Tilt,
            },
            "window-covering",
            "movement",
            "axes[tilt].movement",
        ),
        field(
            F::CoveringSafetyStatus,
            "window-covering",
            "safety-status",
            "safety-status",
        ),
        field(
            F::CoveringFaultStatus,
            "window-covering",
            "fault-status",
            "fault-status",
        ),
    ];
    fields.extend(QuantityProfileRef::ALL.into_iter().map(|profile| {
        field(
            F::MeasurementCurrent { profile },
            "measurement",
            "current",
            "current",
        )
    }));
    fields.extend(StatefulContractKind::ALL.into_iter().map(|contract| {
        field(
            F::ObservationRevision { contract },
            contract.package(),
            "observation-meta.revision",
            "meta.revision",
        )
    }));
    fields
}

fn field(
    field: StateFieldRef,
    package: &'static str,
    wit_field: &'static str,
    record_path: &'static str,
) -> StateFieldMapping {
    StateFieldMapping {
        selector_contract: ContractRef::litehouse(package, "capability", 1),
        type_ref: field.type_ref(),
        field,
        wit_evidence: WitFieldEvidence {
            package,
            state_field: wit_field,
            state_record_path: record_path,
        },
        observed_only: true,
    }
}

fn desired_path_mappings() -> Vec<DesiredPathMapping> {
    DesiredProjectionPath::ALL
        .into_iter()
        .map(|path| {
            let (type_ref, mutability) = match path {
                DesiredProjectionPath::On => (TypeRef::Bool, DesiredPathMutability::Required),
                DesiredProjectionPath::Level
                | DesiredProjectionPath::ThermostatSingleTarget
                | DesiredProjectionPath::ThermostatHeatTarget
                | DesiredProjectionPath::ThermostatCoolTarget
                | DesiredProjectionPath::CoveringLift
                | DesiredProjectionPath::CoveringTilt => (
                    TypeRef::Contract {
                        type_ref: ContractTypeRef::CanonicalDecimal,
                    },
                    DesiredPathMutability::Optional,
                ),
                DesiredProjectionPath::Color => (
                    TypeRef::Contract {
                        type_ref: ContractTypeRef::ColorValue,
                    },
                    DesiredPathMutability::Optional,
                ),
                DesiredProjectionPath::ThermostatMode => (
                    TypeRef::Contract {
                        type_ref: ContractTypeRef::ThermostatMode,
                    },
                    DesiredPathMutability::Required,
                ),
                DesiredProjectionPath::ThermostatFan
                | DesiredProjectionPath::ThermostatSwing
                | DesiredProjectionPath::ThermostatPreset => {
                    (TypeRef::String, DesiredPathMutability::Optional)
                }
            };
            let mut operations = vec![
                PatchOperationKind::Test,
                PatchOperationKind::Add,
                PatchOperationKind::Replace,
            ];
            if matches!(mutability, DesiredPathMutability::Optional) {
                operations.push(PatchOperationKind::Remove);
            }
            DesiredPathMapping {
                path,
                selector_contract: ContractRef::litehouse(path.package(), "capability", 1),
                type_ref,
                mutability,
                operations,
            }
        })
        .collect()
}

fn entry(
    class: EntryClass,
    kind: &'static str,
    ports: Vec<PortDeclaration>,
    dynamic_ports: Vec<DynamicPortDeclaration>,
    runtime_state: RuntimeStateDeclaration,
) -> CatalogEntry {
    CatalogEntry {
        class,
        kind,
        version: 1,
        ports,
        dynamic_ports,
        runtime_state,
    }
}

fn fixed(type_ref: TypeRef) -> TypeExpression {
    TypeExpression::Fixed { type_ref }
}

fn input(
    name: &'static str,
    type_ref: TypeExpression,
    cardinality: PortCardinality,
) -> PortDeclaration {
    PortDeclaration {
        name,
        direction: PortDirection::Input,
        type_ref,
        cardinality,
    }
}

fn control_input(name: &'static str) -> PortDeclaration {
    input(name, fixed(TypeRef::Control), PortCardinality::OneOrMore)
}

fn optional_output(name: &'static str, type_ref: TypeExpression) -> PortDeclaration {
    PortDeclaration {
        name,
        direction: PortDirection::Output,
        type_ref,
        cardinality: PortCardinality::ZeroOrOne,
    }
}

fn stateless() -> RuntimeStateDeclaration {
    RuntimeStateDeclaration {
        scope: RuntimeStateScope::Stateless,
        restart: RestartDisposition::None,
        initial: RuntimeInitialState::Null,
        fields: vec![],
        max_encoded_bytes: 0,
    }
}

fn process_state(
    fields: Vec<RuntimeStateField>,
    max_encoded_bytes: u32,
) -> RuntimeStateDeclaration {
    RuntimeStateDeclaration {
        scope: RuntimeStateScope::ProcessEpochLocal,
        restart: RestartDisposition::Discard,
        initial: RuntimeInitialState::Empty,
        fields,
        max_encoded_bytes,
    }
}

fn state_field(
    name: &'static str,
    type_ref: TypeExpression,
    maximum_items: Option<u16>,
) -> RuntimeStateField {
    RuntimeStateField {
        name,
        type_ref,
        maximum_items,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedPort {
    pub name: String,
    pub direction: PortDirection,
    pub type_ref: TypeRef,
    pub cardinality: PortCardinality,
}

pub(crate) fn resolved_trigger_ports(trigger: &crate::model::TriggerNode) -> Vec<ResolvedPort> {
    let entry = entries()
        .into_iter()
        .find(|entry| entry.kind == trigger.kind_name())
        .expect("closed trigger must have a catalog entry");
    entry
        .ports
        .into_iter()
        .map(|port| ResolvedPort {
            name: port.name.to_owned(),
            direction: port.direction,
            type_ref: resolve_type_expression(&port.type_ref, None, &[]),
            cardinality: port.cardinality,
        })
        .collect()
}

pub(crate) fn resolved_node_ports(
    node: &crate::model::Node,
    nodes: &[crate::model::Node],
) -> Vec<ResolvedPort> {
    let entry = entries()
        .into_iter()
        .find(|entry| entry.kind == node.kind_name())
        .expect("closed node must have a catalog entry");
    let mut resolved = entry
        .ports
        .into_iter()
        .map(|port| ResolvedPort {
            name: port.name.to_owned(),
            direction: port.direction,
            type_ref: resolve_type_expression(&port.type_ref, Some(node), nodes),
            cardinality: port.cardinality,
        })
        .collect::<Vec<_>>();
    for dynamic in entry.dynamic_ports {
        match (dynamic, node) {
            (
                DynamicPortDeclaration::ConditionInputs {
                    direction,
                    cardinality,
                    ..
                },
                crate::model::Node::Condition { expression, .. },
            ) => resolved.extend(expression.inputs.iter().map(|input| ResolvedPort {
                name: input.name.clone(),
                direction,
                type_ref: input.type_ref.clone(),
                cardinality,
            })),
            (
                DynamicPortDeclaration::ForkJoinPhases {
                    fork_input,
                    fork_input_direction,
                    fork_input_cardinality,
                    fork_output_prefix,
                    fork_output_direction,
                    fork_output_cardinality,
                    ..
                },
                crate::model::Node::ForkJoin {
                    phase: crate::model::ForkJoinPhase::Fork { branches, .. },
                    ..
                },
            ) => {
                resolved.push(ResolvedPort {
                    name: fork_input.to_owned(),
                    direction: fork_input_direction,
                    type_ref: TypeRef::Control,
                    cardinality: fork_input_cardinality,
                });
                resolved.extend(branches.iter().map(|branch| ResolvedPort {
                    name: format!("{fork_output_prefix}{}", branch.id),
                    direction: fork_output_direction,
                    type_ref: TypeRef::Control,
                    cardinality: fork_output_cardinality,
                }));
            }
            (
                DynamicPortDeclaration::ForkJoinPhases {
                    join_input_prefix,
                    join_input_direction,
                    join_input_cardinality,
                    join_output,
                    join_output_direction,
                    join_output_cardinality,
                    ..
                },
                crate::model::Node::ForkJoin {
                    phase: crate::model::ForkJoinPhase::Join { branches, .. },
                    ..
                },
            ) => {
                resolved.extend(branches.iter().map(|branch| ResolvedPort {
                    name: format!("{join_input_prefix}{}", branch.id),
                    direction: join_input_direction,
                    type_ref: TypeRef::Control,
                    cardinality: join_input_cardinality,
                }));
                resolved.push(ResolvedPort {
                    name: join_output.to_owned(),
                    direction: join_output_direction,
                    type_ref: TypeRef::Control,
                    cardinality: join_output_cardinality,
                });
            }
            _ => unreachable!("dynamic catalog declaration must match its closed node variant"),
        }
    }
    resolved
}

fn resolve_type_expression(
    expression: &TypeExpression,
    node: Option<&crate::model::Node>,
    nodes: &[crate::model::Node],
) -> TypeRef {
    use crate::model::Node;
    match expression {
        TypeExpression::Fixed { type_ref } => type_ref.clone(),
        TypeExpression::StateField { .. } => match node {
            Some(Node::Snapshot { field, .. }) => field.type_ref(),
            _ => unreachable!("state-field expression belongs only to snapshot"),
        },
        TypeExpression::QuantityInput { optional, .. } => match node {
            Some(Node::Quantity { input_profile, .. }) => {
                let quantity = TypeRef::Quantity {
                    catalog: crate::model::QuantityCatalogRef::V1,
                    profile: *input_profile,
                };
                if *optional {
                    TypeRef::Optional {
                        item: Box::new(quantity),
                    }
                } else {
                    quantity
                }
            }
            _ => unreachable!("quantity-input expression belongs only to quantity"),
        },
        TypeExpression::QuantityResult { .. } => match node {
            Some(Node::Quantity {
                input_profile,
                operation,
                ..
            }) => match operation.result_profile(*input_profile) {
                None => TypeRef::PredicateOutcome,
                Some(profile) => TypeRef::Quantity {
                    catalog: crate::model::QuantityCatalogRef::V1,
                    profile,
                },
            },
            _ => unreachable!("quantity-result expression belongs only to quantity"),
        },
        TypeExpression::CapabilityReceipt { .. } => match node {
            Some(Node::EffectIntent { operation, .. }) => TypeRef::Receipt {
                operation: operation.kind(),
            },
            _ => unreachable!("capability-receipt expression belongs only to effect intent"),
        },
        TypeExpression::ReferencedEffectReceipt { .. } => match node {
            Some(Node::ReceiptBranch { effect_node_id, .. }) => {
                let operation = nodes
                    .iter()
                    .find_map(|candidate| match candidate {
                        Node::EffectIntent { id, operation, .. } if id == effect_node_id => {
                            Some(operation.kind())
                        }
                        _ => None,
                    })
                    .expect("validated receipt branch must reference an effect");
                TypeRef::Receipt { operation }
            }
            _ => unreachable!("referenced-receipt expression belongs only to receipt branch"),
        },
        TypeExpression::TriggerContext { optional, .. } => match node {
            Some(Node::DebounceThrottle { trigger, .. }) => {
                let context = TypeRef::TriggerContext { trigger: *trigger };
                if *optional {
                    TypeRef::Optional {
                        item: Box::new(context),
                    }
                } else {
                    context
                }
            }
            _ => unreachable!("trigger-context expression belongs only to admission state"),
        },
    }
}
