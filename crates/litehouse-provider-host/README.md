# Litehouse provider host

Production Wasmtime adapter for `litehouse:provider@1`. It retains guest facet resources in one per-instance actor/store and projects only typed host-owned identities, descriptors, states, and receipts.

This crate does not compile components, persist configuration, authorize management callers, expose transport, or own registry policy. The Litehouse daemon supplies its existing engine/cache, exact TCP grants, canonical handlers, and service-fabric instance.
