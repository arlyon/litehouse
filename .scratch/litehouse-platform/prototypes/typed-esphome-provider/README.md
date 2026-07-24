# Typed ESPHome provider resource spike

This disposable spike answers one question before the production ESPHome provider is changed:

> Can one Component Model guest instance return arbitrarily many opaque, typed
> subresources for facets discovered on one physical ESPHome node, while keeping
> every handle inside the provider store?

The fixture models one node with two switch entities and one LED entity. The guest returns:

- three On/Off resources (`switch-a`, `switch-b`, and `led-power`);
- one Level resource (`led-brightness`); and
- one Color resource (`led-rgb`).

All LED resources share one guest-owned node state. The host receives typed
Canonical ABI resource handles, invokes them in the provider store, and publishes
only handle-free `EndpointDescriptor` metadata into the Litehouse service-fabric
registry.

This spike does not contact ESPHome hardware, open a socket, define the final
provider ABI, or duplicate the protocol/mapping evidence already captured by
`../esphome-capabilities`.

Run `./check.sh` from this directory.
