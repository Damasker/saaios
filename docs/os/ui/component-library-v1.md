# SaaiOS Component Library v1 — inventory and base specifications

Status: **Experimental contract for VUI-02**

Product source: [Visual Language v1](../architecture/visual-language-v1.md)

Delivery source: [Visual System roadmap](../sprints/VISUAL-ROADMAP.md)

This document is the reviewed design boundary for the first reusable SaaiOS
components. It defines meaning and behavior. Public `.sui` v2 API docs,
stability labels, and the NOW example live in
[`sui-v2-public-api.md`](sui-v2-public-api.md) (ADR-186). `sui 1` is still
the only compiled production document. It is not a gallery of decoration
and it does not move shell business logic into the library.

## 1. Reference surface and coordinate model

- Reference device: Pixel 7, 1080×2400 physical pixels at 3× scale.
- Reference layout viewport: 360×800 logical units.
- Components consume logical units. Only a surface/backend converts them to
  physical pixels.
- Safe insets are supplied by the surface. A component never hardcodes the
  camera cutout, status layer, navigation region, or keyboard inset.
- The same computed rectangle tree drives rendering, hit testing, focus order,
  clipping, and accessibility bounds.
- The minimum primary touch target is 48×48 logical units. A visual mark may be
  smaller but its hit rectangle may not.
- Long Russian text and increased text scale are normal inputs, not edge cases.

## 2. Stability model

| Level | Meaning |
|---|---|
| Shell-private | Temporary composition owned by `saai-shell`; never an app API. |
| Experimental | Shared implementation exists; API and metrics may change during VUI-02/03. |
| Stable | Contract, gallery, accessibility, golden render, layout, hit-test, and migration tests have passed on Pixel 7. |

No component in this document is stable merely because it has been drawn.
Promotion is explicit and versioned.

### 2.1 Public subset (ADR-185)

Third-party `.sui` v2 uses `compile_v2_public()`. The name is public when
it is a proven primitive, composite, or live surface and is not privileged.
The subset is still Experimental. It is not Stable until the VUI-09
promotion checklist passes on Pixel 7.

**Public primitives:** `SemanticText`, `Icon`, `Divider`, `StatusIndicator`,
`Progress`, `Button`, `Field`, `DataRow`, `Metric`, `Disclosure`,
`SurfacePattern`.

**Public composites:** `ContextHeader`, `SystemSection`, `ObjectSummary`,
`BottomNavigation`, `IntentSummary`, `TaskSummary`, `AgentSummary`,
`SettingRow`, `EventRow`, `SpaceRow`, `WifiRow`, `BluetoothRow`.

**Public surfaces:** `now`, `inbox`, `spaces`, `me`, `object`, `intent`,
`apps`, `wifi`, `bluetooth`, `trusted`, `wifi-password`, `pin-setup`,
`consent`, `remote-pair`.

**Privileged (shell `compile_v2()` only):** `OrbHost`, `SystemStatus`,
`DecisionOverlay`, `CapabilityRow`, `TrustedClientRow`, `lock`,
`diagnostic`, `gallery`. Deferred names (`SpaceDetail`, `MemoryReview`,
`ChatThread`, `Widget`) remain outside the vocabulary.

Gallery fixtures may show privileged rows so reviewers can see them.
Those rows are not an app API.

## 3. Inventory

| Layer | Component | Initial level | First real consumer |
|---|---|---:|---|
| Foundation | color, type, spacing, radius, stroke, elevation, motion, safe inset | Experimental | all surfaces |
| Layout | `Stack`, `Row`, `Inset`, `Separator`, `ScrollRegion`, focus order | Experimental | component gallery |
| Primitive | `SemanticText`, `Icon`, `Divider`, `StatusIndicator` | Experimental | gallery, `Сейчас`, no-PIN lock |
| Primitive | `Progress`, `Button`, `Field`, `DataRow`, `Metric`, `Disclosure` | Experimental | gallery and `Сейчас` |
| Composite | `ContextHeader`, `SystemSection`, `ObjectSummary` | Experimental (VUI-03) | `Сейчас` |
| Composite | `BottomNavigation`, `OrbHost`, `SystemStatus` | Experimental (VUI-04) | shell navigation, Orb, status layer |
| Composite | `IntentSummary`, `TaskSummary`, `DecisionOverlay`, `AgentSummary` | Experimental (VUI-05) | Object View |
| Composite | `SettingRow`, `CapabilityRow` | Experimental (VUI-06) | `Я` / `Система` |
| Composite | `EventRow` | Experimental (VUI-07) | `Входящие` |
| Composite | `SpaceRow` | Experimental (VUI-07) | `Пространства` |
| Composite | `WifiRow` | Experimental (VUI-07) | `Wi-Fi сети` |
| Composite | `BluetoothRow` | Experimental (VUI-07) | `Bluetooth устройства` |
| Composite | `BluetoothRow` | Experimental (VUI-07) | `Bluetooth устройства` |
| Composite | `BluetoothRow` | Experimental (VUI-07) | `Bluetooth устройства` |
| Pattern | empty, loading, offline, blocked, failed | Experimental (VUI-07 ADR-155/156) | NOW, apps grid, Bluetooth scan/pair |
| Pattern | confirmation, permission, recovery | Deferred to VUI-07 | system surfaces |

## 4. Shared state contract

Interactive components use one state model. Product state such as `RUNNING` or
`FAILED` remains separate from control interaction state.

| Interaction state | Visual cue | Input behavior | Accessibility |
|---|---|---|---|
| Default | semantic foreground/background | accepts allowed actions | normal role/value |
| Pressed | pressed token, no layout shift | action waits for valid release | state exposed |
| Focused | 2-unit focus outline | keyboard/switch activation | focus exposed |
| Disabled | disabled tokens plus unchanged readable label | no action or haptic | disabled exposed |
| Busy | stable layout plus progress cue | duplicate action suppressed | busy exposed |

Rules:

- `Blocked`, `Attention`, and `Failed` use the universal product-state mapping;
  controls do not invent local amber or red meanings.
- Color is never the only state cue.
- Press and focus do not change component size or move neighboring content.
- Disabled content remains legible and never masquerades as unavailable data.
- Motion follows shared motion tokens and reduced-motion policy.

## 5. Layout specifications

### 5.1 `Stack` and `Row`

- Anatomy: ordered children, main axis, cross-axis alignment, gap, optional
  wrapping policy.
- Default gap: semantic spacing token, never a raw screen-specific value.
- Overflow: explicit clip, scroll, wrap, or measured overflow result; silent
  clipping is invalid.
- Empty children do not create phantom gaps.
- RTL is not promised in v1, but ordering must not be encoded in pixel math.

### 5.2 `Inset`

- Applies semantic edge spacing and surface-provided safe insets.
- Content bounds, hit bounds, and accessibility bounds receive the same
  transform.
- Keyboard and bottom-navigation insets compose rather than overwrite one
  another.

### 5.3 `Separator`

- One logical unit by default; uses `color.border.default` or
  `color.grid.subtle` by semantic purpose.
- Does not receive focus or accessibility exposure unless it represents a
  named boundary.

### 5.4 `ScrollRegion`

- Owns viewport, content extent, offset, clipping, overscroll policy, and
  preserved top/bottom chrome.
- Bottom navigation and the status layer are outside the scrolling viewport.
- Drag recognition does not activate a child action after the movement
  threshold is crossed.
- Required fixtures: empty, one viewport, long content, long Russian text,
  increased text scale, and content mutation while scrolled.

## 6. Primitive specification sheets

### 6.1 `SemanticText`

| Property | Contract |
|---|---|
| Anatomy | text run, semantic type role, semantic color role, optional maximum lines |
| Variants | display, title, section, body, label, caption, mono body |
| Layout | measured from real font metrics; line height belongs to the type role |
| Overflow | wrap by default for prose; ellipsis only when a full-value route exists |
| Scale | follows system text scale without shrinking touch targets |
| Accessibility | preserves the full untruncated string |

Sans and mono are semantic families. Font filenames are backend assets and may
change without changing component callers. No-PIN lock idle (ADR-153) paints
device state as Caption of the live fuel-gauge; missing reading omits.
AOD / deep-idle (ADR-154) paints only the Display-role clock; wake is not
unlock.

### 6.2 `Icon`

| Property | Contract |
|---|---|
| Anatomy | named semantic glyph inside a square visual box |
| Sizes | 16, 20, 24 logical units; 24 is the normal control size |
| Stroke | shared line weight, round joins/caps where the icon source permits |
| Color | inherits a semantic role; never embeds RGB values |
| Accessibility | decorative when paired with text; otherwise requires a name |

The initial icon set must be one coherent, reproducibly generated source. Emoji,
font-dependent symbols, and unrelated icon packs are not valid fallbacks.

### 6.3 `Divider`

- Uses `Separator` geometry and the border token.
- Optional inset aligns with the text edge of neighboring rows.
- Never substitutes for section spacing when no boundary is needed.

### 6.4 `StatusIndicator`

```text
┌ mark ─ label ─ optional reason ┐
└ universal state supplies all three ┘
```

- Inputs: `UniversalState`, localized label, optional concise reason.
- Compact variant: mark plus label; normal variant may add reason.
- The mark comes from `StatusMark`; the color comes from the state style.
- First lock consumer: no-PIN lock idle (ADR-148) paints compact
  `Offline` / `Attention` from store and ATTN booleans. The view type
  cannot hold a title or body; `visible_reason()` stays empty.
- Lock device state (ADR-153) is Caption `SemanticText` of the live
  fuel-gauge (`Заряд N%` / `Зарядка N%`), not a `StatusIndicator`
  color and not a determinate `Progress` track. Missing reading omits.
- `RUNNING` may use the shared quiet activity motion; reduced motion shows the
  static activity mark.
- A reason wraps below the label rather than shrinking it.

### 6.5 `Progress`

- Variants: determinate and indeterminate; never display invented percentage.
- Visual track: at least 4 logical units high; touch is not implied.
- Determinate value is clamped to 0–100 and exposed numerically.
- Indeterminate progress uses restrained motion and a textual activity state.
- Completion changes to `COMPLETE` only after the owning operation verifies it.

### 6.6 `Button`

```text
48 minimum hit height
┌──────────────────────────┐
│ optional icon  label     │  40 visual height, 16 horizontal inset
└──────────────────────────┘
```

- Variants: primary, secondary, quiet, destructive.
- Destructive styling is allowed only for a genuinely destructive action and
  does not remove the confirmation/policy requirement.
- One line is preferred. Long labels wrap to two lines and increase visual
  height while retaining insets; they never reduce type size.
- Pressed feedback begins immediately; haptic policy is requested from the
  shell rather than driving hardware directly.
- Inputs: label, action, variant, enabled, busy, optional semantic icon.
- Emits exactly one action after a valid press/release sequence.

### 6.7 `Field`

- Anatomy: persistent label, value/editor, optional help/error text, optional
  leading/trailing semantic icon.
- Minimum hit height: 48 logical units; text cursor and selection are renderer
  responsibilities.
- Empty value is distinct from placeholder. Placeholder is never the only
  accessible label.
- Error uses `FAILED` cue plus text. Offline/permission limitations name their
  cause and do not pose as validation errors.
- Password/PIN variants never expose their value through logs or accessibility
  unless the user explicitly reveals it.
- First real consumer of `FieldKind::Password`: Wi-Fi password screen
  (ADR-132), PIN setup (ADR-133), and lock unlock (ADR-149). Preview
  and accessibility use `accessible_value()`. No reveal control. Lock
  occupancy is dummy length, never `pin_code`.
- First real consumer of `FieldKind::Text`: intent composer (ADR-135).
  Placeholder `Наберите текст…` is distinct from empty. Offline is
  `help` (`Нет связи`), not `error`. ADR-161 parks the Field on the
  docked QWERTY under `ContextHeader` «Намерение»; Wi-Fi password
  reuses that avoidance as «Пароль». ADR-162 puts the Field in the
  `intent_view` tree as focus stop 0. ADR-163 commits a key only when
  down and up hit the same action; pressed still follows the finger.
  ADR-164 docks the PIN setup dialer the same way; lock unlock is
  unchanged.
- ADR-169: opening intent / Wi-Fi password / PIN setup starts
  `MotionToken::Context`. The Field paints a 2-unit `Focus` outline
  while that clock needs a frame, unless reduced motion. Lock PIN
  stays without it. Size does not change.
- Keyboard keys (ADR-151): pressed fill is `ColorRole::Pressed` while
  the finger is down; one `KeyTick` haptic on down if
  `haptics_enabled` (ADR-171). Tabs and Orb do not tick. ADR-167 keeps
  `Pressed` for `MotionToken::MicroFeedback` after release unless
  reduced motion (ADR-174 drops that clock on the same tap as
  «Меньше движения»). Release still emits the key. Size does not change.

### 6.8 `DataRow`

```text
┌ optional icon ─ primary text ───── value/disclosure ┐
│                 secondary text/reason               │
└─────────────────────────────────────────────────────┘
```

- Minimum row hit height: 48; normal two-line row: 64 logical units.
- Variants: static data, navigation, toggle/action host, and status row.
- Static data is not styled as clickable. Navigation has a disclosure cue and
  action. A value remains visually distinct from secondary explanation.
- Long content reflows vertically. Technical identifiers may use mono body and
  expose a copy action when useful.
- First diagnostic consumer of a bare Static `DataRow`: HIA-20
  «Диагностика» (ADR-136, header ADR-152). Flattened to `ActionCardView`.
  Overflowing «Назад» docks on-screen (`stacked_control_rect`, ADR-159).
  Facts that do not fit above it scroll (`scrolled_row_rect`, ADR-160).
  Wi-Fi / Bluetooth / trusted trailing controls dock as a cluster
  (`stacked_trailing_rect`, ADR-165). The 7-tap gesture is unchanged.

### 6.9 `Metric`

- Anatomy: label, value, optional unit, optional verified timestamp/state.
- Used only when the changing value helps a decision; not for decorative
  dashboards.
- Units never rely on position alone. Unknown and unavailable are text states,
  not zero.
- Numeric alignment may use tabular glyphs when the selected font supports it.

### 6.10 `Disclosure`

- Explicit collapsed/expanded state with a named target.
- Hit region is at least 48×48 even when the chevron is 16–20 units.
- Expansion updates focus order and accessibility state atomically with
  layout. Reduced motion removes the transition, not the state cue.

## 7. Composite specification sheets

Composites compose primitives (section 6); they never draw their own text or
own a rendering path a primitive does not already provide. Scoped per section
3's inventory table: `ContextHeader`, `SystemSection`, and `ObjectSummary` are
VUI-03's three; `BottomNavigation`, `OrbHost`, and `SystemStatus` are VUI-04;
`IntentSummary` and `TaskSummary` land in VUI-05. `DecisionOverlay` lands
in VUI-05 on Object View confirmation. `AgentSummary` lands in VUI-05
only from a real `saaios.action` (otherwise unassigned/unavailable).
`SettingRow`/`CapabilityRow` land in VUI-06. `EventRow` lands in VUI-07
on Inbox. `SpaceRow` lands in VUI-07 on the Пространства tab list.
`WifiRow` lands in VUI-07 on «Wi-Fi сети». `BluetoothRow` lands in
VUI-07 on «Bluetooth устройства». `TrustedClientRow` lands in
VUI-07 on «Доверенные клиенты». `FieldKind::Password` lands in
VUI-07 on the Wi-Fi password keyboard and on PIN setup.
`FieldKind::Text` lands in VUI-07 on intent input.

### 7.1 `ContextHeader`

- Anatomy: active-context label (the selected Space's display name, including
  a non-default lifecycle suffix such as an archived marker), optional
  current-section title, optional trailing `StatusIndicator` compact mark for
  a non-default lifecycle.
- Built entirely from `SemanticText` (title role for the Space name, label
  role for the section title) plus the optional `StatusIndicator`.
- Layout: a single left-aligned header row; a long Space name wraps to a
  second line rather than truncating silently (section 1's "long Russian
  text ... normal input" rule applies to this header as much as any body
  text).
- First real consumer: `Сейчас` (ADR-112), the `Приложения` grid
  (ADR-138), Inbox (ADR-139), Пространства (ADR-140), Система
  (ADR-141), app-consent (ADR-142), PIN setup (ADR-143), remote
  pairing (ADR-144), Bluetooth (ADR-145), Wi-Fi (ADR-146), and trusted
  clients (ADR-147).
- Accessibility: name is the Space name plus section title; a non-default
  lifecycle is exposed through the nested `StatusIndicator`'s own state, not
  a second accessible string glued onto the header's name.

### 7.2 `SystemSection`

- Anatomy: section title (`SemanticText`, section role), a `Divider`
  immediately below it, and zero or more child rows whose type it does not
  own -- any `DataRow`/`StatusIndicator`/`Metric`/`TaskSummary`/`IntentSummary`
  a caller composes into it.
- Empty state: a section with no children renders only its title (or is
  omitted entirely by the caller); `SystemSection` never invents a placeholder
  row to fill space. Matches `human-interface-architecture-v2.md` section 13's
  own worked example directly -- "Сегодня" / "Продолжается" / "Требует
  внимания" are three `SystemSection`s, each with a different, possibly zero,
  child-row count.
- First real consumer: the three labelled groups on `Сейчас`.
- Accessibility: title is exposed as a heading; children keep their own
  individual accessibility contracts -- `SystemSection` never flattens them
  into one combined string.

### 7.3 `ObjectSummary`

- Anatomy: primary label (object title), one secondary meta line (type and a
  distinguishing detail -- version, count, or similar), optional trailing
  value/status.
- Distinct from `DataRow`: a `DataRow` is a generic list item whose meaning a
  caller assembles by position; `ObjectSummary` has one fixed semantic
  meaning -- "this is the object I am currently working with" (section 14's
  OBJECT) -- so its meta line always reads as identity information, never an
  arbitrary second string a caller could repurpose.
- First real consumer: NOW (`now_object_summary`) and Object View
  (ADR-137). Meta is `{entity_type} · версия {revision}`. Object
  View trailing is the live workflow `StatusIndicator`, not a second
  copy of the title. A tap on the NOW summary opens Object View.
- Accessibility: name is the object title; value is the meta line; a trailing
  status uses the universal state mapping when present, never an invented
  local color.

### 7.4 `BottomNavigation`

- Anatomy: an ordered list of `NavigationItem`s, each with a label, an
  optional icon, and independent `selected`/`pressed`/`disabled`/
  `attention`/`badge` state -- section 4's shared interaction-state
  contract, applied to a navigation destination instead of a control.
- `badge` is a real count (e.g. unread items); never a decorative dot with
  no number behind it -- this document's own rule against invented data
  applies to composites as much as primitives.
- Shared, not per-page: one navigation strip, drawn identically regardless
  of what a specific destination shows above it.
- First real consumer: replaces the four-tab strip every root page already
  draws.
- Accessibility: each item exposes its own name/value (the badge count,
  when present)/disabled state independently; `BottomNavigation` itself
  does not flatten them into one combined string, the same never-flatten
  rule every other composite in this document follows.
- ADR-168: `pressed` holds `ColorRole::Pressed` for `MotionToken::
  Selection` after release unless reduced motion. Size does not change.
- ADR-196: `.sui` v2 lists destinations as nested `tab <id> { loc = … }`
  under `BottomNavigation`. Empty navigation invents no v1 hits.
  `NavigationItem` stays a Rust type, not a v2 component name.
- ADR-197: tab labels paint `TextRole::Caption` for selected and idle.
  Selected stays semibold. Size does not encode selection.

### 7.5 `OrbHost`

- Anatomy: a single `UniversalState` (quiet/active/progress/attention/
  offline map onto `Idle`/`Active`/`Running`/`Attention`/`Offline` --
  section 4's own rule against inventing a parallel state vocabulary)
  plus a `reduced_motion` flag.
- `reduced_motion` is a rendering modifier, not a sixth state: matches
  `Progress`'s "restrained motion" and `StatusIndicator`'s "reduced
  motion shows the static activity mark" -- neither of those primitives
  invents a separate state for reduced motion either.
- "Understandable without animation": the state's own non-color mark and
  `label_key` are always present regardless of whether a renderer is
  currently animating anything.
- First real consumer: the existing Orb dot/menu.
- ADR-170: while `motion()` is `ActivityPulse`, the inset hairline
  follows a looping `MotionClock` (on 240 ms / off 240 ms, Context
  duration reused). Reduced motion keeps the `StatusMark` only.
  ADR-174 drops the looping clock when «Меньше движения» turns on.
- ADR-172: `FramePace` is a diagnostic ring, not a visible component.
  The shell writes the last commit line to `/run/saaios/shell-frame.last`.
  ADR-173 adds `p95_scroll` / `p95_ok` for scroll samples against 50 ms.
  The live shell does not abort when over the limit.
  ADR-175 tags each sample with `FrameSurface` and dumps the ring to
  `/run/saaios/shell-frame.trace`.
  ADR-176 remembers the last non-scroll `input_to_commit` as `input_ok`
  against 50 ms so first visible Pressed is the down commit.
  ADR-177 counts commits as `seq` and `idle_ok` so a quiet Сейчас is
  a still main surface.
  ADR-178 names the attached buffer `backend=dmabuf` or `backend=shm`
  without changing either path. ADR-179 closes VUI-08 haptics on that
  same KeyPress map: rate-limited, silent for tabs and Orb.
  ADR-180 lists the `.sui` v2 component/surface vocabulary; it does not
  compile `sui 2`. ADR-181 parses `sui 2` component composition through
  `compile_v2()`; `compile()` still builds only `root.sui`. ADR-182
  adds `text`/`color`/`spacing`/`inset`/`scroll`/`loc`/`focus`/`a11y`
  on those blocks from live `saai-ui-core` names. ADR-183 keeps
  `root.sui` compiled through `compile()` as the v1 rollback artifact.
  ADR-184 emits that compiled `ScreenSpec` as one `layout_v1_root()`
  tree so tabs share hit-test; `compile_v2()` still does not build
  production chrome. ADR-185 names the public third-party subset and
  gates privileged names behind `compile_v2_public()`. ADR-186
  publishes the public NOW example, stability labels, and the
  migration page. ADR-187 removes leftover NOW cards from `root.sui`
  and the diagnostic panel-color literals from production `main.rs`.
  ADR-188 names leftover `draw_text` sizes through `role_px` and an
  explicit leftover list. ADR-197 maps ActionCard and tab labels onto
  `Label`/`Caption`, not Title. ADR-198 maps status time/battery and
  key labels the same way; badge and app-tile leftovers stay.
  ADR-189 is the verification ledger, not Visual v1 sign-off.
  ADR-190 proves 150% text on panther HEAD without a Система tap.
  ADR-191 proves live radio-off: status `Нет сети` while `wlan0` is
  down; Сейчас keeps live object chrome because entityd is still up.
  ADR-192 proves an unlocked `saai-shell` restart on HEAD without a
  lock surface. ADR-193 records known limitations and the Visual v2
  backlog; it is not Visual v1 sign-off. ADR-194 emits `layout_v2()`
  so public NOW tab hits match v1; production still uses
  `layout_v1_root()`. ADR-195 converts logical `SafeInsets` through
  `EdgeInsets::from_safe`; the top inset stays the status layer.
  ADR-196 names nested `tab` ids on `BottomNavigation` so `layout_v2`
  does not borrow `compile_v1_rollback()`. ADR-199 names nested
  `row` ids so `layout_v2` matches the live NOW footer. ADR-200
  docks `ObjectSummary` as the NOW object hit (`open_object`).
  ADR-201 keeps badge, gallery kicker/swatch, and app-tile labels
  named below Caption.

### 7.6 `TaskSummary`

- Anatomy: task title, a `StatusIndicator` using the universal state
  mapping, optional reason (the same status text Object View already
  shows), optional related caption (originating Intent title).
- Distinct from `ObjectSummary`: identity vs current work. Distinct from
  a bare `StatusIndicator` row: the related Intent is a separate caption,
  never merged into the status label.
- Built entirely from `SemanticText` + `StatusIndicator`. No local color,
  no worker count, no invented progress.
- Long Russian titles wrap; they do not truncate silently.
- Empty related caption is omitted, never a placeholder «без намерения».
- First real consumer: `Сейчас` «Продолжается» and a derived-ready
  «Далее» Task row.
- Accessibility: name is the task title; value is the nested
  `StatusIndicator`'s `label_key`; related caption is a separate string,
  not flattened into the name.

### 7.7 `IntentSummary`

- Anatomy: intent title plus at most one nested `TaskSummary` (the
  primary related Task from WORK-08). No Task → truthful caption
  «Нет задачи».
- Does not own a worker list. Board «Воркеры» is 0–1 disposable
  execution already visible as `TaskSummary` `Running`; this composite
  must not invent `Воркеры (3)` or an Agent personality.
- Ready-set size is not a badge here — that would turn `Сейчас` into a
  scheduler dashboard (HIA / WORK-08).
- Built from `SemanticText` + optional nested `TaskSummary`.
- First real consumer: `intent_summary_from_entity` in `saai-shell`
  (WORK-08 related Task). Painted NOW Intent card waits until Object
  View shares this type; this slice does not add a second identity
  card next to `ObjectSummary`.
- Accessibility: name is the intent title; nested task keeps its own
  contract; missing task is a separate caption.

### 7.8 `DecisionOverlay`

- Anatomy: actor, intended action, affected object, scope, optional
  consequence, and two reversible choices (`Подтвердить` / `Отклонить`).
- Missing facts are omitted. The overlay does not invent whether the
  action itself can be undone; reversibility is the pair of choices.
- Distinct from `TaskSummary`: this is the confirmation surface, not a
  list row. Distinct from app-consent (ADR-142): that names requested
  capabilities through `ContextHeader` + Static `DataRow`, this names a
  workflow/OAM decision.
- Built from `SemanticText` plus two `Button`s. No local color, no
  invented agent actor.
- First real consumer: Object View while a Task/Action is
  `waiting_confirmation` (ADR-157). Facts paint as Body; identity stays
  `ObjectSummary`. Buttons are the overlay's `accept` / `decline`.
- Accessibility: name is the object; value is the action id when
  present; role is dialog. Each button keeps its own contract.

### 7.9 `AgentSummary`

- Anatomy: one disposable execution (`saaios.action`) or a truthful
  unassigned/unavailable caption. No personality name, no worker count.
- Distinct from `TaskSummary`: this is the execution slot, not the Task.
  Distinct from `IntentSummary`: that does not own a worker list.
- Assigned only from a real Action entity. Missing Action → «Нет
  исполнения». Source that cannot be read → «Исполнение недоступно».
- Built from `SemanticText` plus optional `StatusIndicator`.
- First real consumer: Object View on Intent/Task/Action/Result.
  Notifications and unknown types omit the composite entirely.
- Accessibility: unassigned/unavailable name is the caption; assigned
  name is the Action title and value is the nested state's `label_key`.

### 7.10 `SettingRow`

- Anatomy: a `DataRow` with a setting label and the current value.
  Variants: readout (static), cycle (toggle), open (navigation), silent
  (looks static, still has a dispatch key for HIA-20).
- Not a gauge. Continuous `Metric`/`Progress` is only for values that
  inform a decision; brightness/volume stay cycle rows.
- Missing hardware, denied permission, or unverified effect is the
  real value string or an omitted row — never a painted healthy cluster.
- Distinct from `CapabilityRow`: this is a device setting, not an app
  grant list.
- First real consumer: `me_system_sections` (ADR-126). Still flattened
  to `ActionCardView` for `draw_context_row_list` (ADR-141).
- Accessibility: delegated to the nested `DataRow`.

### 7.11 `CapabilityRow`

- Anatomy: app name, lifecycle/state value, granted capabilities as
  secondary text. Static. No revoke action (no protocol).
- Empty grant set is «без разрешений», not invented scopes.
- Omit the parent `Приложения` section when there are no apps.
- First real consumer: `me_system_sections` installed-app rows.
- Accessibility: delegated to the nested `DataRow`.

### 7.12 `EventRow`

- Anatomy: a `DataRow` for one Inbox item. Live kinds: decision
  (navigation, value «Ждёт подтверждения») and notice (navigation,
  value = notification body). Absences: empty (static «Нет новых задач
  и уведомлений») and offline (static «Нет связи»).
- No invented timestamp, actor, object, or consequence. Those facts
  stay on `DecisionOverlay` in Object View.
- Distinct from `TaskSummary` (work identity) and `SettingRow` (device
  control). Distinct from NOW «Требует внимания», which stays a
  `StatusIndicator` in a `SystemSection`.
- First real consumer: `inbox_event_rows` (ADR-127). Flattened to
  `ActionCardView`. Header is `ContextHeader` (ADR-139). Inbox tab
  stays (`inbox` / `RootPage::Inbox`).
- Accessibility: delegated to the nested `DataRow`.

### 7.13 `SpaceRow`

- Anatomy: a `DataRow` for one live Space on the Пространства tab.
  Navigation with `select_space:{id}`. `selected` marks the current
  context. Status is object count plus at most one of lifecycle or
  first relation target (ADR-086).
- Empty: static «Нет пространств». Offline: static «Нет связи».
- Not a people list, not an app drawer, not Space detail. Members
  stay on Object View / NOW until that surface exists.
- First real consumer: `space_list_rows` (ADR-128). Flattened to
  `ActionCardView`. Header is `ContextHeader` (ADR-140). Tab id stays
  `spaces`. Space detail still later.
- Accessibility: delegated to the nested `DataRow`.

### 7.14 `WifiRow`

- Anatomy: a `DataRow` for one scanned SSID on «Wi-Fi сети».
  Navigation. Status is `защищена` or `открыта` plus the real scan
  `signal_dbm`. `connected` marks the associated BSS.
- Empty: static «Нет сетей». Missing adapter is named on Система
  («Нет адаптера») and does not open this list.
- Not a password field, not a Bluetooth list, not a Space binding.
- First real consumer: `wifi_list_rows` (ADR-129). Flattened to
  `ActionCardView`. «Обновить» / «Назад» stay trailing control cards.
- Accessibility: delegated to the nested `DataRow`.

### 7.15 `BluetoothRow`

- Anatomy: a `DataRow` for one `bt-scan` device on «Bluetooth
  устройства». Navigation. Value is `CLASSIC`/`BLE` when present.
  `paired` is a `SAVED` name, not live connection.
- Empty after a finished scan: `SurfacePattern` empty «Нет устройств»
  as a static card. While a scan has not finished: `SurfacePattern`
  loading «Сканирование…» as a static card, not a blank. A `PAIR-ERROR`
  log line is `SurfacePattern` failed «Ошибка сопряжения: …» in slot 0,
  with the Failed mark and without Сопрячь. «Искать» is the recovery
  control. Missing adapter is named on Система and does not open
  this list.
- Not RSSI, not a trusted-client row, not a Space binding.
- First real consumer: `bluetooth_list_rows` (ADR-130/155/156). Flattened to
  `ActionCardView`. Scan / Refresh / Back stay trailing control cards.
- Accessibility: delegated to the nested `DataRow`.

### 7.16 `TrustedClientRow`

- Anatomy: a `DataRow` for one `authorized_keys` line on «Доверенные
  клиенты». Navigation. Primary is the comment-field name. Value is
  the ADR-083 SHA256 prefix (24 characters plus ellipsis), not the
  raw key. Action is `revoke_trusted_client`.
- Empty file: static «Нет клиентов». Missing file is the same empty
  list. Not a live-session flag, not a pairing frame, not a Space
  binding.
- First real consumer: `trusted_client_list_rows` (ADR-131). Flattened
  to `ActionCardView`. «Назад» stays a trailing control card.
- Accessibility: delegated to the nested `DataRow`.

### SurfacePattern (empty / loading / offline / blocked / failed)

- Anatomy: `UniversalState` plus a caller-supplied message. Empty is
  Idle and paints no mark. Loading is Waiting plus the Waiting mark.
  Offline is Offline plus the Offline mark. Blocked and Failed use
  those states' marks. Message is Body.
- First consumers: NOW empty/offline (ADR-114 copy), apps-grid
  empty/offline (ADR-138 copy), Bluetooth scan loading / empty
  (ADR-155), Bluetooth `PAIR-ERROR` failed (ADR-156). Confirmation is
  `DecisionOverlay` on Object View (ADR-157). Permission is Object View
  `SurfacePattern::blocked` (ADR-158). Recovery is the Failed mark plus
  Bluetooth «Искать».
- Accessibility: `Status`; busy only while Waiting.

## 8. Required gallery matrix

The first device gallery uses labelled fixture data and contains no fake
runtime telemetry. VUI-02 covers the ten primitives (default state).
VUI-05 adds a second page with composites and every `UniversalState`.
VUI-07 adds `EventRow`, `SpaceRow`, `WifiRow`, `BluetoothRow`, and
`TrustedClientRow` fixtures; that page does not grow a new layout
slot in this slice.
Tap switches pages.

1. default, pressed, focused, disabled, and busy interaction states;
2. compact and normal variants;
3. ordinary and long Russian labels;
4. 100%, 125%, and 150% text scale;
5. light content, dense content, offline, blocked, and failed examples;
6. the 360×800 reference viewport and a narrower logical viewport.

The gallery must expose layout bounds and hit bounds in an optional developer
overlay. Golden renders verify visual output; structural tests verify geometry,
hit targets, overflow, focus order, and state transitions independently.
`DecisionOverlay` and `TrustedClientRow` fixtures are privileged (ADR-185);
they stay on the gallery page and are not part of `compile_v2_public()`.

## 9. API ownership

- `saai-ui-core`: semantic tokens, logical geometry, layout result, component
  contracts/state, hit testing, and backend-independent accessibility data.
- render backend: font loading, glyph rasterization, icon tessellation, pixels,
  clipping, and logical-to-physical conversion.
- `saai-shell`: real system data, navigation, capabilities, privileged actions,
  and composition of system-only surfaces.
- `.sui` compiler: declarative syntax that produces the same core types after
  the vocabulary is proven; it does not implement another layout engine.

## 10. Promotion checklist

A component can move from Experimental to Stable only when:

- its anatomy, variants, measurements, responsive behavior, and exclusions are
  documented here;
- Rust and `.sui` call sites share one semantic contract;
- rendering and hit testing consume one layout result;
- accessibility role, name, value, state, and focus order are tested;
- required gallery fixtures and golden renders exist;
- Pixel 7 touch, text scale, scroll, restart, and cold-boot review pass;
- migration and rollback are documented without changing unrelated business
  logic.
