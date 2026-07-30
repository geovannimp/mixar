# Settings TanStack Form Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate the Settings page from hand-rolled `draft`/`onChange` state to settings-scoped TanStack Form composition (`createFormHook` / `withForm`).

**Architecture:** Settings-local `createFormHook` with pre-bound field/form components; `SettingsPage` owns `useAppForm` over `AppSettings`; audio/library panels are `withForm` children; `useSettings` stays the Tauri save/load layer.

**Tech Stack:** `@tanstack/react-form`, React 19, existing coss UI in `apps/gui-app`.

## Global Constraints

- Form composition only under `apps/gui-app/src/components/settings/` (not app-wide).
- Preserve `useSettings().save`, busy/disabled Save, error/success banners, restart toast.
- Preserve coss UI; no new settings fields or validation beyond `normalizeAppSettings` / existing snaps.
- No `withFieldGroup` in this change.
- Spec: `docs/superpowers/specs/2026-07-30-settings-tanstack-form-design.md`.

## File map

| File | Responsibility |
|------|----------------|
| `apps/gui-app/package.json` | Add `@tanstack/react-form` |
| `settings/form-context.ts` | `createFormHookContexts` exports |
| `settings/form.ts` | `createFormHook`, `useAppForm`, `withForm`, register components |
| `settings/fields/*.tsx` | Pre-bound Number/Select/Toggle/Slider fields |
| `settings/SettingsSaveButton.tsx` | Form-context Save button (`busy` prop) |
| `settings/settingsFormOptions.ts` | `formOptions` + typed default `AppSettings` for `withForm` |
| `SettingsAudioPanel.tsx` | `withForm` + `AppField` / `setFieldValue` |
| `SettingsLibraryPanel.tsx` | `withForm` + `AppField` / `setFieldValue` |
| `pages/SettingsPage.tsx` | `useAppForm`, reset, submit, device subscribe |

---

### Task 1: Dependency + form scaffolding

**Files:**
- Modify: `apps/gui-app/package.json`
- Create: `apps/gui-app/src/components/settings/form-context.ts`
- Create: `apps/gui-app/src/components/settings/settingsFormOptions.ts`
- Create: `apps/gui-app/src/components/settings/form.ts` (stub fieldComponents first, then fill in Task 2)
- Create: `apps/gui-app/src/components/settings/fields/NumberField.tsx`
- Create: `apps/gui-app/src/components/settings/fields/SelectField.tsx`
- Create: `apps/gui-app/src/components/settings/fields/ToggleField.tsx`
- Create: `apps/gui-app/src/components/settings/fields/SliderField.tsx`
- Create: `apps/gui-app/src/components/settings/SettingsSaveButton.tsx`

**Interfaces:**
- Produces: `useAppForm`, `withForm`, `settingsFormOptions` (`formOptions({ defaultValues: AppSettings })`), field components on `form.AppField` children, `SaveButton` on `form.AppForm`

- [ ] **Step 1:** Install `@tanstack/react-form` in `gui-app` (match caret style of other `@tanstack/*` deps).

```bash
npm install @tanstack/react-form -w gui-app
```

- [ ] **Step 2:** Add `form-context.ts`:

```ts
import { createFormHookContexts } from "@tanstack/react-form";

export const { fieldContext, formContext, useFieldContext, useFormContext } =
  createFormHookContexts();
```

- [ ] **Step 3:** Add `settingsFormOptions.ts` using a typed placeholder `defaultValues` built from existing defaults in `busSettings.ts` / library defaults (keys only matter for `withForm` typing; runtime page supplies real defaults). Export `settingsFormOptions = formOptions({ defaultValues })`.

- [ ] **Step 4:** Implement field components with `useFieldContext`:
  - `NumberField`: `{ label, min?, max?, step?, "aria-label"? }` → `SettingsField` + `Input type="number"`; `handleChange(Number(e.target.value) || field.state.value)`
  - `SelectField`: `{ label, "aria-label", options: SettingsSelectOption<string>[], hint? }` → `SettingsField` + `SettingsSelect`
  - `ToggleField`: `{ label }` → `SettingsToggle` bound to boolean field
  - `SliderField`: `{ label, min, max, step, disabled?, formatValue?: (n) => string, description? }` → coss `Slider` / `FieldLabel` / `FieldDescription`

- [ ] **Step 5:** `SettingsSaveButton`: `useFormContext()`; props `{ busy: boolean; label?: string }`; `form.Subscribe` to `isSubmitting`; disable when `busy || isSubmitting`; label `"Saving…"` when busy/submitting else `"Save"`.

- [ ] **Step 6:** Wire `createFormHook` in `form.ts` registering those fieldComponents + `SaveButton` formComponent; export `useAppForm`, `withForm`.

- [ ] **Step 7:** Commit scaffolding.

---

### Task 2: Migrate SettingsLibraryPanel to `withForm`

**Files:**
- Modify: `apps/gui-app/src/components/settings/SettingsLibraryPanel.tsx`

**Interfaces:**
- Consumes: `withForm`, `settingsFormOptions`, field components
- Produces: `<SettingsLibraryPanel form={form} />` (no draft/onChange)

- [ ] **Step 1:** Replace export with:

```tsx
export const SettingsLibraryPanel = withForm({
  ...settingsFormOptions,
  render: function Render({ form }) {
    // analysis_duration Select (object options) via form.AppField name="analysis_duration"
    // scan_folder_tree via form.AppField + field.ToggleField
    // library_table_columns via form.Subscribe / setFieldValue + normalizeLibraryTableColumns
  },
});
```

- [ ] **Step 2:** Ensure analysis mode `Select` still uses `ANALYSIS_MODE_OPTIONS` / `findAnalysisModeOption`; on change `field.handleChange(item.value)`.

- [ ] **Step 3:** Typecheck; commit.

---

### Task 3: Migrate SettingsAudioPanel to `withForm`

**Files:**
- Modify: `apps/gui-app/src/components/settings/SettingsAudioPanel.tsx`

**Interfaces:**
- Consumes: `withForm`, `settingsFormOptions`
- Produces: `<SettingsAudioPanel form={form} devices={…} devicesLoading={…} />`

- [ ] **Step 1:** Convert to `withForm` with props `{ devices: AudioDeviceSummary[]; devicesLoading: boolean }`.

- [ ] **Step 2:** Map each simple control to `form.AppField` + pre-bound field (backend, sample_rate, buffer_size snap in onChange, low_latency, resampler index mapping, normalizer, LUFS, sampler selects, deck bank selects).

- [ ] **Step 3:** Keep `BusChannelFields` / `DeviceSelect` / preview toggle; wire with `form.setFieldValue("master_bus", …)` / dotted paths / `AppField` as appropriate. Backend change must update form value so page device hook refreshes.

- [ ] **Step 4:** Typecheck; commit.

---

### Task 4: Wire SettingsPage

**Files:**
- Modify: `apps/gui-app/src/pages/SettingsPage.tsx`

**Interfaces:**
- Consumes: `useAppForm`, `settingsFormOptions`, migrated panels, `SaveButton`

- [ ] **Step 1:** When `!settings`, keep loading UI.

- [ ] **Step 2:**

```tsx
const form = useAppForm({
  ...settingsFormOptions,
  defaultValues: normalizeAppSettings(settings),
  onSubmit: async ({ value }) => {
    await save(value);
  },
});
```

- [ ] **Step 3:** `useEffect` on `settings` → `form.reset(normalizeAppSettings(settings))` (including after save).

- [ ] **Step 4:** Subscribe to backend for `useAudioDevices`:

```tsx
const backend = useStore(form.store, (s) => s.values.backend);
// or form.Subscribe — prefer whatever TanStack Form docs use with createFormHook
```

If `useStore` from `@tanstack/react-form` / `@tanstack/react-store` is the documented pattern, use it; otherwise `form.Subscribe selector={(s) => s.values.backend}`.

- [ ] **Step 5:** Render `form.AppForm` > `<form onSubmit={(e) => { e.preventDefault(); void form.handleSubmit(); }}>` with section panel + `form.SaveButton busy={busy}` (or `SettingsSaveButton` registered name).

- [ ] **Step 6:** Remove `draft` / `setDraft` / old `SettingsSectionPanel` draft props.

- [ ] **Step 7:** Run `npm run build -w gui-app` (or `tsc`) and `npm run lint -w gui-app`; fix errors. Commit.

---

### Task 5: Branch hygiene + PR

- [ ] Ensure design + plan docs are committed.
- [ ] Push branch and open PR linked to #113 with summary + test plan.
