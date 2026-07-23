use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InteractionTarget {
    #[schemars(
        length(min = 1, max = 128),
        regex(pattern = "^[A-Za-z0-9][A-Za-z0-9_-]{0,127}$")
    )]
    pub facet_id: String,
    #[schemars(range(min = 1))]
    pub facet_generation: u64,
    #[schemars(range(min = 1, max = 30_000))]
    pub timeout_ms: Option<u32>,
    #[schemars(regex(pattern = "^[0-9a-f]{32}$"))]
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum LevelDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ColorDirection {
    Increase,
    Decrease,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum InteractionThermostatMode {
    Off,
    Heat,
    Cool,
    Auto,
    Dry,
    FanOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum InteractionCoveringAxis {
    Lift,
    Tilt,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum InteractionNotificationPriority {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "model", rename_all = "kebab-case", deny_unknown_fields)]
pub enum InteractionColor {
    HueSaturation {
        hue_degrees: String,
        saturation: String,
    },
    CieXy {
        x: String,
        y: String,
    },
    ColorTemperature {
        kelvin: String,
    },
    CalibratedRgb {
        red: String,
        green: String,
        blue: String,
        color_space: InteractionColorSpace,
    },
    UncalibratedRgb {
        red: String,
        green: String,
        blue: String,
    },
    NativeWhiteChannels {
        white: String,
        cold_white: Option<String>,
        warm_white: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum InteractionColorSpace {
    Srgb,
    DisplayP3,
}

/// Closed MCP interaction surface. Every variant corresponds one-for-one with
/// an authoritative capability operation in the management manifest.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", deny_unknown_fields)]
pub enum InteractionRequest {
    #[serde(rename = "capability.on-off.set@1")]
    OnOffSet {
        #[serde(flatten)]
        target: InteractionTarget,
        on: bool,
    },
    #[serde(rename = "capability.on-off.toggle@1")]
    OnOffToggle {
        #[serde(flatten)]
        target: InteractionTarget,
    },
    #[serde(rename = "capability.level.set@1")]
    LevelSet {
        #[serde(flatten)]
        target: InteractionTarget,
        level: String,
    },
    #[serde(rename = "capability.level.step@1")]
    LevelStep {
        #[serde(flatten)]
        target: InteractionTarget,
        step: String,
    },
    #[serde(rename = "capability.level.move@1")]
    LevelMove {
        #[serde(flatten)]
        target: InteractionTarget,
        direction: LevelDirection,
        rate_per_second: String,
    },
    #[serde(rename = "capability.level.stop@1")]
    LevelStop {
        #[serde(flatten)]
        target: InteractionTarget,
    },
    #[serde(rename = "capability.level.transition@1")]
    LevelTransition {
        #[serde(flatten)]
        target: InteractionTarget,
        level: String,
        duration_ms: u32,
    },
    #[serde(rename = "capability.level.set-coupled@1")]
    LevelSetCoupled {
        #[serde(flatten)]
        target: InteractionTarget,
        level: String,
        on_when_nonzero: bool,
    },
    #[serde(rename = "capability.level.step-coupled@1")]
    LevelStepCoupled {
        #[serde(flatten)]
        target: InteractionTarget,
        step: String,
        on_when_nonzero: bool,
    },
    #[serde(rename = "capability.level.move-coupled@1")]
    LevelMoveCoupled {
        #[serde(flatten)]
        target: InteractionTarget,
        direction: LevelDirection,
        rate_per_second: String,
        on_when_nonzero: bool,
    },
    #[serde(rename = "capability.level.transition-coupled@1")]
    LevelTransitionCoupled {
        #[serde(flatten)]
        target: InteractionTarget,
        level: String,
        duration_ms: u32,
        on_when_nonzero: bool,
    },
    #[serde(rename = "capability.color.set@1")]
    ColorSet {
        #[serde(flatten)]
        target: InteractionTarget,
        color: InteractionColor,
    },
    #[serde(rename = "capability.color.step@1")]
    ColorStep {
        #[serde(flatten)]
        target: InteractionTarget,
        color: InteractionColor,
        direction: ColorDirection,
    },
    #[serde(rename = "capability.color.move@1")]
    ColorMove {
        #[serde(flatten)]
        target: InteractionTarget,
        color: InteractionColor,
        direction: ColorDirection,
    },
    #[serde(rename = "capability.color.stop@1")]
    ColorStop {
        #[serde(flatten)]
        target: InteractionTarget,
    },
    #[serde(rename = "capability.color.transition@1")]
    ColorTransition {
        #[serde(flatten)]
        target: InteractionTarget,
        color: InteractionColor,
        duration_ms: u32,
    },
    #[serde(rename = "capability.thermostat.set-mode@1")]
    ThermostatSetMode {
        #[serde(flatten)]
        target: InteractionTarget,
        mode: InteractionThermostatMode,
    },
    #[serde(rename = "capability.thermostat.set-single-target@1")]
    ThermostatSetSingleTarget {
        #[serde(flatten)]
        target: InteractionTarget,
        target_celsius: String,
    },
    #[serde(rename = "capability.thermostat.set-dual-target@1")]
    ThermostatSetDualTarget {
        #[serde(flatten)]
        target: InteractionTarget,
        heat_target_celsius: String,
        cool_target_celsius: String,
    },
    #[serde(rename = "capability.thermostat.set-fan@1")]
    ThermostatSetFan {
        #[serde(flatten)]
        target: InteractionTarget,
        fan_choice: String,
    },
    #[serde(rename = "capability.thermostat.set-swing@1")]
    ThermostatSetSwing {
        #[serde(flatten)]
        target: InteractionTarget,
        swing_choice: String,
    },
    #[serde(rename = "capability.thermostat.set-preset@1")]
    ThermostatSetPreset {
        #[serde(flatten)]
        target: InteractionTarget,
        preset_choice: String,
    },
    #[serde(rename = "capability.covering.open@1")]
    CoveringOpen {
        #[serde(flatten)]
        target: InteractionTarget,
        axis: InteractionCoveringAxis,
    },
    #[serde(rename = "capability.covering.close@1")]
    CoveringClose {
        #[serde(flatten)]
        target: InteractionTarget,
        axis: InteractionCoveringAxis,
    },
    #[serde(rename = "capability.covering.stop@1")]
    CoveringStop {
        #[serde(flatten)]
        target: InteractionTarget,
        axis: InteractionCoveringAxis,
    },
    #[serde(rename = "capability.covering.set-lift@1")]
    CoveringSetLift {
        #[serde(flatten)]
        target: InteractionTarget,
        position: String,
    },
    #[serde(rename = "capability.covering.set-tilt@1")]
    CoveringSetTilt {
        #[serde(flatten)]
        target: InteractionTarget,
        position: String,
    },
    #[serde(rename = "capability.momentary.invoke@1")]
    MomentaryInvoke {
        #[serde(flatten)]
        target: InteractionTarget,
    },
    #[serde(rename = "capability.notification.send@1")]
    NotificationSend {
        #[serde(flatten)]
        target: InteractionTarget,
        notification_template_id: String,
        priority: InteractionNotificationPriority,
        expires_in_ms: Option<u32>,
    },
}

impl InteractionRequest {
    pub fn operation_id(&self) -> &'static str {
        match self {
            Self::OnOffSet { .. } => "capability.on-off.set@1",
            Self::OnOffToggle { .. } => "capability.on-off.toggle@1",
            Self::LevelSet { .. } => "capability.level.set@1",
            Self::LevelStep { .. } => "capability.level.step@1",
            Self::LevelMove { .. } => "capability.level.move@1",
            Self::LevelStop { .. } => "capability.level.stop@1",
            Self::LevelTransition { .. } => "capability.level.transition@1",
            Self::LevelSetCoupled { .. } => "capability.level.set-coupled@1",
            Self::LevelStepCoupled { .. } => "capability.level.step-coupled@1",
            Self::LevelMoveCoupled { .. } => "capability.level.move-coupled@1",
            Self::LevelTransitionCoupled { .. } => "capability.level.transition-coupled@1",
            Self::ColorSet { .. } => "capability.color.set@1",
            Self::ColorStep { .. } => "capability.color.step@1",
            Self::ColorMove { .. } => "capability.color.move@1",
            Self::ColorStop { .. } => "capability.color.stop@1",
            Self::ColorTransition { .. } => "capability.color.transition@1",
            Self::ThermostatSetMode { .. } => "capability.thermostat.set-mode@1",
            Self::ThermostatSetSingleTarget { .. } => "capability.thermostat.set-single-target@1",
            Self::ThermostatSetDualTarget { .. } => "capability.thermostat.set-dual-target@1",
            Self::ThermostatSetFan { .. } => "capability.thermostat.set-fan@1",
            Self::ThermostatSetSwing { .. } => "capability.thermostat.set-swing@1",
            Self::ThermostatSetPreset { .. } => "capability.thermostat.set-preset@1",
            Self::CoveringOpen { .. } => "capability.covering.open@1",
            Self::CoveringClose { .. } => "capability.covering.close@1",
            Self::CoveringStop { .. } => "capability.covering.stop@1",
            Self::CoveringSetLift { .. } => "capability.covering.set-lift@1",
            Self::CoveringSetTilt { .. } => "capability.covering.set-tilt@1",
            Self::MomentaryInvoke { .. } => "capability.momentary.invoke@1",
            Self::NotificationSend { .. } => "capability.notification.send@1",
        }
    }

    pub fn target(&self) -> &InteractionTarget {
        match self {
            Self::OnOffSet { target, .. }
            | Self::OnOffToggle { target }
            | Self::LevelSet { target, .. }
            | Self::LevelStep { target, .. }
            | Self::LevelMove { target, .. }
            | Self::LevelStop { target }
            | Self::LevelTransition { target, .. }
            | Self::LevelSetCoupled { target, .. }
            | Self::LevelStepCoupled { target, .. }
            | Self::LevelMoveCoupled { target, .. }
            | Self::LevelTransitionCoupled { target, .. }
            | Self::ColorSet { target, .. }
            | Self::ColorStep { target, .. }
            | Self::ColorMove { target, .. }
            | Self::ColorStop { target }
            | Self::ColorTransition { target, .. }
            | Self::ThermostatSetMode { target, .. }
            | Self::ThermostatSetSingleTarget { target, .. }
            | Self::ThermostatSetDualTarget { target, .. }
            | Self::ThermostatSetFan { target, .. }
            | Self::ThermostatSetSwing { target, .. }
            | Self::ThermostatSetPreset { target, .. }
            | Self::CoveringOpen { target, .. }
            | Self::CoveringClose { target, .. }
            | Self::CoveringStop { target, .. }
            | Self::CoveringSetLift { target, .. }
            | Self::CoveringSetTilt { target, .. }
            | Self::MomentaryInvoke { target }
            | Self::NotificationSend { target, .. } => target,
        }
    }
}
