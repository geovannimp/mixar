# Settings i18n + Language Preference Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Flutter gen-l10n with `en` + `pt_BR`, extract all Settings user-facing strings, and persist a System/English/pt_BR language preference on `AppSettings` applied on Save.

**Architecture:** ARB-driven `AppLocalizations` for Settings copy; new `UiLanguageSetting` on Rust `AppSettings`/`settings.json`; root `MaterialApp.locale` derived from saved preference (`null` = system). Language edits go through the existing Settings draft → Save path.

**Tech Stack:** Flutter gen-l10n, `flutter_localizations`/`intl`, Riverpod, flutter_rust_bridge, Rust serde settings host.

**Spec:** `docs/superpowers/specs/2026-10-10-gui-settings-i18n-design.md`

## Global Constraints

- Settings-only string extraction this PR (no deck/library chrome).
- Locales: `en`, `pt_BR` with full Portuguese Settings copy.
- Language options: system (default), `en`, `pt_br`; apply only after Save.
- Do not translate musical key names, Camelot codes, file paths, raw engine errors.
- Engine restart-on-save behavior unchanged.
- Older `settings.json` without the new field must default to system.

## Review Focus

- Missing `ui_language` in existing settings.json → loads as system, no crash.
- Forced `Locale('pt', 'BR')` in tests → Settings chrome shows Portuguese.
- Draft language change without Save → `MaterialApp.locale` stays on last saved value.
- Widget tests that pump Settings widgets without `AppLocalizations.delegate` → fail loudly; helpers must add delegates.
- System preference with unsupported device locale → Flutter falls back within `supportedLocales` (`en`/`pt_BR`) without throwing.

---

### Task 1: gen-l10n scaffolding + root delegates

**Files:**
- Modify: `apps/gui-flutter/pubspec.yaml`
- Create: `apps/gui-flutter/l10n.yaml`
- Create: `apps/gui-flutter/lib/l10n/app_en.arb` (minimal keys for smoke)
- Create: `apps/gui-flutter/lib/l10n/app_pt_BR.arb` (matching keys)
- Modify: `apps/gui-flutter/lib/main.dart`
- Create: `apps/gui-flutter/test/settings_l10n_smoke_test.dart`

**Interfaces:**
- Produces: `AppLocalizations` with at least `settingsTitle` → EN `"SETTINGS"`, PT `"CONFIGURAÇÕES"`; import path from gen-l10n output (prefer `package:gui_flutter/l10n/app_localizations.dart` if synthetic package is off).

- [ ] **Step 1: Write the failing smoke test**

```dart
testWidgets('settingsTitle localizes under pt_BR', (tester) async {
  await tester.pumpWidget(
    MaterialApp(
      locale: const Locale('pt', 'BR'),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Builder(
        builder: (context) => Text(AppLocalizations.of(context)!.settingsTitle),
      ),
    ),
  );
  await tester.pumpAndSettle();
  expect(find.text('CONFIGURAÇÕES'), findsOneWidget);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd apps/gui-flutter && mise exec -- flutter test test/settings_l10n_smoke_test.dart`
Expected: FAIL (missing l10n / AppLocalizations)

- [ ] **Step 3: Add deps, `generate: true`, `l10n.yaml`, ARB files, wire delegates in `Application`**

```bash
cd apps/gui-flutter && mise exec -- flutter pub add flutter_localizations --sdk=flutter && mise exec -- flutter pub add intl:any
```

`l10n.yaml`:
```yaml
arb-dir: lib/l10n
template-arb-file: app_en.arb
output-localization-file: app_localizations.dart
```

Wire `localizationsDelegates` to include `AppLocalizations.delegate` plus Material/Widgets/Cupertino globals; set `supportedLocales: AppLocalizations.supportedLocales`. Leave `locale:` unset for now (Task 3).

- [ ] **Step 4: Run smoke test to verify it passes**

Run: `cd apps/gui-flutter && mise exec -- flutter gen-l10n && mise exec -- flutter test test/settings_l10n_smoke_test.dart`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/gui-flutter/pubspec.yaml apps/gui-flutter/pubspec.lock apps/gui-flutter/l10n.yaml apps/gui-flutter/lib/l10n apps/gui-flutter/lib/main.dart apps/gui-flutter/test/settings_l10n_smoke_test.dart
git commit -m "feat(gui): scaffold app gen-l10n with en and pt_BR"
```

---

### Task 2: Persist `ui_language` on AppSettings (Rust + FRB)

**Files:**
- Modify: `crates/host-flutter/src/api/settings.rs`
- Regenerate: `apps/gui-flutter/lib/src/rust/**` via `moon run gui-flutter:generate`
- Modify: `apps/gui-flutter/lib/settings/settings_defaults.dart` (`copyAppSettings`, `appSettingsDirty`, defaults)

**Interfaces:**
- Produces: `enum UiLanguageSetting { system, en, ptBr }` (Rust serde `snake_case`: `system`, `en`, `pt_br`); field `ui_language` / Dart `uiLanguage` on `AppSettings`; default `system`.

- [ ] **Step 1: Write failing Rust tests** (in `settings.rs` test module)

- `missing_ui_language_defaults_system`
- `ui_language_round_trip_survives_reload` for `pt_br` (and assert `en` if cheap)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p host-flutter missing_ui_language_defaults_system ui_language_round_trip -- --nocapture`
Expected: FAIL (field/enum missing)

- [ ] **Step 3: Implement enum + field through host load/save/defaults; regenerate FRB; update Dart copy/dirty/defaults**

- [ ] **Step 4: Re-run Rust tests + a focused Dart defaults test if present**

Run: `cargo test -p host-flutter ui_language -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/host-flutter/src/api/settings.rs apps/gui-flutter/lib/src/rust apps/gui-flutter/rust_builder apps/gui-flutter/lib/settings/settings_defaults.dart
git commit -m "feat(settings): persist ui_language preference"
```

---

### Task 3: Language control + apply locale after Save

**Files:**
- Modify: `apps/gui-flutter/lib/settings/settings_ui_panel.dart`
- Modify: `apps/gui-flutter/lib/main.dart` (ConsumerWidget / locale from `appSettingsProvider`)
- Optionally create: `apps/gui-flutter/lib/settings/ui_language.dart` helper `Locale? localeFromUiLanguage(UiLanguageSetting)`
- Modify: `apps/gui-flutter/test/settings_l10n_smoke_test.dart` or add locale mapping unit test
- Extend ARB: `settingsUiLanguageLabel`, `settingsLanguageSystemDefault` (language endonyms stay hardcoded in Dart)

**Interfaces:**
- Consumes: `AppSettings.uiLanguage`, `AppLocalizations`
- Produces: `Locale? localeFromUiLanguage(UiLanguageSetting value)` → `null` / `Locale('en')` / `Locale('pt','BR')`
- UI select options: `[system, en, ptBr]` on Settings → UI panel

- [ ] **Step 1: Write failing unit test for `localeFromUiLanguage`**

```dart
expect(localeFromUiLanguage(UiLanguageSetting.system), isNull);
expect(localeFromUiLanguage(UiLanguageSetting.en), const Locale('en'));
expect(localeFromUiLanguage(UiLanguageSetting.ptBr), const Locale('pt', 'BR'));
```

- [ ] **Step 2: Run to verify fail → implement helper + UI select + root `locale:` from saved settings (loading → null)**

- [ ] **Step 3: Run unit test + analyze**

Run: `cd apps/gui-flutter && mise exec -- flutter test test/settings_l10n_smoke_test.dart && mise exec -- flutter analyze --no-fatal-infos`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(gui): add Settings language control applied on save"
```

---

### Task 4: Extract all Settings strings to ARB (en + pt_BR)

**Files:**
- Modify: `apps/gui-flutter/lib/l10n/app_en.arb`, `app_pt_BR.arb`
- Modify: every Settings UI file under `apps/gui-flutter/lib/settings/` that contains user-facing literals (`settings_page`, sidebar/section, all `settings_*_panel`, `mixxx_import_panel`, `controller_mapping_row`, etc.)
- Modify: `apps/gui-flutter/test/support/` or settings tests to include `AppLocalizations` delegates
- Modify: `apps/gui-flutter/test/settings_sidebar_test.dart` (and any panel tests) to assert via l10n or provide delegates + English strings

**Interfaces:**
- Consumes: `AppLocalizations.of(context)!`
- `SettingsSection.label` becomes `String label(AppLocalizations l10n)` (or equivalent context-based API)
- Placeholders for interpolated toasts/dialogs (error counts, bytes, minutes)

- [ ] **Step 1: Expand ARBs with all Settings keys (English template first, then full pt_BR)**

Key naming: camelCase by surface, e.g. `settingsSave`, `settingsSectionAudio`, `settingsAudioBackendLabel`. Keep `−`/`+` steppers as literals if purely symbolic.

- [ ] **Step 2: Replace hardcoded Settings copy with `l10n.*`; run `flutter gen-l10n`**

- [ ] **Step 3: Fix tests — add delegates via shared helper; keep English assertions or use `AppLocalizationsEn`**

- [ ] **Step 4: Verify**

Run:
```bash
cd apps/gui-flutter && mise exec -- flutter gen-l10n && mise exec -- flutter test test/settings_sidebar_test.dart test/settings_storage_panel_test.dart test/settings_library_midi_test.dart test/settings_defaults_test.dart test/settings_l10n_smoke_test.dart
mise exec -- flutter analyze --no-fatal-infos
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(gui): localize Settings strings for en and pt_BR"
```

---

### Task 5: End-to-end locale chrome test + final verification

**Files:**
- Modify/Create: `apps/gui-flutter/test/settings_l10n_smoke_test.dart` (pump `SettingsSidebar` or page header under `pt_BR`, expect Portuguese section/title)

- [ ] **Step 1: Write widget test that Portuguese locale changes Settings chrome**

- [ ] **Step 2: Run full gui-flutter tests + host-flutter ui_language tests**

```bash
cargo test -p host-flutter ui_language
cd apps/gui-flutter && mise exec -- flutter test
```

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git commit -m "test(gui): assert Settings chrome follows pt_BR locale"
```
