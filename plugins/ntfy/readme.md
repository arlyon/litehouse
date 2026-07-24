# ntfy

A bounded Litehouse `notification-sink@1` provider for one fixed ntfy origin and topic.
It publishes plain UTF-8 bodies with optional title and priority, and reports only
server acceptance. The access token must be injected as `ACCESS_TOKEN` and is kept
in zeroizing runtime storage.
