# `litehouse:quantity@1.0.0`

A quantity is a built-in immutable profile ID, a canonical decimal magnitude, and that profile's canonical case-sensitive UCUM 2.2 code. Magnitudes are bounded decimal strings, never JSON numbers or binary floats. Providers cannot invent profiles or canonical units in v1.

The 1.0 catalog contains temperature, temperature difference, relative humidity, generic ratio, WMO weather interpretation code, pressure, illuminance, power, energy, voltage, electric current, frequency, battery ratio, signal strength, carbon dioxide, carbon monoxide, volatile organic compounds, PM1, PM2.5, PM10, distance, speed, volume, and volumetric flow. The WMO weather-code profile is a discrete integer in `[0, 99]`, not a normalized ratio.

Input may use bounded UCUM expressions, but output uses the catalog canonical code. `J/s` and `kg.m2/s3` normalize to power in `W`; `%` normalizes to ratio in `1`. Addition/subtraction require the same semantic profile. Multiplication, division, and bounded integer powers succeed only when the result maps to a built-in profile.

Absolute temperature and temperature difference are distinct. Absolute values use `K`; affine offsets apply only to absolute conversion, and absolute minus absolute produces a difference. Unknown units/profiles, annotations, custom/logarithmic units, NaN/infinity, implicit rounding, overflow, and unbounded expressions MUST be rejected. Recurring results require explicit scale and round-half-even quantization with audit metadata.

`quantity/profiles.json` is the release manifest. Existing profile meanings and canonical units never change within major 1.
