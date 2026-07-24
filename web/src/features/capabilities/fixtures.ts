import type { CapabilityFacet, CapabilityIntent } from "./model";

export const capabilityFixture: readonly CapabilityFacet[] = [
  {
    id: "living-room-power",
    kind: "on-off@1",
    label: "Standing lamp power",
    quality: "current",
    value: true,
  },
  {
    id: "living-room-level",
    kind: "level-control@1",
    label: "Standing lamp brightness",
    quality: "current",
    value: 72,
    minimum: 0,
    maximum: 100,
    step: 1,
    receipt: { confidence: "accepted-for-delivery" },
  },
  {
    id: "living-room-color",
    kind: "color-control@1",
    label: "Standing lamp color",
    quality: "current",
    model: "temperature",
    kelvin: 2700,
    minimumKelvin: 2200,
    maximumKelvin: 6500,
  },
  {
    id: "front-door-contact",
    kind: "boolean-state@1",
    label: "Front door contact",
    quality: "current",
    value: false,
    trueLabel: "Open",
    falseLabel: "Closed",
  },
  {
    id: "hall-carbon-dioxide",
    kind: "measurement@1",
    label: "Hall carbon dioxide",
    quality: "stale",
    value: "812",
    unit: "ppm",
    source: "reported",
  },
  {
    id: "hall-thermostat",
    kind: "thermostat@1",
    label: "Hall climate",
    quality: "current",
    mode: "heat",
    targetCelsius: 21.5,
    ambientCelsius: 20.9,
    minimumCelsius: 16,
    maximumCelsius: 28,
    stepCelsius: 0.5,
  },
  {
    id: "bedroom-blind",
    kind: "window-covering@1",
    label: "Bedroom blind",
    quality: "unknown",
    lift: {
      positionPercent: 38,
      positionControl: "discrete",
    },
    supportsStop: true,
    receipt: { confidence: "outcome-unknown" },
  },
  {
    id: "server-identify",
    kind: "momentary-action@1",
    label: "Identify server",
    quality: "current",
    actionLabel: "Invoke once",
  },
  {
    id: "house-notifications",
    kind: "notification-sink@1",
    label: "House notifications",
    quality: "current",
    profile: "plain-text",
    placeholder: "Dinner is ready.",
  },
];

export function recordFixtureIntent(
  facets: readonly CapabilityFacet[],
  intent: CapabilityIntent,
): readonly CapabilityFacet[] {
  assertSupportedIntent(intent);
  return facets.map((facet) =>
    facet.id === intent.facetId
      ? { ...facet, receipt: { confidence: "accepted-for-delivery" } }
      : facet,
  );
}

function assertSupportedIntent(intent: CapabilityIntent): void {
  switch (intent.type) {
    case "on-off.set":
    case "level.set":
    case "color-temperature.set":
    case "thermostat-mode.set":
    case "thermostat-target.set":
    case "window-covering.invoke":
    case "window-covering.position.set":
    case "momentary.invoke":
    case "notification.send":
      return;
    default:
      assertNever(intent);
  }
}

function assertNever(value: never): never {
  throw new Error(`Unsupported fixture intent: ${JSON.stringify(value)}`);
}
