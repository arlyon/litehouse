use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, Instant},
};

use cel::{
    Context, IdedExpr, Value,
    common::ast::{Expr, LiteralValue, operators},
    parser::Parser,
};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::model::{
    CelCondition, ConditionInput, ContractTypeRef, HARD_MAX_CEL_AST_BYTES,
    HARD_MAX_CEL_MEMORY_BYTES, HARD_MAX_CEL_STEPS, TypeRef,
};

#[derive(Debug, Clone)]
pub(crate) struct CheckedCelProgram {
    expression: IdedExpr,
    inputs: Vec<ConditionInput>,
    static_steps: usize,
    checked_memory_bytes: usize,
    maximum_memory_bytes: usize,
    maximum_evaluation_time: Duration,
}

impl CheckedCelProgram {
    pub(crate) fn inputs(&self) -> &[ConditionInput] {
        &self.inputs
    }

    pub(crate) fn static_steps(&self) -> usize {
        self.static_steps
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CelType {
    Bool,
    String,
    I64,
    U64,
    Timestamp,
    Duration,
    Record(ContractTypeRef),
}

pub(crate) fn check_condition(
    condition: &CelCondition,
    maximum_steps: usize,
    maximum_memory_bytes: usize,
    maximum_evaluation_time_ms: usize,
) -> Result<CheckedCelProgram, String> {
    let normalized = normalize_source(&condition.source);
    let digest = format!("{:x}", Sha256::digest(normalized.as_bytes()));
    if digest != condition.normalized_source_sha256 {
        return Err("normalized CEL source digest does not match source".to_owned());
    }
    let expression = Parser::new()
        .max_recursion_depth(32)
        .parse(&normalized)
        .map_err(|error| format!("CEL parse failed: {error}"))?;
    let ast_bytes = format!("{expression:?}").len();
    if ast_bytes > HARD_MAX_CEL_AST_BYTES {
        return Err(format!(
            "checked CEL AST exceeds {HARD_MAX_CEL_AST_BYTES} bytes"
        ));
    }

    let mut declarations = BTreeMap::new();
    for input in &condition.inputs {
        declarations.insert(input.name.as_str(), admitted_type(&input.type_ref)?);
    }
    declarations.insert("evaluation_time", CelType::Timestamp);
    let declared = declarations.keys().copied().collect::<BTreeSet<_>>();
    for variable in expression.references().variables() {
        if !declared.contains(variable) {
            return Err(format!("CEL variable {variable:?} is not declared"));
        }
    }

    const STATIC_STEPS_PER_MILLISECOND: usize = 1_000;
    let time_charged_steps = maximum_evaluation_time_ms
        .checked_mul(STATIC_STEPS_PER_MILLISECOND)
        .ok_or_else(|| "CEL evaluation-time budget overflowed".to_owned())?;
    let hard_steps = maximum_steps
        .min(HARD_MAX_CEL_STEPS)
        .min(time_charged_steps);
    let mut steps = 0;
    let result = check_expression(&expression, &declarations, &mut steps, hard_steps)?;
    if result != CelType::Bool {
        return Err("CEL condition must produce a Boolean result".to_owned());
    }
    let estimated_memory = ast_bytes
        + normalized.len()
        + condition
            .inputs
            .iter()
            .map(|input| input.name.len() + 32)
            .sum::<usize>();
    let hard_memory = maximum_memory_bytes.min(HARD_MAX_CEL_MEMORY_BYTES);
    if estimated_memory > hard_memory {
        return Err(format!(
            "checked CEL memory estimate exceeds {hard_memory} bytes"
        ));
    }

    Ok(CheckedCelProgram {
        expression,
        inputs: condition.inputs.clone(),
        static_steps: steps,
        checked_memory_bytes: estimated_memory,
        maximum_memory_bytes: hard_memory,
        maximum_evaluation_time: Duration::from_millis(maximum_evaluation_time_ms as u64),
    })
}

pub(crate) fn evaluate_condition(
    program: &CheckedCelProgram,
    variables: BTreeMap<String, Value>,
    evaluation_unix_seconds: i64,
    evaluation_nanosecond: u32,
) -> Result<bool, String> {
    let runtime_memory = program.checked_memory_bytes
        + variables
            .iter()
            .map(|(name, value)| name.len() + value_memory_bytes(value))
            .sum::<usize>();
    if runtime_memory > program.maximum_memory_bytes {
        return Err(format!(
            "CEL runtime memory estimate is {runtime_memory} bytes; maximum is {}",
            program.maximum_memory_bytes
        ));
    }
    let started = Instant::now();
    let mut context = Context::default();
    for input in &program.inputs {
        let value = variables
            .get(&input.name)
            .ok_or_else(|| format!("CEL input {:?} is missing", input.name))?;
        context.add_variable_from_value(input.name.clone(), value.clone());
    }
    let timestamp = DateTime::<Utc>::from_timestamp(evaluation_unix_seconds, evaluation_nanosecond)
        .ok_or_else(|| "evaluation_time is outside CEL timestamp range".to_owned())?
        .fixed_offset();
    context.add_variable_from_value("evaluation_time", Value::Timestamp(timestamp));

    let resolved = context
        .resolve(&program.expression)
        .map_err(|error| error.to_string())?;
    if started.elapsed() > program.maximum_evaluation_time {
        return Err(format!(
            "CEL evaluation exceeded {} ms",
            program.maximum_evaluation_time.as_millis()
        ));
    }
    match resolved {
        Value::Bool(value) => Ok(value),
        value => Err(format!(
            "CEL returned {:?}, expected Boolean",
            value.type_of()
        )),
    }
}

fn value_memory_bytes(value: &Value) -> usize {
    match value {
        Value::List(values) => values.iter().map(value_memory_bytes).sum(),
        Value::Map(map) => map
            .map
            .iter()
            .map(|(key, value)| format!("{key:?}").len() + value_memory_bytes(value))
            .sum(),
        Value::Function(name, receiver) => {
            name.len() + receiver.as_deref().map_or(0, value_memory_bytes)
        }
        Value::Int(_) | Value::UInt(_) | Value::Float(_) => 8,
        Value::String(value) => value.len(),
        Value::Bytes(value) => value.len(),
        Value::Bool(_) | Value::Null => 1,
        Value::Duration(_) | Value::Timestamp(_) => 16,
        Value::Opaque(_) => HARD_MAX_CEL_MEMORY_BYTES + 1,
    }
}

pub(crate) fn normalize_source(source: &str) -> String {
    source.replace("\r\n", "\n").trim().to_owned()
}

fn admitted_type(type_ref: &TypeRef) -> Result<CelType, String> {
    match type_ref {
        TypeRef::Bool => Ok(CelType::Bool),
        TypeRef::String => Ok(CelType::String),
        TypeRef::I64 => Ok(CelType::I64),
        TypeRef::U64 => Ok(CelType::U64),
        TypeRef::Timestamp => Ok(CelType::Timestamp),
        TypeRef::Duration => Ok(CelType::Duration),
        TypeRef::Optional { item } => admitted_type(item),
        TypeRef::Contract { type_ref } => match type_ref {
            ContractTypeRef::ColorModel
            | ContractTypeRef::ThermostatMode
            | ContractTypeRef::ThermostatAction
            | ContractTypeRef::CoveringMovement => Ok(CelType::String),
            ContractTypeRef::OnOffState
            | ContractTypeRef::BooleanState
            | ContractTypeRef::BooleanOccurrence
            | ContractTypeRef::LevelState
            | ContractTypeRef::ColorState
            | ContractTypeRef::MeasurementState
            | ContractTypeRef::ThermostatState
            | ContractTypeRef::CoveringState => Ok(CelType::Record(*type_ref)),
            ContractTypeRef::CanonicalDecimal
            | ContractTypeRef::Quantity
            | ContractTypeRef::NativeRepresentation
            | ContractTypeRef::ColorValue
            | ContractTypeRef::ThermostatTargets => {
                Err("quantity-bearing contract values are not admitted to CEL".to_owned())
            }
        },
        TypeRef::TriggerContext { .. } => {
            Err("whole trigger-context records are not admitted to CEL V1".to_owned())
        }
        TypeRef::Quantity { .. } => Err("quantity values are not admitted to CEL".to_owned()),
        TypeRef::List { .. } => Err("list values are not admitted to CEL V1".to_owned()),
        TypeRef::Receipt { .. }
        | TypeRef::Control
        | TypeRef::PredicateOutcome
        | TypeRef::DesiredPatch
        | TypeRef::EffectIntent
        | TypeRef::ForkToken
        | TypeRef::RunOutcome => Err("runtime values are not admitted to CEL".to_owned()),
    }
}

fn contract_field_type(record: ContractTypeRef, field: &str) -> Result<CelType, String> {
    let common_state = match field {
        "availability" | "quality" => Some(Ok(CelType::String)),
        "meta" => Some(Err(
            "nested observation metadata is not admitted to CEL V1".to_owned()
        )),
        _ => None,
    };
    if let Some(result) = common_state
        && !matches!(record, ContractTypeRef::BooleanOccurrence)
    {
        return result;
    }
    let value = match (record, field) {
        (ContractTypeRef::OnOffState | ContractTypeRef::BooleanState, "current") => CelType::Bool,
        (ContractTypeRef::BooleanOccurrence, "event_id" | "source_id") => CelType::String,
        (ContractTypeRef::BooleanOccurrence, "generation" | "received_at_unix_milliseconds") => {
            CelType::U64
        }
        (ContractTypeRef::BooleanOccurrence, "value") => CelType::Bool,
        (ContractTypeRef::LevelState, "transition_remaining_milliseconds") => CelType::U64,
        (ContractTypeRef::ColorState, "active_model") => CelType::String,
        (
            ContractTypeRef::ThermostatState,
            "current_mode" | "target_mode" | "current_action" | "fan" | "swing" | "preset",
        ) => CelType::String,
        (ContractTypeRef::ThermostatState, "running") => CelType::Bool,
        (ContractTypeRef::LevelState, "current" | "target")
        | (ContractTypeRef::ColorState, "current" | "target")
        | (ContractTypeRef::MeasurementState, "current" | "native")
        | (ContractTypeRef::ThermostatState, "targets") => {
            return Err(format!(
                "contract field {field:?} contains quantity data and is not admitted to CEL"
            ));
        }
        (ContractTypeRef::CoveringState, "axes" | "safety_status" | "fault_status") => {
            return Err(format!(
                "contract field {field:?} contains a collection and is not admitted to CEL"
            ));
        }
        _ => {
            return Err(format!(
                "field {field:?} does not exist on approved contract record {record:?}"
            ));
        }
    };
    Ok(value)
}

fn check_expression(
    expression: &IdedExpr,
    declarations: &BTreeMap<&str, CelType>,
    steps: &mut usize,
    maximum_steps: usize,
) -> Result<CelType, String> {
    *steps += 1;
    if *steps > maximum_steps {
        return Err(format!("CEL static cost exceeds {maximum_steps} steps"));
    }
    match &expression.expr {
        Expr::Literal(value) => match value {
            LiteralValue::Boolean(_) => Ok(CelType::Bool),
            LiteralValue::Int(_) => Ok(CelType::I64),
            LiteralValue::UInt(_) => Ok(CelType::U64),
            LiteralValue::String(_) => Ok(CelType::String),
            LiteralValue::Bytes(_) | LiteralValue::Double(_) | LiteralValue::Null => {
                Err("bytes, floats, and null are not admitted to CEL V1".to_owned())
            }
        },
        Expr::Ident(name) => declarations
            .get(name.as_str())
            .copied()
            .ok_or_else(|| format!("CEL variable {name:?} is not declared")),
        Expr::Select(select) => {
            let operand = check_expression(&select.operand, declarations, steps, maximum_steps)?;
            match operand {
                CelType::Record(record) => contract_field_type(record, &select.field),
                _ => Err("field selection is allowed only on contract records".to_owned()),
            }
        }
        Expr::Call(call) => {
            if call.target.is_some() {
                return Err("CEL method calls are not admitted to CEL V1".to_owned());
            }
            let arguments = call
                .args
                .iter()
                .map(|argument| check_expression(argument, declarations, steps, maximum_steps))
                .collect::<Result<Vec<_>, _>>()?;
            check_call(&call.func_name, &arguments)
        }
        Expr::Comprehension(_) | Expr::List(_) | Expr::Map(_) | Expr::Struct(_) => Err(
            "CEL comprehensions, collections, and constructed records are not admitted".to_owned(),
        ),
        Expr::Unspecified => Err("CEL expression is unspecified".to_owned()),
    }
}

fn check_call(function: &str, arguments: &[CelType]) -> Result<CelType, String> {
    match function {
        operators::LOGICAL_NOT if arguments == [CelType::Bool] => Ok(CelType::Bool),
        operators::LOGICAL_AND | operators::LOGICAL_OR
            if arguments == [CelType::Bool, CelType::Bool] =>
        {
            Ok(CelType::Bool)
        }
        operators::EQUALS | operators::NOT_EQUALS
            if arguments.len() == 2 && comparable(arguments[0], arguments[1]) =>
        {
            Ok(CelType::Bool)
        }
        operators::LESS
        | operators::LESS_EQUALS
        | operators::GREATER
        | operators::GREATER_EQUALS
            if arguments.len() == 2 && ordered(arguments[0], arguments[1]) =>
        {
            Ok(CelType::Bool)
        }
        operators::NEGATE if arguments == [CelType::I64] => Ok(CelType::I64),
        "timestamp" if arguments == [CelType::String] => Ok(CelType::Timestamp),
        _ => Err(format!(
            "CEL function/operator {function:?} is not admitted for {arguments:?}"
        )),
    }
}

fn comparable(left: CelType, right: CelType) -> bool {
    left == right && !matches!(left, CelType::Record(_))
}

fn ordered(left: CelType, right: CelType) -> bool {
    matches!(
        (left, right),
        (CelType::I64, CelType::I64)
            | (CelType::U64, CelType::U64)
            | (CelType::String, CelType::String)
            | (CelType::Timestamp, CelType::Timestamp)
            | (CelType::Duration, CelType::Duration)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn condition(source: &str, inputs: Vec<ConditionInput>) -> CelCondition {
        CelCondition {
            source: source.to_owned(),
            normalized_source_sha256: format!("{:x}", Sha256::digest(source.as_bytes())),
            inputs,
        }
    }

    #[test]
    fn contract_fields_are_closed_and_quantity_free() {
        let state = ConditionInput {
            name: "state".to_owned(),
            type_ref: TypeRef::Contract {
                type_ref: ContractTypeRef::OnOffState,
            },
        };
        assert!(
            check_condition(
                &condition("state.current == true", vec![state.clone()]),
                10_000,
                1024 * 1024,
                10,
            )
            .is_ok()
        );
        assert!(
            check_condition(
                &condition("state.missing == true", vec![state.clone()]),
                10_000,
                1024 * 1024,
                10,
            )
            .unwrap_err()
            .contains("does not exist")
        );
        assert!(
            check_condition(&condition("state", vec![state]), 10_000, 1024 * 1024, 10,)
                .unwrap_err()
                .contains("Boolean")
        );
    }

    #[test]
    fn runtime_values_are_charged_to_the_memory_limit() {
        let inputs = (0..5)
            .map(|index| ConditionInput {
                name: format!("value{index}"),
                type_ref: TypeRef::String,
            })
            .collect::<Vec<_>>();
        let checked = check_condition(
            &condition("value0 == \"expected\"", inputs.clone()),
            10_000,
            64 * 1024,
            10,
        )
        .unwrap();
        let variables = inputs
            .into_iter()
            .map(|input| (input.name, Value::String("x".repeat(15_000).into())))
            .collect();
        assert!(
            evaluate_condition(&checked, variables, 1_700_000_000, 0)
                .unwrap_err()
                .contains("runtime memory")
        );
    }
}
