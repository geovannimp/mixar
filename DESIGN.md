---
name: Mixar
description: Open-source DJ software — honest performance tools on warm bone and zinc neutrals with a teal live signal
colors:
  accent: "#0f766e"
  accent-hover: "#115e59"
  accent-tint: "#f0fdfa"
  teal-50: "#f0fdfa"
  teal-100: "#ccfbf1"
  teal-400: "#2dd4bf"
  teal-600: "#0d9488"
  teal-950: "#042f2e"
  deck-a: "#0284c7"
  deck-b: "#e11d48"
  stem-drums: "#009e73"
  stem-bass: "#d55e00"
  stem-other: "#cc79a7"
  stem-vocals: "#56b4e9"
  bg: "#efeee9"
  bg-elevated: "#e6e3dc"
  bg-card: "#faf9f6"
  fg: "#1c1917"
  muted: "#57534e"
  border: "#d6d2c9"
typography:
  display:
    fontFamily: "\"Space Grotesk Variable\", ui-serif, Georgia, serif"
    fontSize: "clamp(2.25rem, 5vw, 3.25rem)"
    fontWeight: 700
    lineHeight: 1.1
    letterSpacing: "-0.025em"
    fontFeature: "\"ss01\", \"ss04\", \"case\", \"tnum\", \"zero\""
  headline:
    fontFamily: "\"Space Grotesk Variable\", ui-serif, Georgia, serif"
    fontSize: "clamp(1.75rem, 4vw, 2.25rem)"
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: "-0.025em"
    fontFeature: "\"ss01\", \"ss04\", \"case\", \"tnum\", \"zero\""
  title:
    fontFamily: "\"Space Grotesk Variable\", ui-serif, Georgia, serif"
    fontSize: "1.0625rem"
    fontWeight: 600
    lineHeight: 1.4
    fontFeature: "\"ss01\", \"ss04\", \"case\", \"tnum\", \"zero\""
  body:
    fontFamily: "\"Outfit Variable\", ui-sans-serif, system-ui, sans-serif"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.625
    fontFeature: "\"ss01\", \"case\", \"tnum\", \"zero\""
  label:
    fontFamily: "\"Outfit Variable\", ui-sans-serif, system-ui, sans-serif"
    fontSize: "0.9375rem"
    fontWeight: 600
    lineHeight: 1.4
    fontFeature: "\"ss01\", \"case\", \"tnum\", \"zero\""
  mono:
    fontFamily: "\"Noto Sans Mono Variable\", ui-monospace, monospace"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.5
    fontFeature: "\"tnum\", \"zero\""
rounded:
  sm: "0"
  md: "0"
  lg: "0"
  xl: "0"
  full: "9999px"
spacing:
  section-x: "clamp(2rem, 5vw, 4rem)"
  section-y: "80px"
  card: "24px"
  button-x: "22px"
  button-y: "12px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "#ffffff"
    rounded: "{rounded.md}"
    padding: "{spacing.button-y} {spacing.button-x}"
    typography: "{typography.label}"
  button-primary-hover:
    backgroundColor: "{colors.accent-hover}"
    textColor: "#ffffff"
    rounded: "{rounded.md}"
    padding: "{spacing.button-y} {spacing.button-x}"
    typography: "{typography.label}"
  button-secondary:
    backgroundColor: "transparent"
    textColor: "{colors.fg}"
    rounded: "{rounded.md}"
    padding: "{spacing.button-y} {spacing.button-x}"
    typography: "{typography.label}"
  badge-default:
    backgroundColor: "{colors.accent-dim}"
    textColor: "{colors.accent}"
    rounded: "{rounded.full}"
    padding: "6px 12px"
    typography: "{typography.label}"
  card-feature:
    backgroundColor: "{colors.bg-card}"
    textColor: "{colors.fg}"
    rounded: "{rounded.lg}"
    padding: "{spacing.card}"
---

# Design System: Mixar

## Overview

**Creative North Star: "The Open Deck"**

Mixar looks like what it is: open, performance-ready DJ software with nothing hidden behind a paywall. The visual system is restrained neutrals — warm bone in the light theme, zinc in the dark — a single teal accent for action and live state, and deck identity colors (sky blue / rose on marketing surfaces) that echo the booth without turning the UI into a light show. Density is comfortable — readable type, generous section rhythm, panels that breathe — while interactive elements feel tactile through soft hover lift and crisp square chrome.

The marketing site (`apps/website`) is the most fully custom expression of this world: Outfit body, Space Grotesk headings, CSS token themes with light/dark via `data-theme` — dark is the default canvas (the booth is dark, and the marketing references derive from dark sites); an explicit stored choice in either direction always wins. The desktop app (`apps/gui-flutter`) uses **MixarTheme** light/dark tokens (with an intentional **ShadTheme** bridge for Shad/Material widgets) plus domain-specific performance colors (fader accents, VU segments, cue amber). Shared brand commitments: Mixar logo (inverted on dark), teal as the primary interactive signal, honest "in development" status when applicable, and deck semantics that must stay distinguishable under pressure.

**Key Characteristics:**

- Warm bone light / zinc dark surfaces with hairline borders; color earns its place
- Teal accent for primary actions and brand signal; deck sky/rose for dual-deck semantics on web
- Square chrome (buttons, inputs, cards, menus, dialogs, marketing CTAs); pills reserved for tiny status chips
- Outfit + Space Grotesk on web; tabular figures for BPM/time via OpenType features
- Soft hover lift on CTAs; scroll reveals respect `prefers-reduced-motion`
- App UI: Mixar shell widgets + Shad-bridged controls, 40px custom title bar, 12px desktop window top radius (chrome stays square inside)

## Colors

A neutral palette — warm bone light, zinc dark — with one confident teal accent and semantic deck hues — performance clarity over decoration.

### Primary

- **Live Teal** (#0f766e / teal-700 light · #2dd4bf / teal-400 dark): Primary CTAs, app brand primary, section icons, accent bullets, links-in-context, booth live chrome (Q / drop-hover). The "go" color — one clear action per viewport when possible. Teal-700 is the accessibility floor in light: teal-600 measured 3.22:1 on bone-100 as text and 3.74:1 white-on-fill, both below AA; teal-700 measures 4.71:1 and 5.47:1.
- **Teal Depth** (#115e59 / teal-800 light · #5eead4 / teal-300 dark): Hover and pressed states on primary buttons.
- **Teal Tint** (#f0fdfa–#ccfbf1 / teal-50–100 light · #042f2e / teal-950 dark): Soft solid fills (selection rows, Play while playing, selected tabs, mapping chips). Never invent this with alpha over the surface. There is no emerald / green brand ladder.

### Secondary

- **Deck A Sky** (#0284c7 / sky-600 light · #38bdf8 / sky-400 dark): Marketing feature tiles and deck-A semantic highlights — same sky family as the app Deck A fader grip.
- **Deck B Rose** (#e11d48 / rose-600 light · #fb7185 / rose-400 dark): Marketing feature tiles and deck-B semantic highlights — same rose family as the app Deck B fader grip.

### Stems

Fixed slot colors for the four NI Stems, written into the `.stem.mp4` STEM atom by `crates/codec` (`stem_atom.rs::default_ni`) and mirrored as `--stem-*` tokens on the website. They are metadata, not themed accents: keep the hex values stable so generated Stems match the NI/stemgen defaults. Stem colors are semantic identifiers (like deck-a / deck-b), never page-level accents.

- **Drums** (#009e73): NI slot 1
- **Bass** (#d55e00): NI slot 2
- **Other** (#cc79a7): NI slot 3
- **Vocals** (#56b4e9): NI slot 4

### Tertiary

- **Dev Amber** (amber-100 / amber-950 soft fills; amber-300 labels): "In development", cue audition wash, and cautionary status — solid scale steps, not alpha washes.

### Neutral

Light is warm bone paper, dark is zinc booth black. The two ladders line up
step for step: page, riser, panel.

- **Stage Black** (#1c1917 light fg / #f4f4f5 dark fg): Primary text.
- **Booth Muted** (#57534e light / #a1a1aa dark): Secondary copy, nav links at rest, footer meta.
- **Floor** (#efeee9 light / #09090b dark): Page background.
- **Riser** (#e6e3dc light / #121216 dark): Alternating section bands (`bg-bg-elevated`), inputs, secondary chips.
- **Panel** (#faf9f6 light / #18181c dark): Cards, theme toggle, popovers. Lightest surface in the light theme — never pure white, and 9 luminance points above the floor so a panel reads as a plane.
- **Hairline** (#d6d2c9 light · zinc-800 #27272a dark, solid on both): Borders on cards, header scroll state, inputs-at-rest. The website used rgba white 8% in dark, which composites to #2a2a2e over the card — 3/3/4 channel values apart, invisible, so both surfaces name the solid step.

### App-specific (Flutter / MixarTheme)

- **MixarTheme semantic tokens** (`background`, `foreground`, `mutedForeground`, `border`, `primary`, `selection`, `primaryTint`, `accentTint`, `cueTint`, …): Base app chrome — do not override with raw hex in widgets; read from `context.theme.colors`. The solid scales (zinc / bone / teal / sky / rose / amber / blue / violet / orange / pink, every step) and the five user-selectable accent palettes are generated, see below.
- **Fader Deck A sky** (grip sky-400 dark / sky-600 light; 6px lane tint sky-900 / sky-200): Vertical/horizontal fader accent for Deck A — the grip is the theme's deck identity, so it matches website `--deck-a` in that theme. The lane carries a deep/pale tint instead of a bright fill: a full-height fader is a soft bar, never a neon stripe.
- **Fader Deck B rose** (grip rose-400 dark / rose-600 light; lane tint rose-900 / rose-200): Fader accent for Deck B — matches website `--deck-b` in that theme.
- **Fader Deck C amber** (grip amber-400 / amber-600, lane amber-900 / amber-200) and **Deck D violet** (grip violet-400 / violet-600, lane violet-900 / violet-200): Distinct four-deck identities, shared across deck chrome, waveforms, mixer controls, history badges, and library load menus through `faderAccentForDeck`.
- **Cue Amber** (#FCD34D / amber-300 · ring #D97706 / amber-600): Headphone cue / PFL active state.
- **VU ladder:** teal → amber → red segments for level meters.

### Token pipeline

Every colour in this document is authored once, in `tokens/` (DTCG JSON), and generated from there:

| Authored | Generated | Read by |
| --- | --- | --- |
| `tokens/primitives.json` — ten ladders, every step | `apps/gui-flutter/lib/core/theme/mixar_palette.dart` | app widgets, `MixarAccentScale` (Teal / Blue / Violet / Orange / Pink picker) |
| `tokens/semantic.json` — light + dark roles | `apps/gui-flutter/lib/core/theme/mixar_colors.dart` | `context.theme.colors` |
| `tokens/web.json` — website aliases and web-only roles | `apps/website/src/styles/tokens.css` | `global.css`, imported at the top |

`npm run tokens:build` writes the three files; `npm run tokens:check` rebuilds and fails CI when the committed output and the tokens disagree. Edit the JSON, never the generated files — their headers say so.

What stays authored by hand: `MixarColors.hover()` / `disable()` / `withAccentScale` behaviour, the website's `@theme inline` Tailwind bridge, `.band-teal`'s variable remap, gradients, `color-mix()` values, shadows, and the `--stem-*` hexes (owned by `crates/codec` so generated Stems match NI defaults). DTCG cannot express most of those, and duplicating them would recreate the drift this pipeline removes.

### Named Rules

**The Solid Palette Rule.** Fills, icons, and borders come from named scale steps (teal-100, teal-950, sky-400, …). Do not `alphaBlend` or `withValues(alpha:)` a brand colour onto a surface to invent a wash — that drifts by base colour and leaves the palette. Alpha is reserved for true see-through chrome (dialog scrims, panels that must show the deck behind them). Do not introduce an emerald/green brand scale.

**The One Signal Rule.** Teal is the only brand accent on a given marketing viewport. Deck sky/rose appear as semantic labels, not competing primaries.

**The Honest Status Rule.** Development and roadmap states use muted opacity or amber dev badges — never fake "available" styling for unreleased platforms.

## Typography

**Display / Headline Font:** Space Grotesk Variable (with ui-serif fallback)  
**Body Font:** Outfit Variable (with system-ui fallback)  
**Mono Font:** Noto Sans Mono Variable (BPM, timecode, grid labels in app; code on web)

**Character:** Geometric and modern without feeling sterile — Space Grotesk gives headlines a booth-poster confidence; Outfit keeps body copy approachable. OpenType features (`ss01`, `case`, `tnum`, `zero` on sans/serif; tabular mono) keep numbers stable in performance contexts.

### Hierarchy

- **Display** (700, clamp 2.25–3.25rem, line-height 1.1, tracking tight): Hero `h1` on landing and developer pages.
- **Headline** (700, clamp 1.75–2.25rem): Section `h2` with optional centered Lucide icon (64px, accent-colored).
- **Title** (600, 1.0625rem): Card titles (`h3`), feature names.
- **Body** (400, 1rem / 1.0625rem for lead, line-height ~1.625): Paragraphs; lead copy max ~36–48ch in heroes.
- **Label** (600, 0.9375rem): Buttons, nav links when active, platform pills.
- **Caption** (400, 0.8125rem–0.875rem): Badges, tooltips, footer, engine status in app header.
- **Mono** (400, 0.875rem+, tabular figures): Tempo, time, cue indices, technical values — plus landing-page technical labels: numbered section eyebrows (`01 / Features`), the hero spec row, and feature-card plate numbers.

### App note

Flutter ships **Outfit** (body), **Space Grotesk** (display), and **Noto Sans Mono** via `MixarFonts` / `lib/shell/app_typography.dart`, exposed on `theme.typography.*`. Match web hierarchy intent with those roles (body, label, heading) rather than hard-coded sizes.

### Named Rules

**The Stable Numbers Rule.** Any value that changes during a set (BPM, elapsed time, bar count) uses tabular figures — `font-mono` on web, `FontFeature.tabularFigures()` in Flutter.

**The Headline Serif Rule.** Marketing `h1–h3` use Space Grotesk (`font-serif`); body UI stays Outfit (`font-sans`).

## Layout

**Marketing grid:** Centered content column `max-width: 70rem` (1120px) with `24px` horizontal padding. Sections use `80px` vertical padding (`py-20`); hero uses slightly tighter bottom spacing.

**Hero:** Two-column at `lg` — copy left, product screenshot right inside a floating landscape tablet mockup: flat dark body, very thin uniform bezel, subtle edge line, and square corners that match the rest of the page chrome. The tablet floats and tilts in 3D on desktop and scroll-tilts on mobile. Stacked on smaller screens with a generous, non-overlapping grid gap. The copy column ends with a mono spec row (License / Platforms / Price) under the CTAs so license and cross-platform answers sit above the fold. Behind both columns sits a theme-aware music constellation: a neutral starfield threaded with small teal audio glyphs, a mirrored oscilloscope trace across the floor, and a faint warm charcoal haze. Dark is near-black; light is warm off-white. The constellation moves only in long ambient loops and freezes under `prefers-reduced-motion`.

**Feature grid:** 1 → 2 (`sm`) → 4 (`lg`) bento with the first two cards double-width (`wide`), `20px` gap; cards are solid panel (`bg-card`) floating over a giant ghost outline word (`ghost-word`, SVG `<text>`, `aria-hidden`) that draws itself in on entry — the word reads in the gutters and above the first row. Cards carry a hover lift + soft shadow (the one sanctioned shadow growth) and a mono plate number top-right.

**Developer pages:** Mirror the landing's depth language — one marquee ticker after the hero, a ghost outline word (`STACK`) behind the Stack card grid with the same draw-on-entry trace, numbered section eyebrows (`01 / Stack`, `02 / Architecture`), and the teal contrast band (`band-teal`) on the "Get involved" CTA. Prose stays in a narrow `640px` centered column; architecture sections may use full content width.

**Closing developers section:** The home page's final band is a dark space window in the dark theme - page color at the edges (merging with the preceding section), a low, dark green bloom rising from the bottom center - and the plain page background in the light theme. The shared starfield and audio glyphs are light on the dark band and dark teal on the light one. A flat, opaque tinted card keeps the normal theme tokens: light-on-dark in the dark theme and dark-on-light in the light theme.

**App shell:** Minimum window 1024×768, default 1280×800. Two-deck layout: Deck A | Mixer | Deck B; four-deck layout stacks A/C at left and B/D at right (`deck_grid.dart`). Four-channel mixer order: EQ/filter A+B | gain/volumes A+B | meters | gain/volumes C+D | EQ/filter C+D. Gain knobs sit above their volume faders, without deck-letter labels. Crossfader endpoint A/B labels open deck-assignment checklists; unchecked decks use Thru. Narrow deck headers scroll rather than shrink their controls. Custom 40px header; undecorated Linux desktop windows keep a 12px top-corner clip (`kDesktopWindowRadius`) so the compositor shadow matches — maximized drops to 0.

**Responsive breakpoints (web):** Tailwind defaults — `sm` 640px, `md` 768px, `lg` 1024px. Below `md` the header keeps the logo, GitHub link, and theme toggle; the section links (Features, NI Stems, Developers) appear from `md` up.

## Elevation & Depth

Mostly flat surfaces with **soft lift on interaction**. Depth comes from tonal layering (floor → riser → panel) and hairline borders before shadows. Shadows are reserved for hero product imagery and floating tooltips — not everyday cards.

### Shadow Vocabulary

- **Screenshot float** (`0 24px 64px rgba(0,0,0,0.12)` light; dark adds `0 0 0 1px rgba(255,255,255,0.03)` plus deeper drop): Hero and marketing product shots only.
- **Popover** (`shadow-lg` on platform tooltips): Small floating labels, not page chrome.
- **Header** (no shadow): Scroll state uses bottom border `border-border`, not box-shadow.

### Named Rules

**The Soft Lift Rule.** Primary buttons may translate `-1px` on hover. Cards lift through border brightening (`hover:border-zinc-400/25`), not shadow growth.

**The Flat Card Rule.** Regular feature cards and panels stay shadowless at rest; background `bg-card` + border is enough separation.

## Shapes

Square, instrument-like geometry — hard corners on chrome, pills only for tiny status chips.

- **Square** (`0`): App buttons, inputs, selects, cards, menus, dialogs, toasts, tabs, tooltips. Marketing CTAs, feature cards, theme toggle, icon tiles, and the hero tablet mockup use the same zero-radius token scale (`apps/website` `--radius-*` in `global.css`).
- **Window top** (`12px`): Undecorated desktop window clip only (`DesktopChrome`) — not a chrome-control radius.
- **Full pill** (`9999px`): Tiny status chips / badges only (e.g. “In development”, compact metadata pills). Circular LED status dots stay full as geometry, not chrome.

Borders are 1px hairlines; hover slightly increases border contrast rather than adding fill weight. Icons: Lucide stroke icons at 16–18px in UI chrome, 64px for section hero icons.

**App note:** `MixarBorderRadius` in `apps/gui-flutter/lib/shell/mixar_style.dart` is the runtime source of truth for the desktop app — every step except `pill` is `BorderRadius.zero`. The marketing site mirrors that scale via Tailwind `--radius-*` tokens flattened to `0` (keeping `--radius-full` for status chips).

## Components

### Buttons (web)

- **Shape:** Square (`rounded-md` → 0)
- **Primary:** Teal fill, white text (dark: teal-950 text on bright teal), semibold 15px, icon gap 8px; hover teal depth + 1px lift
- **Secondary:** Transparent, hairline border, hover accent-dim wash and border brighten

### Badges

- **Default:** Teal mist fill, teal text, pill, 13px medium — the one intentional round form
- **Dev:** Amber border/fill/text for in-development status

### Cards / Feature tiles

- **Corner:** Square (0)
- **Background:** Panel white/dark card
- **Border:** Hairline; hover border only
- **Padding:** 24px
- **Icon tile:** 40×40, square, tone from deck-a / deck-b / accent

### Navigation (web header)

- **Style:** Sticky 64px, frosted header bg + backdrop blur
- **Links:** Muted 15px; hover fg; current page semibold fg
- **Theme toggle:** 36px square, card bg, border; icon swap sun/moon

### App chrome (Flutter)

- **Header:** 40px, background token, logo white-filtered, engine status caption, Mixar `MTabs`, window controls
- **Controls:** Mixar / Shad-bridged buttons, switches, sliders — square corners via `MixarStyle` / Shad `radius`; faders custom with deck accent grips
- **Select menus:** Desktop `ShadSelect` opens with a top-anchored height grow + short fade (`selectTheme.effects` in `shadThemeFromMixar`)
- **Tooltips:** `AppTooltip` — square white bubble, dark ink (`#18181B`), no border radius
- **Toasts:** `showMixarToast` (context_show) for concise error feedback — square chrome
- **Panels:** Bordered deck sections; performance controls grouped in bordered panels per deck spec

### Motion

- **Page navigation (view transitions):** The Astro `ClientRouter` swaps pages without a reload. The header (`site-header`) and the docs menu (`docs-menu`) are named layers that hold still while the page body changes; the body exits with a 220ms fade and an 8px lift and enters on a 300ms expo curve with the site's 16px reveal distance, so the two layers overlap and the page never dips through the background. Forward navigation rises into place; back navigation settles down from above (Astro's `data-astro-transition` on `<html>`). Under `prefers-reduced-motion` the swap is a 120/160ms crossfade with no translation or scale. Styles live in `apps/website/src/styles/view-transitions.css`.
- **Scroll reveal:** 500ms fade + 16px translate, staggered 100ms; off under reduced motion. Hero copy stays visible for LCP.
- **Hero tablet:** Motion-driven float/tilt on desktop; scroll-scrubbed tilt on mobile. No scanline or status chrome sits on the device.
- **Hero stage:** Star layers drift and twinkle, the audio glyphs float, and the mirrored oscilloscope trace drifts; all ambient animation stops under `prefers-reduced-motion`.
- **Closing developers section:** The shared starfield and audio glyphs drift and twinkle on the section behind the flat tinted card under the same reduced-motion guard.
- **Ghost word draw:** The features ghost outline word traces itself in via `stroke-dashoffset` (2000→0, 1.8s ease-out) when scrolled into view — timings follow the site reveal language; drawn instantly under `prefers-reduced-motion`.
- **Marquee ticker:** Full-bleed duplicated-track divider (`.marquee-track`, six copies so the `-50%` loop keeps the band covered at every offset, ~90s linear loop); static under `prefers-reduced-motion`. Content words repeat copy that also appears in real sections.
- **Band video:** The NI Stems section opens with a muted looping full-bleed video band. It never autoplayed before an explicit Play — Astro's cached back-navigation used to restore it already running, which read as the demo auto-starting — so it plays only while the stem player is actually playing and holds one frame under `prefers-reduced-motion`.
- **Teal contrast band:** `band-teal` tints whole sections to the teal primary scale (Pricing and the dev-page CTA) — teal-950 on the dark page, teal-50 on the light page — with a single surface tone: cards match the band and the alpha hairline border does the lift, so no second green stripe appears.
- **Select open (app):** Height factor 0→1 from top + fade (~160ms ease-out); not the stock scale pop

## Do's and Don'ts

### Do:

- **Do** use semantic CSS tokens (`bg`, `fg`, `accent`, `deck-a`, `deck-b`) on the website — never hardcode zinc/teal one-offs in components.
- **Do** keep one primary teal CTA per marketing section; secondary actions use ghost/secondary button variant.
- **Do** mark unreleased platforms with reduced opacity and "On roadmap" tooltip — never full accent styling.
- **Do** read colors from `context.theme` in Flutter. Shared themed colours are generated (see Token pipeline); the `stem-*` hexes in the front matter are codec metadata and stay hand-authored. Deck identity comes from `deckTone(accent)` (`core/theme/deck_accent.dart`), which returns the whole 11-step tone; pick the rung at the call site — `textColorStep(tone, brightness)` for text, badges and knob arcs (400 dark / 800 light — the 600 rungs of sky and amber fail AA on bone), `primaryHover` for the fader grip and primary hover, `tone.s900`/`tone.s200` for a lane tint. The fader's tint pair is private to `fader_slider.dart`.
- **Do** respect reduced motion for scroll reveals and hero animation.

### Don't:

- **Don't** add drop shadows to feature cards or section backgrounds — borders and tonal layers carry depth.
- **Don't** use deck sky/rose as page-level accents; they are semantic deck identifiers only (on web).
- **Don't** fabricate social proof, star counts, or "available now" platform badges.
- **Don't** invent an emerald/green brand ladder — teal is the brand signal; deck-a is sky blue.
- **Don't** hide the in-development status on the landing hero until a real release ships.
- **Don't** reintroduce soft radii on chrome (buttons, cards, menus, dialogs, toasts, marketing CTAs) — square is the system; pills stay for tiny status chips only.
