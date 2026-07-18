# Fader Slider Markers Design

**Date:** 2026-07-18  
**Scope:** GUI only — `apps/gui-app` fader-variant sliders

## Goal

Match classic DJ fader chrome: side tick markers on volume/tempo/crossfader, a stronger center stick on centered faders (tempo + crossfader), and a deck-colored center line in the thumb/dragger.

## Requirements

| Decision | Choice |
|----------|--------|
| Scope of markers | All DJ faders: volume, tempo, crossfader |
| Center stick | Tempo + crossfader only (`centerNotch`) |
| Thumb grip color | Deck accent when `channelAccent` set; neutral for crossfader |
| Snap to center | No — visual only |
| Settings sliders | Unchanged (default thumb variant) |

## Approach

Extend `Slider` (`apps/gui-app/src/components/ui/slider.tsx`) with:

- `showMarkers?: boolean` — hierarchical ticks beside the track
- `centerNotch?: boolean` — short brighter stick at 50%

Color the existing fader thumb `after:` grip line with `DECK_ACCENTS[accent].fader` (new grip class) when `channelAccent` is set; keep `FADER_KNOB.grip` neutral otherwise.

### Marker hierarchy (reference pitch fader)

Along the track (0% → 100%):

- **Major:** 0%, 50%, 100% (longer ticks)
- **Mid:** 25%, 75%
- **Minor:** evenly between majors/mids (four per quarter, matching reference)

Vertical: ticks on both sides of the track.  
Horizontal (crossfader): ticks above and below.

### Call sites

| Slider | `showMarkers` | `centerNotch` | `channelAccent` |
|--------|---------------|---------------|-----------------|
| Volume (`DeckMixer`) | yes | no | a / b |
| Tempo (`DeckTempoPanel`) | yes | yes | a / b |
| Crossfader (`DeckMixer`) | yes | yes | none |

## Success criteria

1. Volume, tempo, and crossfader show side markers with major/mid/minor hierarchy.
2. Tempo and crossfader show a distinct center stick at 50%.
3. Volume/tempo thumbs use deck-colored center grip lines; crossfader stays neutral.
4. Settings page sliders unchanged.

## Files likely touched

- `apps/gui-app/src/components/ui/slider.tsx`
- `apps/gui-app/src/lib/ui.ts` (grip accent tokens)
- `apps/gui-app/src/components/DeckMixer.tsx`
- `apps/gui-app/src/components/DeckTempoPanel.tsx`
