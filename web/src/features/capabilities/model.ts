export type ObservationQuality =
  | "current"
  | "stale"
  | "unknown"
  | "unavailable"
  | "offline";

export type OperationReceipt =
  | { confidence: "accepted-for-delivery" }
  | { confidence: "acknowledged" }
  | { confidence: "observed" }
  | { confidence: "outcome-unknown" };

interface BaseFacet {
  id: string;
  label: string;
  quality: ObservationQuality;
  receipt?: OperationReceipt;
}

export interface OnOffFacet extends BaseFacet {
  kind: "on-off@1";
  value: boolean | null;
}

export interface LevelFacet extends BaseFacet {
  kind: "level-control@1";
  value: number | null;
  minimum: number;
  maximum: number;
  step: number;
}

export interface ColorFacet extends BaseFacet {
  kind: "color-control@1";
  model: "temperature";
  kelvin: number | null;
  minimumKelvin: number;
  maximumKelvin: number;
}

export interface BooleanFacet extends BaseFacet {
  kind: "boolean-state@1";
  value: boolean | null;
  trueLabel: string;
  falseLabel: string;
}

export interface MeasurementFacet extends BaseFacet {
  kind: "measurement@1";
  value: string | null;
  unit: string;
  source: "reported" | "derived";
}

export interface ThermostatFacet extends BaseFacet {
  kind: "thermostat@1";
  mode: "heat" | "off" | null;
  targetCelsius: number | null;
  ambientCelsius: number | null;
  minimumCelsius: number;
  maximumCelsius: number;
  stepCelsius: number;
}

export type CoveringAxis =
  | {
      positionPercent: number | null;
      positionControl: "discrete";
    }
  | {
      positionPercent: number | null;
      positionControl: "settable";
      stepPercent: number;
    };

type CoveringAxes =
  | { lift: CoveringAxis; tilt?: CoveringAxis }
  | { lift?: CoveringAxis; tilt: CoveringAxis };

export type WindowCoveringFacet = BaseFacet &
  CoveringAxes & {
    kind: "window-covering@1";
    supportsStop: boolean;
  };

export interface MomentaryFacet extends BaseFacet {
  kind: "momentary-action@1";
  actionLabel: string;
}

export interface NotificationFacet extends BaseFacet {
  kind: "notification-sink@1";
  profile: "plain-text";
  placeholder: string;
}

export type CapabilityFacet =
  | OnOffFacet
  | LevelFacet
  | ColorFacet
  | BooleanFacet
  | MeasurementFacet
  | ThermostatFacet
  | WindowCoveringFacet
  | MomentaryFacet
  | NotificationFacet;

export type CapabilityIntent =
  | { type: "on-off.set"; facetId: string; value: boolean }
  | { type: "level.set"; facetId: string; value: number }
  | { type: "color-temperature.set"; facetId: string; kelvin: number }
  | { type: "thermostat-mode.set"; facetId: string; mode: "heat" | "off" }
  | { type: "thermostat-target.set"; facetId: string; celsius: number }
  | {
      type: "window-covering.invoke";
      facetId: string;
      axis: "lift" | "tilt";
      command: "open" | "close";
    }
  | {
      type: "window-covering.invoke";
      facetId: string;
      command: "stop";
    }
  | {
      type: "window-covering.position.set";
      facetId: string;
      axis: "lift" | "tilt";
      percent: number;
    }
  | { type: "momentary.invoke"; facetId: string }
  | { type: "notification.send"; facetId: string; message: string };

export type CapabilityDispatcher = (intent: CapabilityIntent) => void;

export const capabilityKinds = [
  "on-off@1",
  "level-control@1",
  "color-control@1",
  "boolean-state@1",
  "measurement@1",
  "thermostat@1",
  "window-covering@1",
  "momentary-action@1",
  "notification-sink@1",
] as const satisfies readonly CapabilityFacet["kind"][];

export function canControl(
  quality: ObservationQuality,
  receipt?: OperationReceipt,
): boolean {
  return (
    receipt?.confidence !== "outcome-unknown" &&
    (quality === "current" || quality === "stale")
  );
}
