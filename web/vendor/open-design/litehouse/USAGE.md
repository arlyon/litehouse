# Litehouse Usage

Design System 2.0 package guide for Open Design agents and reviewers.

## Read Order

1. Read `DESIGN.md` for the unified public and control-plane direction.
2. Copy the complete `:root` block from `tokens.css` before authoring component CSS.
3. Use `components.manifest.json` for inventory and `components.html` for exact accessible states.
4. Inspect `preview/colors.html`, `preview/typography.html`, and `preview/spacing.html` for the foundation.
5. Inspect `preview/wallboard.html`, `preview/phone-controls.html`, and `preview/device-detail.html` for the responsive control-surface contract.
6. Inspect `preview/capability-facets.html` for all nine exact-major renderers, operational states, and deterministic dark-mode coverage.
7. Use `source/evidence.md` when auditing why a value or pattern was selected.

## Design Highlights

- Warm-neutral computed canvas and type: `#FFFFFF`, `#0C0A09`, `#79716B`, `#E7E5E4`.
- Litehouse green `#00A544` signals capability, connection, selection, and primary action.
- Inter carries both public and product surfaces; monospace is reserved for operational values.
- Public layouts breathe and expose the one-pixel grid. Registry and Cockpit use denser rows and controls.
- Every state includes text, shape, or icon cues in addition to color.
- Desktop overview uses room-first rows and an optional context rail; phone control uses a device stack; detailed control uses an in-page panel.
- All nine v1 capability facets have host-owned renderers. Unknown, unavailable, stale, pending, receipt confidence, and outcome unknown remain explicit.
- Offline previews use unchanged Lucide Static 1.24.0 assets under `assets/lucide/`; production React uses the matching Lucide package.

## Do

- Keep public and product surfaces on the same tokens, changing density rather than identity.
- Reuse buttons, fields, status badges, package rows, tables, skeletons, and notices from the fixture.
- Preserve 40 to 44px targets, visible focus, semantic structure, and reduced-motion behavior.
- Use dark mode as an environmental adaptation, not a separate neon theme.

## Avoid

- Avoid the legacy orange primary declarations near the start of Litehouse `globals.css`; later declarations win.
- Avoid generic KPI dashboards, decorative charts, nested cards, and large inactive green fields.
- Avoid raw colors or new custom properties outside the committed token contract.
- Avoid proprietary logos or copied image assets; use the wordmark and the pinned, licensed Lucide set where needed.
- Avoid promoting media, presence, weather, or generic service examples into canonical facets before the plugin portfolio and typed contracts support them.
