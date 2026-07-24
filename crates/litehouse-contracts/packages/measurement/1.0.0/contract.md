# `litehouse:measurement@1.0.0`

The execution-free `types` interface is shared by capability consumers and the provider-host ABI; executable query/subscription functions remain only in `capability`.

A measurement publishes an optional canonical `litehouse:quantity@1` plus optional source/native representation. Kind is instantaneous, total, or total-increasing. Min/max, resolution, step, tolerance, and native quantization are source claims; display digits are not sensor resolution.

The immutable profile fixes semantic meaning and canonical UCUM unit. Providers MUST map source device class and unit unambiguously to an approved profile. Unknown/ambiguous mappings are diagnostic and MUST NOT publish a misleading facet. Custom profiles and units are unsupported in v1.

Unknown current value, unavailable/stale transport, and source-assumed quality are distinct. Every accepted observation advances revision even when equal. Streams hydrate, cross a caught-up barrier, and report explicit gaps.

Unsupported profile/unit, invalid or non-finite value, dimension/profile mismatch, affine misuse, overflow, inexact result requiring quantization, and out-of-range source data are the exhaustive domain details. Arithmetic and normalization follow the quantity contract; no binary-float or silent-rounding wire value is accepted.
