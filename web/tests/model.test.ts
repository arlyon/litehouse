import { describe, expect, it } from "vitest";
import {
  capabilityFixture,
  recordFixtureIntent,
} from "../src/features/capabilities/fixtures";
import {
  canControl,
  capabilityKinds,
} from "../src/features/capabilities/model";

describe("capability fixture contract", () => {
  it("covers every canonical exact-major renderer once", () => {
    expect(capabilityFixture.map((facet) => facet.kind)).toEqual(
      capabilityKinds,
    );
  });

  it("blocks replay until an ambiguous receipt is reconciled", () => {
    expect(canControl("current", { confidence: "outcome-unknown" })).toBe(false);
    expect(canControl("stale", { confidence: "outcome-unknown" })).toBe(false);
    expect(canControl("current", { confidence: "acknowledged" })).toBe(true);
  });

  it("records accepted delivery without mutating observed state", () => {
    const original = capabilityFixture[0];
    if (original.kind !== "on-off@1") {
      throw new Error("Expected On/Off fixture first");
    }

    const next = recordFixtureIntent(capabilityFixture, {
      type: "on-off.set",
      facetId: original.id,
      value: false,
    });
    const updated = next[0];

    expect(updated.kind).toBe("on-off@1");
    if (updated.kind !== "on-off@1") {
      throw new Error("Expected On/Off fixture after intent");
    }
    expect(updated.value).toBe(true);
    expect(updated.receipt).toEqual({
      confidence: "accepted-for-delivery",
    });
  });
});
