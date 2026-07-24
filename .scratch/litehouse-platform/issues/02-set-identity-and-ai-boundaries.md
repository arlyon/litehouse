# Set identity and AI access boundaries

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-13
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which identity modes and AI authority boundary should the first connected
release plan around?

## Answer

A server chooses exactly one authentication mode:

- **Cloud-connected:** central Litehouse OAuth/OIDC, short-lived local pairing
  code, website inventory, Cockpit signaling, and direct WebRTC management.
- **Standalone:** one manually configured external OIDC issuer and direct local
  HTTPS API/MCP access; no central inventory or WebRTC signaling.

Authenticated OAuth/OIDC requires HTTPS. Plain HTTP is available only through an
explicit unauthenticated `unsecure-local` flag and is not represented as OAuth.
Device addresses and Noise keys remain local and never enter central systems or
AI context.

AI receives full access to authenticated **Litehouse APIs** through a thin MCP
adapter over the canonical management surface. It does not receive host shell,
arbitrary filesystem/container/network access, raw secrets, ownership transfer,
trust-policy bypass, self-granted permissions, or the ability to weaken
safeguards. A focused ticket must research a lightweight maintained Rust MCP
library.
