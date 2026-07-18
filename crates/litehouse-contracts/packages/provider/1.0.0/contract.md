# `litehouse:provider@1.0.0`

This host-facing execution contract lets one configured provider component publish repeated typed facets from one Component Model store. A runner represents one physical provider instance and owns its connection, state cache, and returned facet resources.

`start` receives the stable instance identity and prepared typed configuration. Protected fields may be present only after host-side environment resolution; Litehouse never persists them. Providers must not include protected values in errors, descriptors, or logs.

Facet `key` values are stable provider-owned source locators. `incarnation` changes only when the logical facet is replaced within one provider generation. The host combines these values with its instance identity and generation, retains all guest handles in the provider actor, and publishes handle-free descriptors to consumers.

The pre-release ABI includes repeated typed On/Off, Boolean State, Measurement, Notification Sink, Level Control, Color Control, Thermostat, Window Covering, and Momentary Action resources. Every resource uses its canonical capability's execution-free `types` interface. Queryable resources expose `describe` and `query`; command resources expose every canonical typed command. Subscription handles remain deliberately excluded from this provider execution ABI, so providers retain current state and `query` returns the latest truthful observation. No arbitrary facet or command envelope is permitted.

Dropping a runner ends the provider generation and invalidates every facet handle it returned. Provider errors use the shared closed error model. Commands preserve their typed operation intent and capability receipt semantics; accepted delivery is not device confirmation, and an uncertain post-dispatch failure is `outcome-unknown`.
