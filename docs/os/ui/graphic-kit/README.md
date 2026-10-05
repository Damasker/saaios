# SaaiOS UI v1 — graphic kit

Branch pack for the accepted product visual destination.
Base: `feat/som-v1`. This kit does **not** change runtime code.

## Ready-made concept boards

Copied from `docs/os/ui/references/`:

| File | Role |
|------|------|
| `references/v1-orb.jpg` | Orb states, input, plan, confirmation, result |
| `references/v1-visual-system.jpg` | Brand, color, icons, type, components |
| `references/v1-surfaces.jpg` | Сейчас / Пространство / Объект / Намерение / Система |

Authoritative prose: [`../product-visual-target-v1.md`](../product-visual-target-v1.md),
[`../../architecture/visual-language-v1.md`](../../architecture/visual-language-v1.md).

## Machine tokens

| File | Use |
|------|-----|
| `tokens/v1-tokens.json` | Colors, type, spacing, orb states, nav |
| `tokens/v1-tokens.css` | CSS custom properties for prototypes |

Shipping pixels must follow `visual-language-v1.md`. Poster hexes from the
visual-system board are listed under `poster_family_not_shipping` and must
not override calibrated tokens without Pixel evidence.

## Orb SVG (implementation-ready)

Opaque-first, calibrated teal palette:

- `orb/orb-idle.svg`
- `orb/orb-listening.svg`
- `orb/orb-analyzing.svg`
- `orb/orb-planning.svg`
- `orb/orb-confirm.svg`
- `orb/orb-executing.svg`
- `orb/orb-result.svg`
- `orb/orb-error.svg`

Voice/`listening` is a **visual destination**; Pixel voice remains blocked.

## Icons

Nav: `icons/nav-now.svg`, `nav-spaces.svg`, `nav-search.svg`, `nav-system.svg`  
Actions: `action-confirm.svg`, `action-cancel.svg`  
Objects: `object-device.svg`, `object-file.svg`

## Working mockups

Phone-oriented destination frames (generated working art, not Pixel captures):

- `mockups/surface-now.jpg`
- `mockups/surface-space.jpg`
- `mockups/surface-object.jpg`
- `mockups/surface-intent.jpg`
- `mockups/surface-system.jpg`
- `mockups/orb-states-sheet.jpg`

These illustrate hierarchy for design review. They do not authorize fake
telemetry, Android VM rows, or chat-home IA.

## v1 navigation contract

```text
Сейчас · Пространства · Поиск · Система
```

`Входящие` is not a primary tab in this destination. Attention lives on
`Сейчас`.

## Out of scope

- drm-splash / panther C changes
- inventing agents, workers-as-personas, or fake clusters
- replacing HIA / VUI sprint ownership
