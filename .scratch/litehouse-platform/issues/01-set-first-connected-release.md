# Set the first connected release

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-13
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

What concrete user journey and product boundary should the decision-complete
route target first?

## Answer

The first genuinely usable milestone is a connected platform:

1. Sign in with a basic central account and pair a local Litehouse container by
   short-lived code.
2. Install the signed first-party ESPHome plugin from the central R2 registry.
3. Configure one ESPHome physical-device instance locally and expose only its
   advertised vendor-neutral On/Off, Level, Color, Boolean, Measurement,
   Thermostat, Covering, and Momentary facets; install Tasmota HTTP, ntfy, or the
   non-commercial attributed Open-Meteo provider as separate instances when
   those independent integrations are wanted.
4. Control them through the web UI, bounded Rule JSON/React Flow automations, or
   generated MCP tools using the same canonical management operations.
5. Keep web and structured API behavior at parity.

Use the existing WIT/Component Model pipeline and make Rust the first polished
plugin SDK. Versioned Litehouse Rule JSON is the authored and persisted rule
language; React Flow and the synchronized semantic outline are projections over
that same JSON state. Registry installs and updates are exact-digest pinned,
reviewed, and rollbackable. V1 supports one
owner; sharing roles follow later.
