import {
  BellRing,
  Blinds,
  DoorClosed,
  Gauge,
  MousePointerClick,
  Palette,
  Power,
  SlidersHorizontal,
  Thermometer,
  type LucideIcon,
} from "lucide-react";
import { useState } from "react";
import type {
  BooleanFacet,
  CapabilityDispatcher,
  CapabilityFacet,
  ColorFacet,
  CoveringAxis,
  LevelFacet,
  MeasurementFacet,
  MomentaryFacet,
  NotificationFacet,
  ObservationQuality,
  OnOffFacet,
  OperationReceipt,
  ThermostatFacet,
  WindowCoveringFacet,
} from "./model";
import { canControl } from "./model";

interface CapabilityTableProps {
  facets: readonly CapabilityFacet[];
  dispatch?: CapabilityDispatcher;
}

const iconByKind: Record<CapabilityFacet["kind"], LucideIcon> = {
  "on-off@1": Power,
  "level-control@1": SlidersHorizontal,
  "color-control@1": Palette,
  "boolean-state@1": DoorClosed,
  "measurement@1": Gauge,
  "thermostat@1": Thermometer,
  "window-covering@1": Blinds,
  "momentary-action@1": MousePointerClick,
  "notification-sink@1": BellRing,
};

const qualityLabel: Record<ObservationQuality, string> = {
  current: "Current",
  stale: "Stale",
  unknown: "Unknown",
  unavailable: "Unavailable",
  offline: "Offline",
};

export function CapabilityTable({ facets, dispatch }: CapabilityTableProps) {
  return (
    <section className="capability-section" aria-labelledby="capability-heading">
      <header className="capability-heading">
        <div>
          <p className="eyebrow">Canonical renderer coverage</p>
          <h2 id="capability-heading">Nine typed capability facets</h2>
        </div>
        <p className="capability-heading-copy">
          Controls project typed view models. They do not claim physical state
          changes without a later observation.
        </p>
      </header>

      <div className="capability-list">
        {facets.map((facet) => (
          <CapabilityRow key={facet.id} facet={facet} dispatch={dispatch} />
        ))}
      </div>
    </section>
  );
}

function CapabilityRow({
  facet,
  dispatch,
}: {
  facet: CapabilityFacet;
  dispatch?: CapabilityDispatcher;
}) {
  const Icon = iconByKind[facet.kind];

  return (
    <article className="capability-row" aria-labelledby={`${facet.id}-label`}>
      <div className="capability-identity">
        <span className="capability-icon" aria-hidden="true">
          <Icon />
        </span>
        <div>
          <h3 id={`${facet.id}-label`}>{facet.label}</h3>
          <code>{facet.kind}</code>
        </div>
      </div>

      <div className="capability-control">
        {renderControl(facet, dispatch)}
        <div className="capability-signals">
          <QualityBadge quality={facet.quality} />
          {facet.receipt ? <ReceiptBadge receipt={facet.receipt} /> : null}
        </div>
      </div>
    </article>
  );
}

function renderControl(
  facet: CapabilityFacet,
  dispatch: CapabilityDispatcher | undefined,
) {
  switch (facet.kind) {
    case "on-off@1":
      return <OnOffControl facet={facet} dispatch={dispatch} />;
    case "level-control@1":
      return <LevelControl facet={facet} dispatch={dispatch} />;
    case "color-control@1":
      return <ColorControl facet={facet} dispatch={dispatch} />;
    case "boolean-state@1":
      return <BooleanValue facet={facet} />;
    case "measurement@1":
      return <MeasurementValue facet={facet} />;
    case "thermostat@1":
      return <ThermostatControl facet={facet} dispatch={dispatch} />;
    case "window-covering@1":
      return <WindowCoveringControl facet={facet} dispatch={dispatch} />;
    case "momentary-action@1":
      return <MomentaryControl facet={facet} dispatch={dispatch} />;
    case "notification-sink@1":
      return <NotificationControl facet={facet} dispatch={dispatch} />;
    default:
      return assertNever(facet);
  }
}

function OnOffControl({
  facet,
  dispatch,
}: {
  facet: OnOffFacet;
  dispatch?: CapabilityDispatcher;
}) {
  const disabled = !dispatch || !canControl(facet.quality, facet.receipt);

  return (
    <fieldset className="segmented-control" disabled={disabled}>
      <legend className="sr-only">{facet.label}</legend>
      <label>
        <input
          type="radio"
          name={facet.id}
          checked={facet.value === true}
          onChange={() =>
            dispatch?.({ type: "on-off.set", facetId: facet.id, value: true })
          }
        />
        On
      </label>
      <label>
        <input
          type="radio"
          name={facet.id}
          checked={facet.value === false}
          onChange={() =>
            dispatch?.({ type: "on-off.set", facetId: facet.id, value: false })
          }
        />
        Off
      </label>
    </fieldset>
  );
}

function LevelControl({
  facet,
  dispatch,
}: {
  facet: LevelFacet;
  dispatch?: CapabilityDispatcher;
}) {
  if (facet.value === null) {
    return (
      <output className="unknown-control" aria-label={`${facet.label} value`}>
        Unknown
      </output>
    );
  }

  const disabled = !dispatch || !canControl(facet.quality, facet.receipt);

  return (
    <label className="range-control">
      <span className="sr-only">{facet.label}</span>
      <input
        type="range"
        min={facet.minimum}
        max={facet.maximum}
        step={facet.step}
        value={facet.value}
        disabled={disabled}
        onChange={(event) =>
          dispatch?.({
            type: "level.set",
            facetId: facet.id,
            value: event.currentTarget.valueAsNumber,
          })
        }
      />
      <output>{facet.value}%</output>
    </label>
  );
}

function ColorControl({
  facet,
  dispatch,
}: {
  facet: ColorFacet;
  dispatch?: CapabilityDispatcher;
}) {
  if (facet.kelvin === null) {
    return (
      <div className="color-control">
        <span className="control-model">Color temperature</span>
        <output
          className="unknown-control"
          aria-label={`${facet.label} temperature`}
        >
          Unknown
        </output>
      </div>
    );
  }

  const disabled = !dispatch || !canControl(facet.quality, facet.receipt);

  return (
    <div className="color-control">
      <span className="control-model">Color temperature</span>
      <label className="range-control">
        <span className="sr-only">{facet.label} temperature</span>
        <input
          type="range"
          min={facet.minimumKelvin}
          max={facet.maximumKelvin}
          value={facet.kelvin}
          disabled={disabled}
          onChange={(event) =>
            dispatch?.({
              type: "color-temperature.set",
              facetId: facet.id,
              kelvin: event.currentTarget.valueAsNumber,
            })
          }
        />
        <output>{facet.kelvin} K</output>
      </label>
    </div>
  );
}

function BooleanValue({ facet }: { facet: BooleanFacet }) {
  const value =
    facet.value === null
      ? "Unknown"
      : facet.value
        ? facet.trueLabel
        : facet.falseLabel;
  return <strong className="observed-value">{value}</strong>;
}

function MeasurementValue({ facet }: { facet: MeasurementFacet }) {
  return (
    <div className="measurement-value">
      <strong>
        {facet.value === null ? "Unknown" : `${facet.value} ${facet.unit}`}
      </strong>
      <span>Source {facet.source}</span>
    </div>
  );
}

function ThermostatControl({
  facet,
  dispatch,
}: {
  facet: ThermostatFacet;
  dispatch?: CapabilityDispatcher;
}) {
  const disabled = !dispatch || !canControl(facet.quality, facet.receipt);
  const targetDisabled = disabled || facet.targetCelsius === null;

  return (
    <div className="thermostat-control">
      <fieldset className="segmented-control" disabled={disabled}>
        <legend className="sr-only">{facet.label} mode</legend>
        <label>
          <input
            type="radio"
            name={`${facet.id}-mode`}
            checked={facet.mode === "heat"}
            onChange={() =>
              dispatch?.({
                type: "thermostat-mode.set",
                facetId: facet.id,
                mode: "heat",
              })
            }
          />
          Heat
        </label>
        <label>
          <input
            type="radio"
            name={`${facet.id}-mode`}
            checked={facet.mode === "off"}
            onChange={() =>
              dispatch?.({
                type: "thermostat-mode.set",
                facetId: facet.id,
                mode: "off",
              })
            }
          />
          Off
        </label>
      </fieldset>

      <div className="stepper-control">
        <button
          type="button"
          disabled={targetDisabled}
          aria-label={`Decrease ${facet.label} target`}
          onClick={() =>
            dispatch?.({
              type: "thermostat-target.set",
              facetId: facet.id,
              celsius: Math.max(
                facet.minimumCelsius,
                (facet.targetCelsius ?? facet.minimumCelsius) -
                  facet.stepCelsius,
              ),
            })
          }
        >
          −
        </button>
        <output>
          {facet.targetCelsius === null
            ? "Unknown"
            : `${facet.targetCelsius.toFixed(1)} °C`}
        </output>
        <button
          type="button"
          disabled={targetDisabled}
          aria-label={`Increase ${facet.label} target`}
          onClick={() =>
            dispatch?.({
              type: "thermostat-target.set",
              facetId: facet.id,
              celsius: Math.min(
                facet.maximumCelsius,
                (facet.targetCelsius ?? facet.maximumCelsius) +
                  facet.stepCelsius,
              ),
            })
          }
        >
          +
        </button>
      </div>

      <span className="ambient-value">
        Ambient{" "}
        {facet.ambientCelsius === null
          ? "unknown"
          : `${facet.ambientCelsius.toFixed(1)} °C`}
      </span>
    </div>
  );
}

function WindowCoveringControl({
  facet,
  dispatch,
}: {
  facet: WindowCoveringFacet;
  dispatch?: CapabilityDispatcher;
}) {
  const disabled = !dispatch || !canControl(facet.quality, facet.receipt);

  return (
    <div className="covering-control">
      <div className="covering-axes">
        {facet.lift ? (
          <CoveringAxisControl
            axis="lift"
            value={facet.lift}
            facet={facet}
            dispatch={dispatch}
            disabled={disabled}
          />
        ) : (
          <span className="feature-unavailable">Lift unavailable</span>
        )}
        {facet.tilt ? (
          <CoveringAxisControl
            axis="tilt"
            value={facet.tilt}
            facet={facet}
            dispatch={dispatch}
            disabled={disabled}
          />
        ) : (
          <span className="feature-unavailable">Tilt unavailable</span>
        )}
      </div>
      {facet.supportsStop ? (
        <button
          type="button"
          disabled={disabled}
          onClick={() =>
            dispatch?.({
              type: "window-covering.invoke",
              facetId: facet.id,
              command: "stop",
            })
          }
        >
          Stop movement
        </button>
      ) : (
        <span className="feature-unavailable">Stop unavailable</span>
      )}
    </div>
  );
}

function CoveringAxisControl({
  axis,
  value,
  facet,
  dispatch,
  disabled,
}: {
  axis: "lift" | "tilt";
  value: CoveringAxis;
  facet: WindowCoveringFacet;
  dispatch?: CapabilityDispatcher;
  disabled: boolean;
}) {
  const axisLabel = axis[0].toUpperCase() + axis.slice(1);

  return (
    <div className="covering-axis">
      <span className="control-model">{axisLabel}</span>
      <div className="button-group">
        {(["open", "close"] as const).map((command) => (
          <button
            key={command}
            type="button"
            disabled={disabled}
            aria-label={`${command} ${axis}`}
            onClick={() =>
              dispatch?.({
                type: "window-covering.invoke",
                facetId: facet.id,
                axis,
                command,
              })
            }
          >
            {command[0].toUpperCase() + command.slice(1)}
          </button>
        ))}
      </div>
      {value.positionControl === "settable" &&
      value.positionPercent !== null ? (
        <label className="range-control covering-position">
          <span className="sr-only">{facet.label} {axis} position</span>
          <input
            type="range"
            min="0"
            max="100"
            step={value.stepPercent}
            value={value.positionPercent}
            aria-label={`${facet.label} ${axis} position`}
            disabled={disabled}
            onChange={(event) =>
              dispatch?.({
                type: "window-covering.position.set",
                facetId: facet.id,
                axis,
                percent: event.currentTarget.valueAsNumber,
              })
            }
          />
          <output>{value.positionPercent}%</output>
        </label>
      ) : null}
      <span className="observed-value">
        {value.positionPercent === null
          ? `${axisLabel} position unknown`
          : `${axisLabel} ${value.positionPercent}%`}
      </span>
    </div>
  );
}

function MomentaryControl({
  facet,
  dispatch,
}: {
  facet: MomentaryFacet;
  dispatch?: CapabilityDispatcher;
}) {
  return (
    <button
      className="primary-action"
      type="button"
      disabled={!dispatch || !canControl(facet.quality, facet.receipt)}
      onClick={() =>
        dispatch?.({ type: "momentary.invoke", facetId: facet.id })
      }
    >
      {facet.actionLabel}
    </button>
  );
}

function NotificationControl({
  facet,
  dispatch,
}: {
  facet: NotificationFacet;
  dispatch?: CapabilityDispatcher;
}) {
  const [message, setMessage] = useState("");
  const disabled = !dispatch || !canControl(facet.quality, facet.receipt);

  return (
    <div className="notification-control">
      <label>
        <span>Message</span>
        <textarea
          rows={2}
          value={message}
          disabled={disabled}
          placeholder={facet.placeholder}
          onChange={(event) => setMessage(event.currentTarget.value)}
        />
      </label>
      <button
        className="primary-action"
        type="button"
        disabled={disabled || message.trim().length === 0}
        onClick={() => {
          dispatch?.({
            type: "notification.send",
            facetId: facet.id,
            message: message.trim(),
          });
          setMessage("");
        }}
      >
        Send notification
      </button>
    </div>
  );
}

function QualityBadge({ quality }: { quality: ObservationQuality }) {
  return (
    <span className={`status-badge status-${quality}`}>
      {qualityLabel[quality]}
    </span>
  );
}

function ReceiptBadge({ receipt }: { receipt: OperationReceipt }) {
  const labels: Record<OperationReceipt["confidence"], string> = {
    "accepted-for-delivery": "Accepted for delivery",
    acknowledged: "Acknowledged",
    observed: "Observed",
    "outcome-unknown": "Outcome unknown",
  };

  return (
    <span
      className={`receipt-badge receipt-${receipt.confidence}`}
      role="status"
    >
      {labels[receipt.confidence]}
    </span>
  );
}

function assertNever(value: never): never {
  throw new Error(`Unsupported capability facet: ${JSON.stringify(value)}`);
}
