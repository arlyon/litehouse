import { createFileRoute } from "@tanstack/react-router";
import { CapabilityReference } from "../features/capabilities/CapabilityReference";

export const Route = createFileRoute("/")({
  component: CapabilityReference,
});
