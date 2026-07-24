# Set deployment and evidence boundaries

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-13
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

What deployment artifact and evidence bar must the decision-complete route
cover?

## Answer

Litehouse remains a portable container with one documented persistent data
volume. A future “Litehouse OS” is only a very thin Butane configurator that
places and configures that container; Litehouse does not bake an OS or own a
CoreOS lifecycle in v1.

Wayfinder ends at an evidence-backed, implementation-ready route. Implementation
requires separate approval. Research is required where standards/ecosystem
facts can invalidate a choice. Disposable prototypes are required only for:

1. typed cross-store service-fabric ownership/routing;
2. a real ESPHome shared-node, multi-facet integration with deterministic
   fixtures; and
3. registry publish/query/install/offline/rollback behavior.

WebRTC, CoreOS, AI, and full product-journey prototypes are not map-completion
gates.
