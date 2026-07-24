# `litehouse:notification-sink@1.0.0`

The execution-free `types` interface is shared by capability consumers and the provider-host ABI; executable send functions remain only in `capability`.

Notification Sink sends bounded plain-text body with optional title, priority, and expiry. The descriptor advertises exactly which optional fields, byte bounds, deduplication, deadline, and maximum receipt confidence are supported. Unsupported fields are rejected, never discarded.

`send` is a side effect and is not automatically retried. Caller idempotency context is honored only when the descriptor advertises a matching guarantee. Expired messages are rejected before dispatch where determinable. Success returns a typed receipt; accepted delivery never implies presentation, and provider acknowledgement never implies user attention. Loss after possible dispatch is `outcome-unknown`.

Invalid/oversized content, unsupported field, rate limit, expired message, and provider rejection are exhaustive domain details. Retry-after guidance does not by itself authorize retry. Response choices, replies, scheduling, cancellation, presentation lifecycle, and fabric scoping are outside v1. Provider selection is not part of this vendor-neutral contract.
