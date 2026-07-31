//! Sampler banks — persist, assign, and trigger one-shots.

use serde::{Deserialize, Serialize};
use strum::EnumString;
use tauri::{AppHandle, State};

use library::{SamplerBankRecord, SamplerPlayMode as LibPlayMode, TrackId};
use library_core::{AudioSource, FileAudioSource, Library};

use engine_core::SamplerPlayMode as DspPlayMode;

use crate::engine_controller::{publish_deck, publish_status};
use crate::{with_engine, AppState, SharedAppState, NUM_DECKS};

pub const SAMPLER_SLOT_COUNT: usize = library::SAMPLER_BANK_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SamplerPlayModeSetting {
    #[default]
    Oneshot,
    Hold,
    Loop,
}

impl SamplerPlayModeSetting {
    pub fn to_dsp(self) -> DspPlayMode {
        match self {
            Self::Oneshot => DspPlayMode::Oneshot,
            Self::Hold => DspPlayMode::Hold,
            Self::Loop => DspPlayMode::Loop,
        }
    }

    pub fn from_lib(mode: LibPlayMode) -> Self {
        match mode {
            LibPlayMode::Oneshot => Self::Oneshot,
            LibPlayMode::Hold => Self::Hold,
            LibPlayMode::Loop => Self::Loop,
        }
    }

    pub fn to_lib(self) -> LibPlayMode {
        match self {
            Self::Oneshot => LibPlayMode::Oneshot,
            Self::Hold => LibPlayMode::Hold,
            Self::Loop => LibPlayMode::Loop,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SamplerSlotInfo {
    pub label: Option<String>,
    pub track_id: Option<String>,
    pub path: Option<String>,
    pub duration_ms: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplerBankInfo {
    pub id: String,
    pub name: String,
    /// `None` = inherit settings.
    pub play_mode: Option<SamplerPlayModeSetting>,
    pub sort_index: i32,
}

impl From<SamplerBankRecord> for SamplerBankInfo {
    fn from(bank: SamplerBankRecord) -> Self {
        Self {
            id: bank.id,
            name: bank.name,
            play_mode: bank.play_mode.map(SamplerPlayModeSetting::from_lib),
            sort_index: bank.sort_index,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SamplerStatus {
    pub banks: Vec<SamplerBankInfo>,
    pub active_bank_id: Option<String>,
    pub active_bank_name: Option<String>,
    pub bank_play_mode: Option<SamplerPlayModeSetting>,
    pub deck_slots: Vec<Vec<SamplerSlotInfo>>,
    pub effective_play_modes: Vec<SamplerPlayModeSetting>,
}

impl SamplerStatus {
    pub fn from_state(state: &AppState) -> Self {
        let mut banks: Vec<SamplerBankInfo> = state
            .library
            .lock()
            .unwrap()
            .list_sampler_banks()
            .unwrap_or_default()
            .into_iter()
            .map(SamplerBankInfo::from)
            .collect();

        if let Some(draft) = state.draft_sampler_bank.clone() {
            banks.push(draft);
        }

        let active_bank_id = state.loaded_sampler_bank_id[0]
            .clone()
            .or_else(|| state.decks[0].active_sampler_bank_id.clone())
            .or_else(|| state.deck_default_sampler_bank_id[0].clone())
            .or_else(|| banks.first().map(|b| b.id.clone()));

        let active = active_bank_id
            .as_ref()
            .and_then(|id| banks.iter().find(|b| &b.id == id));

        let active_bank_name = active.map(|b| b.name.clone());
        let bank_play_mode = active.and_then(|b| b.play_mode);
        let deck_slots = state
            .sampler_slots
            .iter()
            .map(|slots| slots.to_vec())
            .collect();
        let effective_play_modes = (0..NUM_DECKS)
            .map(|deck_id| effective_play_mode_for_deck(state, deck_id, &banks))
            .collect();

        Self {
            banks,
            active_bank_id,
            active_bank_name,
            bank_play_mode,
            deck_slots,
            effective_play_modes,
        }
    }
}

fn effective_play_mode_for_deck(
    state: &AppState,
    deck_id: usize,
    banks: &[SamplerBankInfo],
) -> SamplerPlayModeSetting {
    let bank_id = state.loaded_sampler_bank_id[deck_id]
        .clone()
        .or_else(|| state.decks[deck_id].active_sampler_bank_id.clone())
        .or_else(|| state.deck_default_sampler_bank_id[deck_id].clone());
    let bank_mode = bank_id
        .as_ref()
        .and_then(|id| banks.iter().find(|b| &b.id == id))
        .and_then(|b| b.play_mode);
    effective_play_mode(bank_mode, state.sampler_play_mode)
}

pub fn empty_sampler_slots() -> [SamplerSlotInfo; SAMPLER_SLOT_COUNT] {
    std::array::from_fn(|_| SamplerSlotInfo::default())
}

pub fn empty_deck_sampler_slots() -> [[SamplerSlotInfo; SAMPLER_SLOT_COUNT]; NUM_DECKS] {
    std::array::from_fn(|_| empty_sampler_slots())
}

pub fn effective_play_mode(
    bank_mode: Option<SamplerPlayModeSetting>,
    settings_mode: SamplerPlayModeSetting,
) -> SamplerPlayModeSetting {
    bank_mode.unwrap_or(settings_mode)
}

fn slot_label(source: &AudioSource) -> String {
    let metadata = source.metadata();
    match (&metadata.title, &metadata.artist) {
        (Some(title), Some(artist)) => format!("{artist} — {title}"),
        (Some(title), None) => title.clone(),
        _ => source
            .file()
            .and_then(|file| file.path().file_stem())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Sample".to_string()),
    }
}

fn resolve_bank_id(
    state: &mut AppState,
    bank_id: Option<&str>,
    deck_id: usize,
) -> Result<String, String> {
    if let Some(id) = bank_id {
        return Ok(id.to_string());
    }
    if let Some(id) = state
        .decks
        .get(deck_id)
        .and_then(|d| d.active_sampler_bank_id.clone())
    {
        return Ok(id);
    }
    if let Some(id) = state
        .deck_default_sampler_bank_id
        .get(deck_id)
        .cloned()
        .flatten()
    {
        return Ok(id);
    }
    if let Some(draft) = &state.draft_sampler_bank {
        return Ok(draft.id.clone());
    }
    if let Some(bank) = state
        .library
        .lock()
        .unwrap()
        .list_sampler_banks()
        .map_err(|e| e.to_string())?
        .into_iter()
        .next()
    {
        return Ok(bank.id);
    }
    Ok(start_draft_sampler_bank(state, deck_id, None, None)?.id)
}

pub(crate) fn apply_effective_play_mode(state: &mut AppState, deck_id: usize) -> Result<(), String> {
    let bank_id = resolve_bank_id(state, None, deck_id)?;
    apply_effective_play_mode_for_bank(state, deck_id, &bank_id)
}

pub(crate) fn load_bank_into_engine(
    state: &mut AppState,
    deck_id: usize,
    bank_id: &str,
) -> Result<(), String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }
    if state.loaded_sampler_bank_id[deck_id].as_deref() == Some(bank_id) {
        return apply_effective_play_mode_for_bank(state, deck_id, bank_id);
    }

    if state
        .draft_sampler_bank
        .as_ref()
        .is_some_and(|draft| draft.id == bank_id)
    {
        with_engine(state, |engine| {
            engine
                .clear_all_sampler_slots(deck_id)
                .map_err(|e| e.to_string())
        })?;
        state.sampler_slots[deck_id] = empty_sampler_slots();
        state.loaded_sampler_bank_id[deck_id] = Some(bank_id.to_string());
        return apply_effective_play_mode_for_bank(state, deck_id, bank_id);
    }

    let slots = state
        .library
        .lock()
        .unwrap()
        .list_sampler_bank_slots(bank_id)
        .map_err(|e| e.to_string())?;

    with_engine(state, |engine| {
        engine
            .clear_all_sampler_slots(deck_id)
            .map_err(|e| e.to_string())
    })?;

    state.sampler_slots[deck_id] = empty_sampler_slots();

    for record in slots {
        let slot = usize::from(record.slot_index);
        if slot >= SAMPLER_SLOT_COUNT {
            continue;
        }
        let (source, path, duration_ms, loudness) =
            load_source_for_slot(state, record.track_id.as_deref(), record.path.as_deref())?;
        let label = record.label.unwrap_or_else(|| slot_label(&source));
        with_engine(state, |engine| {
            engine
                .assign_sampler_slot(deck_id, slot, source, label.clone(), loudness)
                .map_err(|e| e.to_string())
        })?;
        state.sampler_slots[deck_id][slot] = SamplerSlotInfo {
            label: Some(label),
            track_id: record.track_id,
            path: Some(path),
            duration_ms,
        };
    }

    state.loaded_sampler_bank_id[deck_id] = Some(bank_id.to_string());
    apply_effective_play_mode_for_bank(state, deck_id, bank_id)
}

fn apply_effective_play_mode_for_bank(
    state: &mut AppState,
    deck_id: usize,
    bank_id: &str,
) -> Result<(), String> {
    let bank_mode = if let Some(draft) = state
        .draft_sampler_bank
        .as_ref()
        .filter(|draft| draft.id == bank_id)
    {
        draft.play_mode
    } else {
        state
            .library
            .lock()
            .unwrap()
            .get_sampler_bank(bank_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Sampler bank not found: {bank_id}"))?
            .play_mode
            .map(SamplerPlayModeSetting::from_lib)
    };
    let mode = effective_play_mode(bank_mode, state.sampler_play_mode);
    with_engine(state, |engine| {
        engine
            .set_sampler_play_mode(deck_id, mode.to_dsp())
            .map_err(|e| e.to_string())
    })
}

fn new_draft_bank_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("draft-{nanos}")
}

fn is_draft_bank_id(state: &AppState, bank_id: &str) -> bool {
    state
        .draft_sampler_bank
        .as_ref()
        .is_some_and(|draft| draft.id == bank_id)
}

fn discard_draft_bank(state: &mut AppState) {
    state.draft_sampler_bank = None;
}

fn discard_draft_if_leaving(state: &mut AppState, next_bank_id: &str) {
    if state
        .draft_sampler_bank
        .as_ref()
        .is_some_and(|draft| draft.id != next_bank_id)
    {
        discard_draft_bank(state);
    }
}

/// Start an unsaved draft bank (same path as UI create). Caller loads engine / publishes.
fn start_draft_sampler_bank(
    state: &mut AppState,
    deck_id: usize,
    name: Option<String>,
    play_mode: Option<SamplerPlayModeSetting>,
) -> Result<SamplerBankInfo, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }
    discard_draft_bank(state);

    let persisted_count = state
        .library
        .lock()
        .unwrap()
        .list_sampler_banks()
        .map_err(|e| e.to_string())?
        .len();
    let name = name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Bank {}", persisted_count + 1));

    let draft = SamplerBankInfo {
        id: new_draft_bank_id(),
        name,
        play_mode,
        sort_index: i32::MAX,
    };
    state.draft_sampler_bank = Some(draft.clone());
    state.decks[deck_id].active_sampler_bank_id = Some(draft.id.clone());
    Ok(draft)
}

/// First remaining persisted bank, or a new draft if the library has none.
fn fallback_bank_id(state: &mut AppState, deck_id: usize) -> Result<String, String> {
    if let Some(draft) = &state.draft_sampler_bank {
        return Ok(draft.id.clone());
    }
    if let Some(bank) = state
        .library
        .lock()
        .unwrap()
        .list_sampler_banks()
        .map_err(|e| e.to_string())?
        .into_iter()
        .next()
    {
        return Ok(bank.id);
    }
    Ok(start_draft_sampler_bank(state, deck_id, None, None)?.id)
}

/// Persist an unsaved "+" draft on first real edit (rename / play mode / sample).
fn persist_draft_bank_if_needed(state: &mut AppState, bank_id: &str) -> Result<String, String> {
    let Some(draft) = state.draft_sampler_bank.clone() else {
        return Ok(bank_id.to_string());
    };
    if draft.id != bank_id {
        return Ok(bank_id.to_string());
    }

    let created = state
        .library
        .lock()
        .unwrap()
        .create_sampler_bank(
            &draft.name,
            draft.play_mode.map(|mode| mode.to_lib()),
        )
        .map_err(|e| e.to_string())?;

    state.draft_sampler_bank = None;
    for deck in &mut state.decks {
        if deck.active_sampler_bank_id.as_deref() == Some(bank_id) {
            deck.active_sampler_bank_id = Some(created.id.clone());
        }
    }
    for loaded in &mut state.loaded_sampler_bank_id {
        if loaded.as_deref() == Some(bank_id) {
            *loaded = Some(created.id.clone());
        }
    }
    Ok(created.id)
}

/// Ensure the shared sampler engine has this deck's active bank loaded.
pub(crate) fn ensure_deck_bank_loaded(state: &mut AppState, deck_id: usize) -> Result<(), String> {
    let bank_id = resolve_bank_id(state, None, deck_id)?;
    state.decks[deck_id].active_sampler_bank_id = Some(bank_id.clone());
    load_bank_into_engine(state, deck_id, &bank_id)
}

fn load_source_for_slot(
    state: &AppState,
    track_id: Option<&str>,
    path: Option<&str>,
) -> Result<(AudioSource, String, Option<i32>, Option<f64>), String> {
    if let Some(track_id) = track_id {
        let tid = TrackId::new(track_id.to_string());
        let source = state
            .library
            .lock()
            .unwrap()
            .get_track(&tid)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Track not found in library.".to_string())?;
        let path = source
            .file()
            .ok_or_else(|| "Only file tracks can be assigned to sampler pads.".to_string())?
            .path()
            .to_string_lossy()
            .into_owned();
        let duration_ms = source.metadata().duration_ms;
        let loudness = state
            .library
            .lock()
            .unwrap()
            .track_loudness_lufs(&tid)
            .map_err(|e| e.to_string())?;
        return Ok((source, path, duration_ms, loudness));
    }

    let path = path
        .ok_or_else(|| "Path is required.".to_string())?
        .to_string();
    let source = AudioSource::File(FileAudioSource::from_path(&path));
    let duration_ms = source.metadata().duration_ms;
    Ok((source, path, duration_ms, None))
}

pub(crate) fn ensure_sampler_ready(state: &mut AppState) -> Result<(), String> {
    discard_draft_bank(state);

    let banks = state
        .library
        .lock()
        .unwrap()
        .list_sampler_banks()
        .map_err(|e| e.to_string())?;
    let bank_id = if let Some(first) = banks.first() {
        first.id.clone()
    } else {
        start_draft_sampler_bank(state, 0, None, None)?.id
    };

    for deck_id in 0..NUM_DECKS {
        // Only persist deck defaults for real banks, not unsaved drafts.
        if state.deck_default_sampler_bank_id[deck_id].is_none() && state.draft_sampler_bank.is_none()
        {
            state.deck_default_sampler_bank_id[deck_id] = Some(bank_id.clone());
        }
        if state.decks[deck_id].active_sampler_bank_id.is_none() {
            state.decks[deck_id].active_sampler_bank_id = state.deck_default_sampler_bank_id
                [deck_id]
                .clone()
                .or_else(|| Some(bank_id.clone()));
        }
    }

    for deck_id in 0..NUM_DECKS {
        let active = resolve_bank_id(state, None, deck_id)?;
        state.decks[deck_id].active_sampler_bank_id = Some(active.clone());
        load_bank_into_engine(state, deck_id, &active)?;
    }
    Ok(())
}

pub(crate) fn select_bank_for_track_load(
    state: &mut AppState,
    deck_id: usize,
    track_id: Option<&str>,
) -> Result<(), String> {
    discard_draft_bank(state);
    let preferred = if let Some(tid) = track_id {
        state
            .library
            .lock()
            .unwrap()
            .get_track_last_sampler_bank_id(&TrackId::new(tid.to_string()))
            .map_err(|e| e.to_string())?
            .filter(|id| {
                state
                    .library
                    .lock()
                    .unwrap()
                    .get_sampler_bank(id)
                    .ok()
                    .flatten()
                    .is_some()
            })
    } else {
        None
    };

    let bank_id = preferred
        .or_else(|| state.deck_default_sampler_bank_id[deck_id].clone())
        .or_else(|| {
            state
                .library
                .lock()
                .unwrap()
                .list_sampler_banks()
                .ok()
                .and_then(|banks| banks.into_iter().next().map(|b| b.id))
        });

    let bank_id = match bank_id {
        Some(id) => id,
        None => start_draft_sampler_bank(state, deck_id, None, None)?.id,
    };

    state.decks[deck_id].active_sampler_bank_id = Some(bank_id.clone());
    load_bank_into_engine(state, deck_id, &bank_id)?;
    Ok(())
}

pub(crate) fn reapply_sampler_gains(state: &mut AppState) -> Result<(), String> {
    let enabled = state.volume_normalizer_enabled;
    let target = state.target_lufs;

    for deck_id in 0..NUM_DECKS {
        let bank_id = resolve_bank_id(state, None, deck_id)?;
        let slots = state
            .library
            .lock()
            .unwrap()
            .list_sampler_bank_slots(&bank_id)
            .map_err(|e| e.to_string())?;

        for record in slots {
            let slot = usize::from(record.slot_index);
            if slot >= SAMPLER_SLOT_COUNT {
                continue;
            }
            let loudness = record.track_id.as_ref().and_then(|tid| {
                state
                    .library
                    .lock()
                    .unwrap()
                    .track_loudness_lufs(&TrackId::new(tid.clone()))
                    .ok()
                    .flatten()
            });
            let gain = if enabled {
                match loudness {
                    Some(l) if l.is_finite() => (target - l as f32).clamp(-12.0, 12.0),
                    _ => 0.0,
                }
            } else {
                0.0
            };
            with_engine(state, |engine| {
                engine
                    .set_sampler_slot_auto_gain(deck_id, slot, gain)
                    .map_err(|e| e.to_string())
            })?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn list_sampler_banks(state: State<'_, SharedAppState>) -> Result<Vec<SamplerBankInfo>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let banks = state
        .library
        .lock()
        .unwrap()
        .list_sampler_banks()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(SamplerBankInfo::from)
        .collect();
    Ok(banks)
}

pub(crate) fn create_sampler_bank_inner(
    app: &AppHandle,
    state: &mut AppState,
    deck_id: usize,
    name: Option<String>,
    play_mode: Option<SamplerPlayModeSetting>,
) -> Result<SamplerBankInfo, String> {
    let draft = start_draft_sampler_bank(state, deck_id, name, play_mode)?;
    load_bank_into_engine(state, deck_id, &draft.id)?;
    publish_deck(app, state, deck_id);
    publish_status(app, state);
    Ok(draft)
}

pub(crate) fn update_sampler_bank_inner(
    app: &AppHandle,
    state: &mut AppState,
    bank_id: String,
    name: String,
    play_mode: Option<SamplerPlayModeSetting>,
) -> Result<SamplerStatus, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Bank name cannot be empty.".to_string());
    }

    let bank_id = if is_draft_bank_id(state, &bank_id) {
        if let Some(draft) = state.draft_sampler_bank.as_mut() {
            draft.name = name;
            draft.play_mode = play_mode;
        }
        persist_draft_bank_if_needed(state, &bank_id)?
    } else {
        state
            .library
            .lock()
            .unwrap()
            .update_sampler_bank(&bank_id, &name, play_mode.map(|m| m.to_lib()))
            .map_err(|e| e.to_string())?;
        bank_id
    };

    for deck_id in 0..NUM_DECKS {
        if state.decks[deck_id].active_sampler_bank_id.as_deref() == Some(bank_id.as_str())
            || state.loaded_sampler_bank_id[deck_id].as_deref() == Some(bank_id.as_str())
        {
            apply_effective_play_mode_for_bank(state, deck_id, &bank_id)?;
        }
    }
    publish_status(app, state);
    Ok(SamplerStatus::from_state(state))
}

pub(crate) fn delete_sampler_bank_inner(
    app: &AppHandle,
    state: &mut AppState,
    bank_id: String,
) -> Result<SamplerStatus, String> {
    if is_draft_bank_id(state, &bank_id) {
        discard_draft_bank(state);
        let fallback = fallback_bank_id(state, 0)?;
        for i in 0..NUM_DECKS {
            if state.decks[i].active_sampler_bank_id.as_deref() == Some(bank_id.as_str()) {
                state.decks[i].active_sampler_bank_id = Some(fallback.clone());
                load_bank_into_engine(state, i, &fallback)?;
            }
        }
        publish_status(app, state);
        return Ok(SamplerStatus::from_state(state));
    }

    state
        .library
        .lock()
        .unwrap()
        .delete_sampler_bank(&bank_id)
        .map_err(|e| e.to_string())?;

    let fallback = fallback_bank_id(state, 0)?;

    for i in 0..NUM_DECKS {
        if state.deck_default_sampler_bank_id[i].as_deref() == Some(bank_id.as_str()) {
            state.deck_default_sampler_bank_id[i] = if is_draft_bank_id(state, &fallback) {
                None
            } else {
                Some(fallback.clone())
            };
        }
        if state.decks[i].active_sampler_bank_id.as_deref() == Some(bank_id.as_str()) {
            state.decks[i].active_sampler_bank_id = Some(fallback.clone());
            load_bank_into_engine(state, i, &fallback)?;
        } else if state.loaded_sampler_bank_id[i].as_deref() == Some(bank_id.as_str()) {
            state.loaded_sampler_bank_id[i] = None;
        }
    }
    publish_status(app, state);
    Ok(SamplerStatus::from_state(state))
}

pub(crate) fn set_deck_sampler_bank_inner(
    app: &AppHandle,
    state: &mut AppState,
    deck_id: usize,
    bank_id: String,
) -> Result<SamplerStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }
    if !is_draft_bank_id(state, &bank_id)
        && state
            .library
            .lock()
            .unwrap()
            .get_sampler_bank(&bank_id)
            .map_err(|e| e.to_string())?
            .is_none()
    {
        return Err(format!("Sampler bank not found: {bank_id}"));
    }
    discard_draft_if_leaving(state, &bank_id);
    state.decks[deck_id].active_sampler_bank_id = Some(bank_id.clone());
    load_bank_into_engine(state, deck_id, &bank_id)?;
    publish_deck(app, state, deck_id);
    publish_status(app, state);
    Ok(SamplerStatus::from_state(state))
}

pub(crate) fn assign_sampler_slot_inner(
    app: &AppHandle,
    state: &mut AppState,
    slot: usize,
    path: String,
    bank_id: Option<String>,
    deck_id: usize,
) -> Result<SamplerStatus, String> {
    if slot >= SAMPLER_SLOT_COUNT {
        return Err(format!("Invalid sampler slot: {slot}"));
    }
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }
    let bank_id = resolve_bank_id(state, bank_id.as_deref(), deck_id)?;
    let bank_id = persist_draft_bank_if_needed(state, &bank_id)?;
    let (source, resolved_path, duration_ms, loudness) =
        load_source_for_slot(state, None, Some(&path))?;
    let label = slot_label(&source);

    state
        .library
        .lock()
        .unwrap()
        .assign_sampler_bank_slot(
            &bank_id,
            slot as u8,
            None,
            Some(resolved_path.clone()),
            Some(label.clone()),
        )
        .map_err(|e| e.to_string())?;

    if state.loaded_sampler_bank_id[deck_id].as_deref() == Some(bank_id.as_str()) {
        with_engine(state, |engine| {
            engine
                .assign_sampler_slot(deck_id, slot, source, label.clone(), loudness)
                .map_err(|e| e.to_string())
        })?;
        state.sampler_slots[deck_id][slot] = SamplerSlotInfo {
            label: Some(label),
            track_id: None,
            path: Some(resolved_path),
            duration_ms,
        };
    } else if state.decks[deck_id].active_sampler_bank_id.as_deref() == Some(bank_id.as_str()) {
        load_bank_into_engine(state, deck_id, &bank_id)?;
    }

    publish_status(app, state);
    Ok(SamplerStatus::from_state(state))
}

pub(crate) fn assign_sampler_slot_from_track_inner(
    app: &AppHandle,
    state: &mut AppState,
    slot: usize,
    track_id: String,
    bank_id: Option<String>,
    deck_id: usize,
) -> Result<SamplerStatus, String> {
    if slot >= SAMPLER_SLOT_COUNT {
        return Err(format!("Invalid sampler slot: {slot}"));
    }
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }
    let bank_id = resolve_bank_id(state, bank_id.as_deref(), deck_id)?;
    let bank_id = persist_draft_bank_if_needed(state, &bank_id)?;
    let (source, resolved_path, duration_ms, loudness) =
        load_source_for_slot(state, Some(&track_id), None)?;
    let label = slot_label(&source);

    state
        .library
        .lock()
        .unwrap()
        .assign_sampler_bank_slot(
            &bank_id,
            slot as u8,
            Some(track_id.clone()),
            Some(resolved_path.clone()),
            Some(label.clone()),
        )
        .map_err(|e| e.to_string())?;

    if state.loaded_sampler_bank_id[deck_id].as_deref() == Some(bank_id.as_str()) {
        with_engine(state, |engine| {
            engine
                .assign_sampler_slot(deck_id, slot, source, label.clone(), loudness)
                .map_err(|e| e.to_string())
        })?;
        state.sampler_slots[deck_id][slot] = SamplerSlotInfo {
            label: Some(label),
            track_id: Some(track_id),
            path: Some(resolved_path),
            duration_ms,
        };
    } else if state.decks[deck_id].active_sampler_bank_id.as_deref() == Some(bank_id.as_str()) {
        load_bank_into_engine(state, deck_id, &bank_id)?;
    }

    publish_status(app, state);
    Ok(SamplerStatus::from_state(state))
}

pub(crate) fn clear_sampler_slot_inner(
    app: &AppHandle,
    state: &mut AppState,
    slot: usize,
    bank_id: Option<String>,
    deck_id: usize,
) -> Result<SamplerStatus, String> {
    if slot >= SAMPLER_SLOT_COUNT {
        return Err(format!("Invalid sampler slot: {slot}"));
    }
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }
    let bank_id = resolve_bank_id(state, bank_id.as_deref(), deck_id)?;
    if is_draft_bank_id(state, &bank_id) {
        if state.loaded_sampler_bank_id[deck_id].as_deref() == Some(bank_id.as_str()) {
            with_engine(state, |engine| {
                engine
                    .clear_sampler_slot(deck_id, slot)
                    .map_err(|e| e.to_string())
            })?;
            state.sampler_slots[deck_id][slot] = SamplerSlotInfo::default();
        }
        publish_status(app, state);
        return Ok(SamplerStatus::from_state(state));
    }

    state
        .library
        .lock()
        .unwrap()
        .clear_sampler_bank_slot(&bank_id, slot as u8)
        .map_err(|e| e.to_string())?;

    if state.loaded_sampler_bank_id[deck_id].as_deref() == Some(bank_id.as_str()) {
        with_engine(state, |engine| {
            engine
                .clear_sampler_slot(deck_id, slot)
                .map_err(|e| e.to_string())
        })?;
        state.sampler_slots[deck_id][slot] = SamplerSlotInfo::default();
    }

    publish_status(app, state);
    Ok(SamplerStatus::from_state(state))
}
