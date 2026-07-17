use anyhow::{Context, Result, anyhow, bail};
use litehouse_contracts::quantity::{ErrorCode as NativeErrorCode, Quantity, profile_ids};
use std::path::PathBuf;
use wasmtime::{
    Engine, Store,
    component::{Component, Linker, ResourceTable},
};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

wasmtime::component::bindgen!({
    path: "../litehouse-contracts/packages/quantity/1.0.0/wit",
    world: "quantity-engine",
});

use exports::litehouse::quantity::engine::SourceSpan as WireSourceSpan;
use litehouse::quantity::types::Quantity as WireQuantity;

struct State {
    table: ResourceTable,
    context: WasiCtx,
}

impl WasiView for State {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.context,
            table: &mut self.table,
        }
    }
}

fn main() -> Result<()> {
    let component_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .context("usage: litehouse-quantity-parity <component.wasm>")?;
    let mut configuration = wasmtime::Config::new();
    configuration.wasm_component_model(true);
    configuration.wasm_component_model_async(true);
    let engine = Engine::new(&configuration)?;
    let component = Component::from_file(&engine, &component_path)
        .map_err(|error| anyhow!("load {}: {error}", component_path.display()))?;
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    let mut store = Store::new(
        &engine,
        State {
            table: ResourceTable::new(),
            context: WasiCtxBuilder::new().build(),
        },
    );
    let bindings = QuantityEngine::instantiate(&mut store, &component, &linker)?;
    let guest = bindings.litehouse_quantity_engine();

    let valid = [
        (profile_ids::POWER, "1", "J/s"),
        (profile_ids::POWER, "1", "kg.m2/s3"),
        (profile_ids::RELATIVE_HUMIDITY, "50", "%"),
        (profile_ids::TEMPERATURE_ABSOLUTE, "0", "Cel"),
        (profile_ids::ENERGY, "1", "kWh"),
        (profile_ids::RATIO, "25", "%"),
        (profile_ids::SIGNAL_STRENGTH, "-42", "[dBm]"),
    ];
    for (profile, magnitude, unit) in valid {
        let native = Quantity::parse(profile, magnitude, unit)?;
        let wasm = guest
            .call_normalize(&mut store, profile, magnitude, unit)?
            .map_err(|error| anyhow!("Wasm quantity error: {:?}", error.code))?;
        assert_quantity(&native, &wasm, "normalize")?;
    }

    let invalid = [
        ("litehouse:quantity/missing@1", "1", "W"),
        (profile_ids::POWER, "NaN", "W"),
        (profile_ids::POWER, "1", "V"),
        (profile_ids::RATIO, "1.1", "1"),
    ];
    for (profile, magnitude, unit) in invalid {
        let native = Quantity::parse(profile, magnitude, unit).unwrap_err();
        let wasm = guest
            .call_normalize(&mut store, profile, magnitude, unit)?
            .expect_err("invalid vector must fail in Wasm");
        assert_error(&native.code, native.span, &wasm.code, wasm.span.as_ref())?;
    }

    let ratio_high = Quantity::parse(profile_ids::RATIO, "0.8", "1")?;
    let ratio_low = Quantity::parse(profile_ids::RATIO, "0.4", "1")?;
    let native_ratio = ratio_high.checked_add(&ratio_low).unwrap_err();
    let wasm_ratio = guest
        .call_add(&mut store, &to_wire(&ratio_high), &to_wire(&ratio_low))?
        .expect_err("out-of-range ratio arithmetic must fail");
    assert_error(
        &native_ratio.code,
        native_ratio.span,
        &wasm_ratio.code,
        wasm_ratio.span.as_ref(),
    )?;

    let signal = Quantity::parse(profile_ids::SIGNAL_STRENGTH, "-42", "[dBm]")?;
    let ratio_half = Quantity::parse(profile_ids::RATIO, "0.5", "1")?;
    let native_signal = signal.checked_add(&signal).unwrap_err();
    let wasm_signal = guest
        .call_add(&mut store, &to_wire(&signal), &to_wire(&signal))?
        .expect_err("logarithmic addition must fail");
    assert_error(
        &native_signal.code,
        native_signal.span,
        &wasm_signal.code,
        wasm_signal.span.as_ref(),
    )?;
    let native_signal = signal.checked_sub(&signal).unwrap_err();
    let wasm_signal = guest
        .call_subtract(&mut store, &to_wire(&signal), &to_wire(&signal))?
        .expect_err("logarithmic subtraction must fail");
    assert_error(
        &native_signal.code,
        native_signal.span,
        &wasm_signal.code,
        wasm_signal.span.as_ref(),
    )?;
    let native_signal = signal.checked_mul(&ratio_half).unwrap_err();
    let wasm_signal = guest
        .call_multiply(&mut store, &to_wire(&signal), &to_wire(&ratio_half))?
        .expect_err("logarithmic multiplication must fail");
    assert_error(
        &native_signal.code,
        native_signal.span,
        &wasm_signal.code,
        wasm_signal.span.as_ref(),
    )?;
    let native_signal = signal.checked_div(&ratio_half).unwrap_err();
    let wasm_signal = guest
        .call_divide(&mut store, &to_wire(&signal), &to_wire(&ratio_half))?
        .expect_err("logarithmic division must fail");
    assert_error(
        &native_signal.code,
        native_signal.span,
        &wasm_signal.code,
        wasm_signal.span.as_ref(),
    )?;
    let native_signal = signal.checked_powi(1).unwrap_err();
    let wasm_signal = guest
        .call_power(&mut store, &to_wire(&signal), 1)?
        .expect_err("logarithmic power must fail");
    assert_error(
        &native_signal.code,
        native_signal.span,
        &wasm_signal.code,
        wasm_signal.span.as_ref(),
    )?;

    let power = Quantity::parse(profile_ids::POWER, "1", "W")?;
    let ratio = Quantity::parse(profile_ids::RATIO, "0.3", "1")?;
    let native_inexact = power.checked_div(&ratio).unwrap_err();
    let wasm_inexact = guest
        .call_divide(&mut store, &to_wire(&power), &to_wire(&ratio))?
        .expect_err("recurring division must require quantization");
    assert_error(
        &native_inexact.code,
        native_inexact.span,
        &wasm_inexact.code,
        wasm_inexact.span.as_ref(),
    )?;

    let native_division = power.checked_div_quantized(&ratio, 2)?;
    let wasm_division = guest
        .call_divide_quantized(&mut store, &to_wire(&power), &to_wire(&ratio), 2)?
        .map_err(|error| anyhow!("Wasm divide-quantized error: {:?}", error.code))?;
    assert_quantity(&native_division.quantity, &wasm_division.value, "divide")?;
    if native_division.audit.inexact != wasm_division.audit.inexact
        || native_division.audit.tie != wasm_division.audit.tie
        || native_division.audit.requested_scale != wasm_division.audit.requested_scale
        || format!("{:?}", native_division.audit.adjustment)
            != adjustment_debug_name(&wasm_division.audit.adjustment)
    {
        bail!("native/Wasm quantization audit drift");
    }

    let decimal_power = Quantity::parse(profile_ids::POWER, "1.235", "W")?;
    let native_power = decimal_power.checked_powi_quantized(1, 2)?;
    let wasm_power = guest
        .call_power_quantized(&mut store, &to_wire(&decimal_power), 1, 2)?
        .map_err(|error| anyhow!("Wasm power-quantized error: {:?}", error.code))?;
    assert_quantity(&native_power.quantity, &wasm_power.value, "power")?;

    let malformed = WireQuantity {
        profile: profile_ids::POWER.to_string(),
        magnitude: "1.0".to_string(),
        unit: "W".to_string(),
    };
    let wasm_malformed = guest
        .call_multiply(&mut store, &malformed, &to_wire(&ratio))?
        .expect_err("noncanonical wire quantity must fail");
    if debug_variant(&wasm_malformed.code) != debug_variant(&NativeErrorCode::InvalidDecimal) {
        bail!("Wasm accepted a noncanonical arithmetic operand");
    }

    println!(
        "quantity native/Wasm parity valid ({} normalize, {} invalid, arithmetic and audit vectors)",
        valid.len(),
        invalid.len()
    );
    Ok(())
}

fn to_wire(value: &Quantity) -> WireQuantity {
    WireQuantity {
        profile: value.profile_id().as_str().to_string(),
        magnitude: value.magnitude().to_string(),
        unit: value.unit().to_string(),
    }
}

fn assert_quantity(native: &Quantity, wasm: &WireQuantity, operation: &str) -> Result<()> {
    if wasm.profile != native.profile_id().as_str()
        || wasm.magnitude != native.magnitude().to_string()
        || wasm.unit != native.unit()
    {
        bail!(
            "native/Wasm {operation} drift: native={} {}, wasm={} {}",
            native.magnitude(),
            native.unit(),
            wasm.magnitude,
            wasm.unit
        );
    }
    Ok(())
}

fn assert_error<W: std::fmt::Debug>(
    native_code: &NativeErrorCode,
    native_span: Option<litehouse_contracts::quantity::SourceSpan>,
    wasm_code: &W,
    wasm_span: Option<&WireSourceSpan>,
) -> Result<()> {
    if debug_variant(native_code) != debug_variant(wasm_code) {
        bail!("native/Wasm error-code drift: {native_code:?} != {wasm_code:?}");
    }
    let native_span = native_span.map(|span| (span.start, span.end));
    let wasm_span = wasm_span.map(|span| (span.start, span.end));
    if native_span != wasm_span {
        bail!("native/Wasm diagnostic-span drift: {native_span:?} != {wasm_span:?}");
    }
    Ok(())
}

fn debug_variant(value: &impl std::fmt::Debug) -> String {
    format!("{value:?}")
        .rsplit("::")
        .next()
        .unwrap_or_default()
        .to_string()
}

fn adjustment_debug_name(value: &str) -> String {
    value
        .split('-')
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}
