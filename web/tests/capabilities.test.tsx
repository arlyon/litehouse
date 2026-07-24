import "@testing-library/jest-dom/vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { CapabilityReference } from "../src/features/capabilities/CapabilityReference";
import { CapabilityTable } from "../src/features/capabilities/CapabilityTable";
import { capabilityFixture } from "../src/features/capabilities/fixtures";
import type { CapabilityFacet } from "../src/features/capabilities/model";

describe("CapabilityTable", () => {
  it("renders every canonical facet kind", () => {
    render(<CapabilityTable facets={capabilityFixture} />);

    for (const facet of capabilityFixture) {
      expect(screen.getByText(facet.kind)).toBeInTheDocument();
    }
  });

  it("does not project unknown On/Off state as Off", () => {
    const unknownPower: CapabilityFacet = {
      id: "unknown-power",
      kind: "on-off@1",
      label: "Unknown power",
      quality: "unknown",
      value: null,
    };

    render(<CapabilityTable facets={[unknownPower]} dispatch={vi.fn()} />);

    expect(screen.getByRole("radio", { name: "On" })).not.toBeChecked();
    expect(screen.getByRole("radio", { name: "Off" })).not.toBeChecked();
    expect(screen.getByRole("radio", { name: "On" })).toBeDisabled();
    expect(screen.getByText("Unknown")).toBeInTheDocument();
  });

  it("disables mutation controls for an unknown covering", () => {
    render(<CapabilityTable facets={capabilityFixture} dispatch={vi.fn()} />);

    expect(screen.getByRole("button", { name: "open lift" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Stop movement" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "close lift" })).toBeDisabled();
    expect(screen.getByText("Outcome unknown")).toBeInTheDocument();
  });

  it("does not expose fabricated sliders for unknown numeric values", () => {
    const unknownLevel: CapabilityFacet = {
      id: "unknown-level",
      kind: "level-control@1",
      label: "Unknown level",
      quality: "unknown",
      value: null,
      minimum: 0,
      maximum: 100,
      step: 1,
    };
    const unknownColor: CapabilityFacet = {
      id: "unknown-color",
      kind: "color-control@1",
      label: "Unknown color",
      quality: "unknown",
      model: "temperature",
      kelvin: null,
      minimumKelvin: 2200,
      maximumKelvin: 6500,
    };

    render(
      <CapabilityTable facets={[unknownLevel, unknownColor]} dispatch={vi.fn()} />,
    );

    expect(screen.queryByRole("slider")).not.toBeInTheDocument();
    expect(screen.getByLabelText("Unknown level value")).toHaveTextContent(
      "Unknown",
    );
    expect(screen.getByLabelText("Unknown color temperature")).toHaveTextContent(
      "Unknown",
    );
  });

  it("gates every advertised covering axis and position operation", () => {
    const covering: CapabilityFacet = {
      id: "dual-axis-covering",
      kind: "window-covering@1",
      label: "Dual-axis covering",
      quality: "current",
      lift: {
        positionPercent: 40,
        positionControl: "settable",
        stepPercent: 5,
      },
      tilt: {
        positionPercent: null,
        positionControl: "discrete",
      },
      supportsStop: false,
    };

    render(<CapabilityTable facets={[covering]} dispatch={vi.fn()} />);

    expect(screen.getByRole("button", { name: "open lift" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "close tilt" })).toBeEnabled();
    expect(screen.getByRole("slider", { name: "Dual-axis covering lift position" })).toHaveValue("40");
    expect(screen.getByText("Tilt position unknown")).toBeInTheDocument();
    expect(screen.getByText("Stop unavailable")).toBeInTheDocument();
  });

  it("blocks explicit replay while an operation outcome is unknown", () => {
    const ambiguousAction: CapabilityFacet = {
      id: "ambiguous-action",
      kind: "momentary-action@1",
      label: "Ambiguous action",
      quality: "current",
      actionLabel: "Invoke once",
      receipt: { confidence: "outcome-unknown" },
    };

    render(
      <CapabilityTable facets={[ambiguousAction]} dispatch={vi.fn()} />,
    );

    expect(screen.getByRole("button", { name: "Invoke once" })).toBeDisabled();
    expect(screen.getByText("Outcome unknown")).toBeInTheDocument();
  });

  it("makes unavailable and offline controls visibly non-actionable", () => {
    const unavailablePower: CapabilityFacet = {
      id: "unavailable-power",
      kind: "on-off@1",
      label: "Unavailable power",
      quality: "unavailable",
      value: true,
      receipt: { confidence: "acknowledged" },
    };
    const offlineAction: CapabilityFacet = {
      id: "offline-action",
      kind: "momentary-action@1",
      label: "Offline action",
      quality: "offline",
      actionLabel: "Invoke once",
      receipt: { confidence: "observed" },
    };

    render(
      <CapabilityTable
        facets={[unavailablePower, offlineAction]}
        dispatch={vi.fn()}
      />,
    );

    expect(screen.getByText("Unavailable")).toBeInTheDocument();
    expect(screen.getByText("Offline")).toBeInTheDocument();
    expect(screen.getByText("Acknowledged")).toBeInTheDocument();
    expect(screen.getByText("Observed")).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "On" })).toBeDisabled();
    expect(screen.getByRole("radio", { name: "Off" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Invoke once" })).toBeDisabled();
  });
});

describe("CapabilityReference", () => {
  it("records intent without changing the observed value", async () => {
    const user = userEvent.setup();
    render(<CapabilityReference />);

    const power = screen.getByRole("group", {
      name: "Standing lamp power",
    });
    const on = within(power).getByRole("radio", { name: "On" });
    const off = within(power).getByRole("radio", { name: "Off" });
    expect(on).toBeChecked();

    await user.click(off);

    expect(on).toBeChecked();
    expect(off).not.toBeChecked();
    expect(
      screen.getByText(
        "Recorded on-off.set; observed values remain unchanged.",
      ),
    ).toBeInTheDocument();
  });

  it("toggles the canonical dark token scope", async () => {
    const user = userEvent.setup();
    render(<CapabilityReference />);

    const toggle = screen.getByRole("button", { name: "Use dark mode" });
    await user.click(toggle);

    expect(document.documentElement).toHaveClass("dark");
    expect(document.documentElement).toHaveAttribute("data-theme", "dark");
    expect(
      screen.getByRole("button", { name: "Use light mode" }),
    ).toHaveAttribute("aria-pressed", "true");
  });

  it("requires notification content before recording an intent", async () => {
    const user = userEvent.setup();
    render(<CapabilityReference />);

    const send = screen.getByRole("button", { name: "Send notification" });
    expect(send).toBeDisabled();

    await user.type(
      screen.getByRole("textbox", { name: "Message" }),
      "Dinner is ready.",
    );
    expect(send).toBeEnabled();
    await user.click(send);

    expect(
      screen.getByText(
        "Recorded notification.send; observed values remain unchanged.",
      ),
    ).toBeInTheDocument();
  });
});
