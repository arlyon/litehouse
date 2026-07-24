# Changelog

## 1.0.0 — 2026-07-18

Completed the pre-release provider execution ABI with repeated typed On/Off, Boolean State, Measurement, Notification Sink, Level Control, Color Control, Thermostat, Window Covering, and Momentary Action resources. The five final capability packages now expose execution-free `types` interfaces reused by the provider ABI. A bounded runner drain preserves Boolean occurrence events and reports overflow explicitly without exposing guest handles. Subscription resources remain intentionally omitted. No previously published provider ABI exists.
