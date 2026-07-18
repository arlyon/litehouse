# Open-Meteo provider

`open-meteo` is a read-only `wasm32-wasip2` Litehouse provider for current weather-model conditions at one explicitly configured coordinate. It supports only Open-Meteo's free non-commercial endpoint and has no API-key, endpoint, URL, query, forecast, or write configuration.

Configuration requires bounded `latitude` and `longitude`, plus `"usage":"free-noncommercial"` as the operator's explicit acknowledgement that this mode is not for commercial use. `poll_seconds` is optional (900–3600, default 900), as is a bounded `label`.

Every facet carries the non-removable `attribution/open-meteo-cc-by-4.0` descriptor tag. Any UI or export displaying its values must render adjacent attribution equivalent to:

> Weather data by [Open-Meteo.com](https://open-meteo.com/) (normalized by Litehouse), licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).

The artifact must not be released through a display/export path that cannot guarantee that adjacent attribution.

The component enforces the published 500/minute, 4,000/hour, 9,000/day, and 270,000/month safety windows for its own instance. Because the current provider ABI has no authority-wide shared limiter or instance-admission counter, deployment must additionally cap this free profile at 90 enabled instances until the host supplies that shared enforcement.
