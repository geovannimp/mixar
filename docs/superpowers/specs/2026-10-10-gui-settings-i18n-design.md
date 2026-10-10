# Settings i18n + Language preference — Design

## Intent

User-visible Settings copy in `apps/gui-flutter` is hardcoded English. The app already mounts Material localization delegates, but there is no app-owned `AppLocalizations` / `.arb` set. This work adds Flutter gen-l10n, extracts **all Settings** strings into `en` + `pt_BR`, and adds a Settings → UI → Language control so people can follow the system locale or force English / Português (Brasil).

## Why now

Hardcoded Settings chrome is the main localization debt that blocks shipping a Portuguese-friendly desktop UI. Locale infrastructure is half-present on `MaterialApp`; finishing the app-owned layer unblocks incremental extraction elsewhere later.

## Goals

- Flutter gen-l10n runs for `gui-flutter` (`generate: true`, `l10n.yaml`, ARB templates).
- Every user-facing string in the Settings flow uses `AppLocalizations.of(context)` (page chrome, sidebar section labels, all section panels, confirm dialogs, toasts in that flow).
- Supported app locales: `en`, `pt_BR`.
- Full Portuguese (Brazil) copy for Settings on first ship (not an English stub).
- Language preference: **System default**, **English**, **Português (Brasil)**.
- Preference lives on `AppSettings` / `settings.json`, edited in the draft and applied only on **Save** (same path as Show tooltips).
- After save (and on cold start), `MaterialApp.locale` reflects the preference (`null` for system).
- Analyze / unit tests still pass; at least one test forces a locale and asserts Settings chrome changes.

## Non-goals

- Localizing deck chrome, library empty states, mixer shell, or other non-Settings UI in this pass.
- Translating docs / README.
- Skipping engine restart when only language changed (current Save may restart if the engine is running — keep that).
- Translating musical key names, Camelot codes, file paths, or raw engine error codes.
- Shipping additional locales beyond `en` and `pt_BR`.

## Anti-goals

- Replacing Material / widget-kit localizations with a custom system.
- Live locale switching on dropdown change before Save (would diverge from other Settings).
- A parallel Flutter-only preferences file for language while other UI prefs stay on `AppSettings`.

## Constraints

- Package: `apps/gui-flutter`.
- Persistence: extend Rust `AppSettings` in `crates/host-flutter/src/api/settings.rs`, regenerate FRB bindings (`moon run gui-flutter:generate` / existing generate task).
- Existing UI: Forui is not the current shell — root is `MaterialApp` + Mixar/Shad theme. Wire `AppLocalizations.delegate` alongside `GlobalMaterialLocalizations` (and widgets/cupertino delegates as needed).
- Do not break missing-field defaults for older `settings.json` files.
- Keep enum/option **values** that are protocol or device identifiers untranslated; only labels shown to humans go through l10n.

## UX

### Language control

- Location: Settings → **UI** panel, with Show tooltips.
- Control: `SettingsSelect` with three options:
  1. System default
  2. English
  3. Português (Brasil)
- Changing the select only updates the draft (marks dirty). Locale of the running UI changes after successful Save (and on next launch from persisted settings).
- **Builder call:** Language *names* in the select stay endonyms / fixed (`English`, `Português (Brasil)`). Only “System default” (and the field label) go through ARB.

### Locale resolution

| Preference | `MaterialApp.locale` |
|------------|----------------------|
| System default | `null` (platform locale; Flutter falls back within `supportedLocales`) |
| English | `Locale('en')` |
| Português (Brasil) | `Locale('pt', 'BR')` |

`supportedLocales` must include both app locales (and remain compatible with Material delegates).

## Architecture

### gen-l10n

- Add `flutter_localizations` (SDK) + `intl` if not already direct deps.
- `pubspec.yaml`: `flutter.generate: true`.
- `apps/gui-flutter/l10n.yaml`:
  - `arb-dir: lib/l10n`
  - `template-arb-file: app_en.arb`
  - `output-localization-file: app_localizations.dart`
- ARB files: `app_en.arb`, `app_pt_BR.arb`.
- Access pattern: `final l10n = AppLocalizations.of(context)!;` (or nullable-safe equivalent used by the project).

### Persistence

- New field on Rust `AppSettings`, e.g. `ui_language: UiLanguageSetting` (name up to implementer if equivalent).
- Enum variants (serde `snake_case`): `system` (default), `en`, `pt_br`.
- Mirror through `SettingsHost`, `settings_from_host` / `apply_to_host`, defaults for missing JSON keys, round-trip tests in the Rust settings module (same style as `show_tooltips`).
- Flutter: extend generated `AppSettings`, `copyAppSettings`, `appSettingsDirty`, `defaultAppSettings`, UI panel select.

### Applying locale at runtime

- Root `Application` must read saved settings (Riverpod `appSettingsProvider` or equivalent) and set `MaterialApp.locale` from the preference.
- While settings are still loading, treat as system (`locale: null`).
- After Save invalidates `appSettingsProvider`, the root rebuilds and the new locale applies — including Settings chrome already open.

### String extraction scope

Extract hardcoded user-facing copy under:

- `settings_page.dart` (title, subtitle, loading, save/busy, toasts, unsaved dialog)
- `settings_sidebar.dart` / `settings_section.dart` labels
- All `settings_*_panel.dart` section headers, field labels, descriptions, empty/loading labels, dialog/toast titles in those panels
- Related Settings-only helpers that surface copy from Settings (e.g. Mixxx import panel strings shown from Settings → Library/Storage as applicable)

Dynamic interpolation (counts, error payloads) uses ARB placeholders; raw exception text may remain as description body.

## Testing

- Rust: default when key missing; round-trip persist/reload for each language variant.
- Flutter: widget/unit test that pumps Settings (or a thin chrome widget) under `locale: Locale('pt', 'BR')` and expects a known Portuguese string (e.g. Settings title or UI section label).
- Existing settings panel tests updated to provide localization delegates where they pump widgets that now call `AppLocalizations`.

## Out of scope for builder invention (fixed decisions)

- Languages: system + en + pt_BR only.
- Apply on Save only; engine restart behavior unchanged.
- Full pt_BR Settings strings required.
- Settings-only extraction this PR.

## Left to the builder

- Exact ARB key naming scheme (prefer stable, descriptive camelCase keys).
- Exact Rust enum/field names if they match the semantics above.
- Whether `Application` becomes a `ConsumerWidget` vs a small locale bridge widget under `ProviderScope`.
- Packaging of large ARB files (single file per locale is fine).
- Import path for generated localizations (`package:flutter_gen/...` vs `package:gui_flutter/l10n/...` depending on `synthetic-package` / Flutter version defaults).
