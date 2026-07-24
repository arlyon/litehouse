# Litehouse

> Category: Home Automation
> A unified public-site and product-control-plane system. Quietly capable, direct, and trustworthy, with green operational signals, warm-neutral structure, and two deliberate information densities.

## 1. Visual Theme & Atmosphere

Litehouse feels like a dependable local instrument, not an enterprise dashboard. The public site uses broad breathing room, a visible one-pixel grid, and occasional green field details. Registry, documentation, pairing, and Cockpit surfaces tighten the same vocabulary into practical rows, fields, tables, and state markers.

The physical scene is a home operator checking a local server in normal room light, then opening denser logs when something needs attention. Light mode is the default canvas. Dark mode supports low-light operation without turning the product into a neon command center.

Preserve these source-backed signatures:

- Warm black and stone neutrals from the declarations that win the `globals.css` cascade.
- Green as a capability and connection signal, not a decorative wash.
- Hairline borders, square grid divisions, rounded controls, and restrained card elevation.
- Inter for public and product copy; monospace only for identifiers, versions, commands, and logs.
- Public composition can expand to 60px display type. Product controls stay mostly between 12px and 24px.

## 2. Color

### Winning light roles

| Role | Value | Use |
|---|---|---|
| Canvas | `#FFFFFF` | Page background. This intentionally preserves the current computed value. |
| Surface | `#FFFFFF` | Cards, fields, popovers. |
| Warm surface | `#FAFAF9` | Sidebars and quiet secondary regions. |
| Foreground | `#0C0A09` | Primary text and strong focus indication. |
| Secondary foreground | `#1C1917` | Labels and neutral primary controls. |
| Muted foreground | `#79716B` | Supporting copy and metadata; passes AA on white. |
| Border | `#E7E5E4` | Hairlines and control edges. |
| Litehouse green | `#00A544` | Primary capability, connection, and selected-state signal. |
| Success | `#008138` | Stable success text and icons on light surfaces. |
| Warning | `#C53C00` | Warning text and icons on light surfaces. |
| Danger | `#E40014` | Errors and destructive actions. |

The earlier orange HSL theme in `site/app/globals.css` is legacy cascade input. The later OKLCH block wins, compiling to the warm-neutral values above. Do not revive orange as the primary. Green comes from the current source utilities used across the public hero, Registry label, pairing, selection, and Cockpit connection states.

Green-filled controls use warm black text, giving at least 4.5:1 contrast. Green text links use the darker active green and an underline. Status always includes an icon or word such as “Connected,” “Warning,” or “Failed.” Never communicate state by color alone.

Dark mode rebinds the same roles to `#0C0A09`, `#1C1917`, `#292524`, `#FAFAF9`, and the current green ramp. It is an alternate operating condition, not a separate aesthetic.

## 3. Typography

Use `Inter, ui-sans-serif, system-ui, sans-serif` for display and body. Use `ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace` for machine-readable content.

| Token | Size | Typical role |
|---|---:|---|
| `--text-xs` | 12px | Badges, timestamps, helper text |
| `--text-sm` | 14px | Controls, table cells, product body |
| `--text-base` | 16px | Reading body |
| `--text-lg` | 18px | Lead copy, compact subsection title |
| `--text-xl` | 20px | Component heading |
| `--text-2xl` | 24px | Product page title |
| `--text-3xl` | 36px | Public section title |
| `--text-4xl` | 60px | Public hero display on larger screens |

Body leading is 1.5. Headings use 1.1 and `-0.025em` tracking at display sizes. Product headings are sentence case. Cap prose at 68ch. Do not use monospace as atmosphere; reserve it for values users may compare, copy, or diagnose.

## 4. Spacing

Use the source-backed four-pixel scale: 4, 8, 12, 16, 20, 24, 32, and 48px. Public sections use 64px vertical rhythm on desktop, 48px on tablet, and 32px on phone. Product views may compress internal rows to 8 or 12px while retaining 24 to 32px between distinct task groups.

Controls are at least 40px tall in dense product views and 44px for primary workflow actions. Keep a 24px minimum separation between unrelated groups. Do not apply identical padding to every surface: public modules may use 24 to 32px, while table and log rows use 8 to 16px.

## 5. Layout & Composition

Use one vocabulary at two densities:

- **Public and documentation:** centered content up to 1280px, wide section rhythm, visible grid lines, asymmetry when useful, and no more than one prominent green field per viewport.
- **Registry and Cockpit:** sticky 64px utility header, compact search and controls, scrollable semantic tables, and side or tool regions on warm surfaces.
- **Responsive:** desktop gutters 32px, tablet 24px, phone 16px. Collapse multi-column regions at 1024px and preserve a single reading column below 640px.
- **Wallboard:** use a narrow contextual rail plus room-first device rows. The rail contains only installed, authorized context such as time, measurements, rule schedules, and server status. It is not a fixed weather, presence, or media requirement.
- **Phone controls:** stack bounded physical-device surfaces with 44px targets, direct controls for common operations, and a small bottom navigation. Keep each device's facets together.
- **Device detail:** use an in-page two-column control/state layout on desktop and one stacked route on phones. Reserve modal dialogs for interruptive decisions, not routine device inspection.

Hairline divisions may construct a larger field, as on the public `main-grid`. Cards are for bounded objects such as packages, physical devices, or server summaries, not universal wrappers. Avoid nested cards. Dense logs use rows and separators instead of individual tiles.

## 6. Components

- **Buttons:** 8px radius, 44px minimum target for primary actions, 14px medium-weight labels. Primary green uses dark text. Secondary controls use the warm neutral fill. Destructive controls include explicit text and iconography.
- **Inputs and search:** 8px radius for standard fields; pill radius is reserved for global search and compact filters. Labels remain visible. Error fields use `aria-invalid`, a danger border, and linked error copy.
- **Cards and package rows:** 14px radius for standard bounded surfaces, one-pixel border, and the current small shadow only when lift clarifies grouping. Interactive cards change border and surface together and expose one clear link target.
- **Badges and status:** pill shape, 12px text, icon plus label. Use outline or quiet tint so status does not dominate the task.
- **Tables and logs:** 14px product text, 12px metadata, one-pixel row separators, tabular or monospace values where comparison matters. Use real table semantics and captions.
- **Loading:** use skeleton blocks for content regions. Buttons may use a small progress indicator with `aria-busy`; retain the action label.
- **On / Off:** use a labeled switch for fast overview control or an explicit On/Off segmented control when state comparison matters. Unknown is never rendered as Off.
- **Level:** pair a native range input with an exact visible value and native quantization metadata when useful. Coupled On/Off behavior is a separate advertised action.
- **Color:** show only advertised color models and physical ranges. A color-temperature row, hue/saturation controls, CIE xy fields, or provider-native channels remain distinct rather than collapsing into one universal color wheel.
- **Boolean and measurement:** present as read-only values with semantic role, source quality, freshness, and unknown/unavailable state. Use profile-specific measurement icons only from the fixed host mapping.
- **Thermostat:** keep mode, target model, fan, swing, and preset choices together. Ambient temperature and humidity remain separate Measurement facets in the same physical-device composition.
- **Window covering:** keep Open, Close, Stop, lift, and tilt feature-gated. Movement or idle state never claims physical completion.
- **Momentary and notification:** use explicit labeled actions. Show typed receipt confidence next to the originating action, and never convert Accepted for delivery into observed state.

Every supported interactive component defines default, hover, visible focus, active, disabled, loading, and error states. Disabled controls retain readable labels and do not rely on opacity alone. Focus uses a two-stage ring with strong foreground contrast. Use Lucide 1.24.0 outline icons from the vendored static set; icons support labels and plugins never select icon names.

## 7. Motion & Interaction

Use motion only to explain state. Fast control feedback is 150ms; broader state changes are 200ms. Preserve the current standard easing `cubic-bezier(0.4, 0, 0.2, 1)`. Hover may shift a control by at most one pixel, but active state must return it to the baseline. Do not animate layout properties.

Skeletons may pulse gently. Loading indicators may rotate. Under `prefers-reduced-motion: reduce`, stop both, remove transforms, and make state changes immediate. Connection, error, and success changes remain legible through text and symbols when all animation is removed.

## 8. Voice & Brand

Litehouse is quietly capable. Copy is concise, candid, and operationally specific.

- Say “Connected directly” rather than “Everything looks great.”
- Say “Pairing code expired. Request a new code.” rather than “Something went wrong.”
- Explain local trust, permissions, and offline behavior at the action that depends on them.
- Start with plain language, then reveal IDs, contracts, traces, and diagnostics on demand.

Use simple 1.5 to 2px outline icons from the pinned Lucide vocabulary. The Open Design package vendors unchanged Lucide Static 1.24.0 SVGs plus its ISC license for offline previews. Product React code uses the matching `lucide-react` package. Icons support labels; they do not replace essential text. Capability icons come from a closed host mapping, never plugin-supplied renderer names.

## 9. Anti-patterns

- Do not resemble generic enterprise SaaS: no interchangeable KPI-card grids, decorative analytics, nested cards, or sales-led dashboard chrome.
- Do not use green as decoration across large inactive regions. It means capability, connection, selection, or action.
- Do not restore the legacy orange primary values that lose the current `globals.css` cascade.
- Do not use gradients for text. Existing source gradients are evidence of the current site, not a reusable product primitive.
- Do not use color-only status, low-contrast focus, unlabeled icon controls, or motion without a reduced-motion fallback.
- Do not hide technical truth behind vague reassurance. Keep errors calm, specific, and recoverable.
- Do not invent one-off colors, radii, shadows, or spacing values outside `tokens.css`.
- Do not make media playback, presence, weather, NAS, router, tablet, or generic service controls part of the canonical v1 facet surface. Those appear only when a separately approved plugin portfolio supplies typed operations.
