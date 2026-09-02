---
name: Mixar
description: Open-source DJ software — honest performance tools on neutral zinc with a teal live signal
colors:
  accent: "#0d9488"
  accent-hover: "#0f766e"
  accent-dim: "rgba(13, 148, 136, 0.1)"
  deck-a: "#16a34a"
  deck-b: "#0284c7"
  bg: "#fafafa"
  bg-elevated: "#f4f4f5"
  bg-card: "#ffffff"
  fg: "#18181b"
  muted: "#71717a"
  border: "rgba(0, 0, 0, 0.08)"
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
  sm: "4px"
  md: "10px"
  lg: "12px"
  xl: "20px"
  full: "9999px"
spacing:
  section-x: "24px"
  section-y: "80px"
  card: "24px"
  button-x: "22px"
  button-y: "12px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "#ffffff"
    rounded: "{rounded.full}"
    padding: "{spacing.button-y} {spacing.button-x}"
    typography: "{typography.label}"
  button-primary-hover:
    backgroundColor: "{colors.accent-hover}"
    textColor: "#ffffff"
    rounded: "{rounded.full}"
    padding: "{spacing.button-y} {spacing.button-x}"
    typography: "{typography.label}"
  button-secondary:
    backgroundColor: "transparent"
    textColor: "{colors.fg}"
    rounded: "{rounded.full}"
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

Mixar looks like what it is: open, performance-ready DJ software with nothing hidden behind a paywall. The visual system is restrained zinc neutrals, a single teal accent for action and live state, and deck identity colors (green / blue on marketing surfaces) that echo the booth without turning the UI into a light show. Density is comfortable — readable type, generous section rhythm, cards that breathe — while interactive elements feel tactile through soft hover lift and crisp pill shapes.

The marketing site (`apps/website`) is the most fully custom expression of this world: Outfit body, Space Grotesk headings, CSS token themes with light/dark via `data-theme`. The desktop app (`apps/gui-flutter`) operates in **Forui neutral** light/dark themes with domain-specific performance colors (fader accents, VU segments, cue amber). Shared brand commitments: Mixar logo (inverted on dark), teal as the primary interactive signal, honest "in development" status when applicable, and deck semantics that must stay distinguishable under pressure.

**Key Characteristics:**

- Neutral zinc surfaces with hairline borders; color earns its place
- Teal accent for primary actions and brand signal; deck green/blue for dual-deck semantics on web
- Pill buttons and badges; rounded cards (12px) and hero media (20px)
- Outfit + Space Grotesk on web; tabular figures for BPM/time via OpenType features
- Soft hover lift on CTAs; scroll reveals respect `prefers-reduced-motion`
- App UI: Forui components, 40px custom title bar, 12px desktop window radius

## Colors

A zinc-forward palette with one confident teal accent and semantic deck hues — performance clarity over decoration.

### Primary

- **Live Teal** (#0d9488 light / #2dd4bf dark): Primary CTAs, section icons, accent bullets, links-in-context on marketing. The "go" color — use for one clear action per viewport when possible.
- **Teal Depth** (#0f766e light / #5eead4 dark): Hover and pressed states on primary buttons.
- **Teal Mist** (rgba accent @ 10–12%): Badge fills, icon tile backgrounds, subtle gradient washes (e.g. developer CTA panel).

### Secondary

- **Deck A Green** (#16a34a light / #4ade80 dark): Marketing feature tiles and deck-A semantic highlights on the website.
- **Deck B Sky** (#0284c7 light / #38bdf8 dark): Marketing feature tiles and deck-B semantic highlights on the website.

### Tertiary

- **Dev Amber** (amber-400 family at ~12–15% opacity): "In development" and cautionary status badges only — not a general warning color.

### Neutral

- **Stage Black** (#18181b light fg / #f4f4f5 dark fg): Primary text.
- **Booth Muted** (#71717a light / #a1a1aa dark): Secondary copy, nav links at rest, footer meta.
- **Floor** (#fafafa light / #09090b dark): Page background.
- **Riser** (#f4f4f5 light / #121216 dark): Alternating section bands (`bg-bg-elevated`).
- **Panel** (#ffffff light / #18181c dark): Cards, theme toggle, popovers.
- **Hairline** (rgba black/white @ 8%): Borders on cards, header scroll state, inputs-at-rest.

### App-specific (Flutter / Forui)

- **Forui neutral semantic tokens** (`background`, `foreground`, `mutedForeground`, `border`, `primary`): Base app chrome — do not override with raw hex in widgets; read from `context.theme.colors`.
- **Fader Deck A sky** (#38bdf8 grip): Vertical/horizontal fader accent for Deck A in the app (differs from web deck-a green — align in a future pass).
- **Fader Deck B rose** (#fb7185 grip): Fader accent for Deck B in the app.
- **Cue Amber** (#FCD34D / ring #F59E0B @ 40%): Headphone cue / PFL active state.
- **VU ladder:** green → amber → red segments for level meters.

### Named Rules

**The One Signal Rule.** Teal is the only brand accent on a given marketing viewport. Deck green/blue appear as semantic labels, not competing primaries.

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
- **Mono** (400, 0.875rem+, tabular figures): Tempo, time, cue indices, technical values.

### App note

Flutter currently inherits **Forui neutral** type scale (`theme.typography.*`). Custom Outfit / Space Grotesk in the app is planned; until then, match web hierarchy intent using Forui roles (body, label, heading) rather than hard-coded sizes.

### Named Rules

**The Stable Numbers Rule.** Any value that changes during a set (BPM, elapsed time, bar count) uses tabular figures — `font-mono` on web, `FontFeature.tabularFigures()` in Flutter.

**The Headline Serif Rule.** Marketing `h1–h3` use Space Grotesk (`font-serif`); body UI stays Outfit (`font-sans`).

## Layout

**Marketing grid:** Centered content column `max-width: 70rem` (1120px) with `24px` horizontal padding. Sections use `80px` vertical padding (`py-20`); hero uses slightly tighter bottom spacing.

**Hero:** Two-column at `lg` — copy left, product screenshot right with 3D tilt/float motion on desktop; stacked on mobile.

**Feature grid:** 1 → 2 (`sm`) → 3 (`lg`) columns, `20px` gap.

**Developer pages:** Narrow reading column `640px` centered for prose; architecture sections may use full content width.

**App shell:** Minimum window 1024×768, default 1280×800. Deck layout: Deck A | Mixer | Deck B in a single card row (`deck_grid.dart`). Custom 40px header; content clipped to 12px top radius on undecorated Linux desktop windows.

**Responsive breakpoints (web):** Tailwind defaults — `sm` 640px, `lg` 1024px. Mobile nav collapses Features link; theme toggle always visible.

## Elevation & Depth

Mostly flat surfaces with **soft lift on interaction**. Depth comes from tonal layering (floor → riser → panel) and hairline borders before shadows. Shadows are reserved for hero product imagery and floating tooltips — not everyday cards.

### Shadow Vocabulary

- **Screenshot float** (`0 24px 64px rgba(0,0,0,0.12)` light; dark adds `0 0 0 1px rgba(255,255,255,0.03)` plus deeper drop): Hero and marketing product shots only.
- **Popover** (`shadow-lg` on platform tooltips): Small floating labels, not page chrome.
- **Header** (no shadow): Scroll state uses bottom border `border-border`, not box-shadow.

### Named Rules

**The Soft Lift Rule.** Primary buttons and interactive pills may translate `-1px` on hover. Cards lift through border brightening (`hover:border-zinc-400/25`), not shadow growth.

**The Flat Card Rule.** Feature cards and panels stay shadowless at rest; background `bg-card` + border is enough separation.

## Shapes

Rounded, approachable geometry — pills for actions, soft rectangles for content, no sharp corporate corners.

- **Full pill** (`9999px`): Buttons, badges, platform pills, theme toggle.
- **XL radius** (20px): Hero screenshot frame, developer CTA panel.
- **Large** (12px / `rounded-xl`): Feature cards, app desktop window top corners.
- **Medium** (10px): Icon tiles inside feature cards.
- **Small** (4px): Logo link hit target.

Borders are 1px hairlines; hover slightly increases border contrast rather than adding fill weight. Icons: Lucide stroke icons at 16–18px in UI chrome, 64px for section hero icons.

## Components

### Buttons (web)

- **Shape:** Full pill (rounded-full)
- **Primary:** Teal fill, white text (dark: teal-950 text on bright teal), semibold 15px, icon gap 8px; hover teal depth + 1px lift
- **Secondary:** Transparent, hairline border, hover accent-dim wash and border brighten

### Badges

- **Default:** Teal mist fill, teal text, pill, 13px medium
- **Dev:** Amber border/fill/text for in-development status

### Cards / Feature tiles

- **Corner:** 12px
- **Background:** Panel white/dark card
- **Border:** Hairline; hover border only
- **Padding:** 24px
- **Icon tile:** 40×40, 10px radius, tone from deck-a / deck-b / accent

### Navigation (web header)

- **Style:** Sticky 64px, frosted header bg + backdrop blur
- **Links:** Muted 15px; hover fg; current page semibold fg
- **Theme toggle:** 36px circle, card bg, border; icon swap sun/moon

### App chrome (Flutter)

- **Header:** 40px, background token, logo white-filtered, engine status caption, Forui tabs, window controls
- **Controls:** Forui buttons, switches, sliders; faders custom with deck accent grips
- **Tooltips:** App tooltip wrapper for performance controls
- **Panels:** Bordered deck sections; performance controls grouped in bordered panels per deck spec

### Motion

- **Scroll reveal:** Fade + 16px translate, 500ms ease, staggered 100ms; disabled when `prefers-reduced-motion`
- **Hero screenshot:** Motion-driven float/tilt on desktop; scroll-scrubbed tilt on mobile

## Do's and Don'ts

### Do:

- **Do** use semantic CSS tokens (`bg`, `fg`, `accent`, `deck-a`, `deck-b`) on the website — never hardcode zinc/teal one-offs in components.
- **Do** keep one primary teal CTA per marketing section; secondary actions use ghost/secondary button variant.
- **Do** mark unreleased platforms with reduced opacity and "On roadmap" tooltip — never full accent styling.
- **Do** read colors from `context.theme` in Flutter; use `FaderColors.forAccent` for deck faders.
- **Do** respect reduced motion for scroll reveals and hero animation.

### Don't:

- **Don't** add drop shadows to feature cards or section backgrounds — borders and tonal layers carry depth.
- **Don't** use deck green/blue as page-level accents; they are semantic deck identifiers only (on web).
- **Don't** fabricate social proof, star counts, or "available now" platform badges.
- **Don't** mix marketing deck colors (green/blue) into app fader tokens without an explicit alignment pass — surfaces currently differ.
- **Don't** hide the in-development status on the landing hero until a real release ships.
