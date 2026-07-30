# Settings TanStack Form Migration Design

**Issue:** [#113](https://github.com/geovannimp/rust-dj-engine/issues/113)  
**Date:** 2026-07-30

## Goal

Replace the hand-rolled `draft` / `onChange` settings state on the Settings page with [TanStack Form](https://tanstack.com/form) using **form composition** (`createFormHook` / `withForm`), scoped to settings only.

## Requirements

| Decision | Choice |
|----------|--------|
| Library | `@tanstack/react-form` in `apps/gui-app` |
| Composition | `createFormHookContexts` + `createFormHook`; panels via `withForm` |
| Hook scope | Settings-only under `apps/gui-app/src/components/settings/` (not app-wide) |
| Form values | Full `AppSettings` shape |
| Defaults / reset | `normalizeAppSettings(settings)` on load and after successful save / remote refresh |
| Save path | Existing `useSettings().save` → Tauri `save_settings` |
| UI | Keep coss components (`Field`, `Input`, `Select`, `Switch`, `Slider`, etc.) |
| Dirty Save button | Same as today: disabled while `busy` only (no new “must be dirty” gate) |
| Validation | No new rules; keep `normalizeAppSettings` + existing snaps (buffer size, LUFS bounds) |

**Out of scope:** New settings fields, migrating other forms, `withFieldGroup`, app-wide form hook.

## Architecture

```text
apps/gui-app/src/components/settings/
  form-context.ts       # createFormHookContexts()
  form.ts               # createFormHook → useAppForm, withForm, pre-bound components
  fields/               # useFieldContext-bound wrappers around coss UI
  SettingsPage.tsx      # useAppForm; AppForm shell; section switch; submit
  SettingsAudioPanel    # withForm({ defaultValues: AppSettings … })
  SettingsLibraryPanel  # withForm({ defaultValues: AppSettings … })
```

- `useSettings` remains the persistence/busy/error/saved layer.
- Device listing stays on the page: subscribe to `backend` from form values, pass `devices` / `devicesLoading` into the audio panel as props.
- Drop page-level `useState(draft)` and the `useEffect` that copies `settings` → draft.
- Drop panel props `draft` / `onChange: (next: AppSettings) => void`.

## Components

### Form hook

`form-context.ts` exports `fieldContext`, `formContext`, `useFieldContext`, `useFormContext`.

`form.ts` registers:

| Kind | Name | Role |
|------|------|------|
| Field | `NumberField` | Labeled number `Input` via `useFieldContext<number>()` |
| Field | `SelectField` | Wraps existing `SettingsSelect` for string unions |
| Field | `ToggleField` | Wraps `SettingsToggle` / Switch |
| Field | `SliderField` | Labeled `Slider` + optional display |
| Form | `SaveButton` | `useFormContext` + subscribe; disabled when parent passes `busy` or while submitting |

Panels use `form.AppField` + `field.NumberField` / etc. Nested keys use TanStack dotted paths (`master_bus.device_id`, `deck_default_sampler_bank_id[0]`, …).

Controls that need multi-field updates (mono/stereo channel mode, library column set, resampler quality index mapping) call `form.setFieldValue` / replace a nested object from the panel `render` — not forced into a single pre-bound field.

### Page flow

1. Wait until `settings` from `useSettings` is non-null (loading UI unchanged).
2. `useAppForm({ defaultValues: normalizeAppSettings(settings), onSubmit: async ({ value }) => { await save(value) } })`.
3. When `settings` identity/content updates from save/refresh, `form.reset(normalizeAppSettings(settings))`.
4. Wrap body in `form.AppForm`; native `<form onSubmit={form.handleSubmit}>`.
5. Section panels: `<SettingsAudioPanel form={form} devices={…} devicesLoading={…} />` / library equivalent.

### Nested bus / sampler

`BusChannelFields` stays a local helper but reads/writes via `form` (field names under a bus prefix passed as props, or `setFieldValue` on the whole `BusRouteSettings`). Master and preview cards remain two instances; no `withFieldGroup` in this change.

`DeviceSelect` keeps its presentational API; parent wires it through `AppField` / `setFieldValue` for `*.device_id`.

## Data flow

```text
get_settings → useSettings.settings
                 → normalizeAppSettings → form defaultValues / reset
user edits → form store (AppSettings)
      submit → save(form values) → save_settings
                 → updated settings → reset form
```

Engine restart / toast-on-failure behavior stays inside `useSettings.save`.

## Error handling

- Load failure: existing `error` banner from `useSettings`.
- Save failure: existing error banner + toast; form values remain (user can retry).
- No form-level schema validators in this migration.

## Testing

- Typecheck / lint for `gui-app` (`tsc` / oxlint as already wired).
- Manual: load settings, edit audio + library fields (including bus mode and columns), save, confirm persist + form reset; change backend and confirm device list refreshes.

## Acceptance mapping

| Criterion | How |
|-----------|-----|
| Load into defaults; refresh resets | `defaultValues` + `form.reset` on `settings` change |
| Same `AppSettings` + `save_settings` | Unchanged `useSettings.save` |
| Dirty/submit UX ≥ today | Busy-disabled Save; banners/toasts unchanged |
| Devices + nested bus/sampler | Backend subscribe + existing panel logic on form APIs |
