# Fader Slider Markers Implementation Plan

> **For agentic workers:** User approved approach 1; proceed to implement.

**Goal:** Markers on all DJ faders, center notch on tempo/crossfader, deck-colored thumb grip.

**Architecture:** Extend shared `Slider` with `showMarkers` / `centerNotch`; accent grip via `DECK_ACCENTS`; wire call sites.

**Tech Stack:** React, Tailwind, Base UI Slider.

---

### Task 1: Tokens + Slider UI

- Add fader grip accent classes on `DECK_ACCENTS`.
- Implement markers + center notch + colored grip in `slider.tsx`.

### Task 2: Call sites

- Volume / tempo / crossfader: enable markers; tempo + crossfader: `centerNotch`.

### Task 3: Verify

- Typecheck / lint touched files.
