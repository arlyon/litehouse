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
2. Install a signed first-party ESPHome plugin from the central registry.
3. Configure ESPHome locally; discover vendor-neutral typed capabilities across
   switches, lights with brightness/color, sensors, climate, covers, buttons,
   and notifier.
4. Control them through the web UI, small typed React Flow automations, or AI
   using the same management API.
5. Keep web and structured API behavior at parity.

Use the existing WIT/Component Model pipeline and make Rust the first polished
plugin SDK. React Flow is only an editor; a versioned standardized or
standard-informed rule DSL remains a research/decision ticket. Registry installs
and updates are exact-digest pinned, reviewed, and rollbackable. V1 supports one
owner; sharing roles follow later.
