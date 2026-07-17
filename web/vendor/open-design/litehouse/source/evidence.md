# Litehouse Source Evidence

## Source Scope

This bundled package extracts the current visual vocabulary from the Litehouse repository's `site/` surfaces, with strategic direction from `PRODUCT.md`. It does not copy proprietary assets. Values were reviewed on 2026-07-13.

## Cascade Resolution

`site/app/globals.css` contains an earlier HSL `:root` and `.dark` theme followed later by OKLCH declarations with the same custom-property names. The later declarations win in ordinary cascade order. The compiled development stylesheet confirms their current hex equivalents:

- `site/app/globals.css:125-153` supplies the winning light theme.
- `site/app/globals.css:158-186` supplies the winning dark theme.
The package normalizes those canonical OKLCH declarations to portable sRGB hex values in `tokens.css`; generated `.next` output is deliberately excluded from the evidence contract.

The early orange `--primary` and `--ring` at `site/app/globals.css:20-32` do not define the current theme and were not promoted. Litehouse green is a separate, repeated source utility rather than the final semantic `--primary` variable.

## Identity Evidence

- `site/app/layout.tsx:10-12,27` loads Inter and applies it to the document.
- `site/app/page.tsx:70-162` establishes the public one-pixel grid, broad section spacing, green field accents, rounded actions, and monospace supporting copy.
- `site/app/page.tsx:78-162` uses the green utility ramp for the hero field, Registry link, Cockpit action, pattern, and supporting copy.
- `site/app/registry/layout.tsx:28-55` uses green for the Registry identity while preserving semantic theme tokens for search and focus.
- `site/components/ui/button.tsx:7-31`, `site/components/ui/card.tsx:8-78`, and `site/app/globals.css:109-122` establish the current spacing, type, radius, transition, and small-shadow vocabulary.

## Component Evidence

- `site/components/ui/button.tsx:7-31` provides rounded medium controls, 40 to 44px sizes, hover, focus-visible, and disabled states.
- `site/components/ui/input.tsx:10-12` provides the matching field height, radius, border, focus, placeholder, and disabled treatment.
- `site/components/ui/card.tsx:8-78` provides bordered white surfaces, 14px computed radius, 24px padding, and small elevation.
- `site/components/ui/badge.tsx:7-19` provides pill badges, 12px labels, visible focus, and invalid/destructive states.
- `site/app/registry/layout.tsx:28-55` provides the sticky 64px header, Litehouse green label, pill search, and loading skeleton.
- `site/components/plugin-card.tsx:20-47` provides the source-backed package-row composition and coordinated hover changes.
- `site/components/ui/table.tsx:7-104` provides scroll containment, semantic table structure, 48px headers, row separators, hover, and selected states.
- `site/components/cockpit-page.tsx:122-220` provides monospace logs and explicit INFO, WARN, and ERROR state labels.
- `site/components/ui/skeleton.tsx:5-13` provides the rounded muted loading block.
- `site/components/flow-editor.css:1-15` confirms selected flow nodes use a border cue rather than shadow alone.

## Product Direction

`PRODUCT.md` defines the product register, “quietly capable” personality, unified public and control-plane system, explicit rejection of generic enterprise SaaS, progressive disclosure of technical truth, calm failure behavior, local trust, and WCAG 2.2 AA. Those constraints govern how the extracted vocabulary is documented and demonstrated.

## Normalization Decisions

- Green `#00A544` is the shared action/capability accent. Its dark label `#0C0A09` has a WCAG contrast ratio above 6:1.
- Success and warning use darker source ramp values on light surfaces so status text reaches AA; every status also has a word and icon.
- The current white canvas is preserved even though many new systems tint white. This package records the winning source, not a redesign.
- Existing gradient text was not made reusable. It conflicts with the confirmed anti-reference and is not needed to express the current operational vocabulary.
- The package uses no logo, bundled font, screenshot, or other proprietary asset.

## Control-Surface Direction Evidence

The responsive control setup was reviewed against the existing Litehouse component fixture and four user-supplied format studies on 2026-07-15:

- `litehouse-format-wallboard-overview.png` established the room-first desktop overview, optional context rail, stable device rows, and bottom quick-action dock.
- `litehouse-format-phone-control-stack.png` established the phone device stack, direct capability controls, state vocabulary, and bottom navigation.
- `litehouse-format-inline-detail-sheet.png` established the control/state/relationship information split. The package deliberately converts its modal presentation into an in-page desktop detail and stacked phone route.
- `litehouse-format-media-services-console.png` informed future portfolio-driven media and service layouts but is not part of the canonical nine-facet v1 surface.

The reference images are not copied into the package. The previews use only the Litehouse token vocabulary, typed capability decisions, and pinned open-source Lucide icons.

## Generated Contract

`source/token-contract.report.json` maps all 56 TOKEN_SCHEMA bindings to the canonical declaration in `tokens.css`. `source/tokens.source.json`, `design-tokens.json`, `tailwind-v4.css`, and `components.manifest.json` are derived artifacts and must not be edited independently of their canonical inputs.
