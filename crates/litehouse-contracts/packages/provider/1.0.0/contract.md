# `litehouse:provider@1.0.0`

This host-facing execution contract lets one configured provider component publish repeated typed facets from one Component Model store. A runner represents one physical provider instance and owns its connection, state cache, and returned facet resources.

`start` receives the stable instance identity and prepared typed configuration. Protected fields may be present only after host-side environment resolution; Litehouse never persists them. Providers must not include protected values in errors, descriptors, or logs.

Facet `key` values are stable provider-owned source locators. `incarnation` changes only when the logical facet is replaced within one provider generation. The host combines these values with its instance identity and generation, retains all guest handles in the provider actor, and publishes handle-free descriptors to consumers.

On/Off, Boolean, Measurement, and Notification Sink methods use execution-free canonical capability types without reflection or a generic dispatch operation. The first release deliberately excludes subscription handles from this execution ABI; providers retain current state and `query` returns the latest truthful observation. Notification sends remain typed side effects with their own receipt contract. The remaining approved capability resource kinds may be completed before the first provider ABI release, but no arbitrary facet envelope is permitted.

Dropping a runner ends the provider generation and invalidates every facet handle it returned. Provider errors use the shared closed error model. Commands follow their capability receipt semantics; accepted delivery is not device confirmation, and an uncertain post-dispatch failure is `outcome-unknown`.
