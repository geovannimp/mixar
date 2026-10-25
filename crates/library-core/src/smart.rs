//! Smart-playlist rule documents: persisted, versioned membership rules.
//!
//! A smart playlist stores one [`SmartPlaylistRules`] JSON document instead of
//! `collection_entries` rows; membership is computed on read through
//! [`SmartPlaylistRules::matches`]. The document is versioned
//! ([`SMART_PLAYLIST_RULES_VERSION`]) and read forward-compatibly: unknown keys
//! are ignored, only a different `version` is rejected by
//! [`SmartPlaylistRules::validate`].
//!
//! Deck-relative fields (`deck_bpm_delta`, `deck_key`) compare a track against
//! what the playing decks are sounding. The host computes the sounding values
//! (tempo fader and Key Shift already applied) and passes them as
//! [`DeckReference`]s; this crate only does the percent and harmonic math.

use serde::{Deserialize, Serialize};

use crate::key_format::{canonical_key, harmonic_family};
use crate::types::ColorLabel;

/// Rule-document schema version written by this crate.
pub const SMART_PLAYLIST_RULES_VERSION: u32 = 1;

/// Root group counts as depth 1; three levels of nested groups fit inside it.
const MAX_DEPTH: usize = 4;
const MAX_CONDITIONS: usize = 64;
/// Cap on `in` value lists; also the size of the key table.
const MAX_VALUES: usize = 64;

/// How a group combines its rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmartMatch {
    /// Every rule must match.
    All,
    /// At least one rule must match.
    Any,
}

/// Track attribute a condition tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmartField {
    /// Track title.
    Title,
    /// Track artist.
    Artist,
    /// Track album.
    Album,
    /// Genre tag.
    Genre,
    /// Comment tag.
    Comment,
    /// Beats per minute from tags or analysis.
    Bpm,
    /// Star rating 1–5; `NULL` means unrated.
    Rating,
    /// Release year.
    Year,
    /// Track length in milliseconds.
    DurationMs,
    /// Measured integrated loudness (`track_analysis.loudness_lufs`).
    LoudnessLufs,
    /// Whole days since the track entered the library.
    AddedDaysAgo,
    /// Musical key.
    Key,
    /// Colour label.
    ColorLabel,
    /// Percent difference between the track and a deck's sounding BPM.
    DeckBpmDelta,
    /// Key relation between the track and a deck's sounding key.
    DeckKey,
}

impl SmartField {
    /// Wire name, used in JSON and validation messages.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Artist => "artist",
            Self::Album => "album",
            Self::Genre => "genre",
            Self::Comment => "comment",
            Self::Bpm => "bpm",
            Self::Rating => "rating",
            Self::Year => "year",
            Self::DurationMs => "duration_ms",
            Self::LoudnessLufs => "loudness_lufs",
            Self::AddedDaysAgo => "added_days_ago",
            Self::Key => "key",
            Self::ColorLabel => "color_label",
            Self::DeckBpmDelta => "deck_bpm_delta",
            Self::DeckKey => "deck_key",
        }
    }

    /// Deck-relative fields read a [`DeckReference`] instead of the track only.
    pub fn is_deck(self) -> bool {
        matches!(self, Self::DeckBpmDelta | Self::DeckKey)
    }

    fn kind(self) -> FieldKind {
        match self {
            Self::Title | Self::Artist | Self::Album => FieldKind::Text,
            Self::Genre | Self::Comment => FieldKind::TextSet,
            Self::Bpm
            | Self::Rating
            | Self::Year
            | Self::DurationMs
            | Self::LoudnessLufs
            | Self::AddedDaysAgo
            | Self::DeckBpmDelta => FieldKind::Numeric,
            Self::Key => FieldKind::Key,
            Self::ColorLabel => FieldKind::Color,
            Self::DeckKey => FieldKind::DeckKey,
        }
    }

    /// Operators the field accepts, excluding the universal `is_empty` /
    /// `is_not_empty` pair that every non-deck field also takes.
    fn allowed_ops(self) -> &'static [SmartOp] {
        match self {
            Self::Title | Self::Artist | Self::Album => &[SmartOp::Contains, SmartOp::Equals],
            Self::Genre | Self::Comment => &[SmartOp::Contains, SmartOp::Equals, SmartOp::In],
            Self::Bpm
            | Self::Rating
            | Self::Year
            | Self::DurationMs
            | Self::LoudnessLufs
            | Self::AddedDaysAgo
            | Self::DeckBpmDelta => &[SmartOp::Between],
            Self::Key | Self::ColorLabel => &[SmartOp::In],
            Self::DeckKey => &[SmartOp::Equals, SmartOp::HarmonicallyCompatible],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FieldKind {
    /// One value slot (`contains` / `equals`).
    Text,
    /// Free-form tag matched by substring; `in` takes a list.
    TextSet,
    /// `between` on `min`/`max`.
    Numeric,
    /// `in` on canonical key tokens.
    Key,
    /// `in` on canonical colour names.
    Color,
    /// `equals` / `harmonically_compatible` against a deck.
    DeckKey,
}

/// Comparison a condition applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmartOp {
    /// Inclusive numeric window; `min` and/or `max`.
    Between,
    /// Case-insensitive substring.
    Contains,
    /// Case-insensitive equality (after trimming).
    Equals,
    /// Membership in a value list.
    In,
    /// Value is missing or blank.
    IsEmpty,
    /// Value is present and non-blank.
    IsNotEmpty,
    /// The track key is in the deck key's Camelot-compatible family.
    HarmonicallyCompatible,
}

impl SmartOp {
    /// Wire name, used in JSON and validation messages.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Between => "between",
            Self::Contains => "contains",
            Self::Equals => "equals",
            Self::In => "in",
            Self::IsEmpty => "is_empty",
            Self::IsNotEmpty => "is_not_empty",
            Self::HarmonicallyCompatible => "harmonically_compatible",
        }
    }
}

/// One condition inside a [`SmartRuleGroup`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SmartCondition {
    /// Attribute under test.
    pub field: SmartField,
    /// Comparison applied to it.
    pub op: SmartOp,
    /// Inclusive lower bound (`between` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Inclusive upper bound (`between` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// Value list: one entry for `contains`/`equals`, 1..=64 for `in`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
    /// Deck-relative fields only: `None` reads any playing deck, `Some(id)` pins one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deck: Option<u16>,
}

/// Rules combined by [`SmartMatch`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SmartRuleGroup {
    /// How the rules combine.
    #[serde(rename = "match")]
    pub match_mode: SmartMatch,
    /// Conditions and nested groups, in document order.
    pub rules: Vec<SmartRule>,
}

impl Default for SmartRuleGroup {
    fn default() -> Self {
        Self {
            match_mode: SmartMatch::All,
            rules: Vec::new(),
        }
    }
}

/// A group entry: a condition or a nested group.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SmartRule {
    /// Leaf condition.
    Condition(SmartCondition),
    /// Nested group.
    Group(SmartRuleGroup),
}

/// Sort key for the computed membership list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmartSortField {
    /// Track title.
    Title,
    /// Track artist.
    Artist,
    /// Beats per minute.
    Bpm,
    /// Star rating.
    Rating,
    /// Release year.
    Year,
    /// Track length.
    DurationMs,
    /// Integrated loudness.
    LoudnessLufs,
    /// When the track entered the library: ascending is oldest first, so
    /// `ascending: false` is "recently added".
    AddedDaysAgo,
}

/// Order of the computed membership list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmartSort {
    /// Attribute to order by.
    pub field: SmartSortField,
    /// `true` ascending, `false` descending; missing values always sort last.
    pub ascending: bool,
}

/// Persisted rule document of a smart playlist.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SmartPlaylistRules {
    /// Schema version; see [`SMART_PLAYLIST_RULES_VERSION`].
    pub version: u32,
    /// Ordering; `None` sorts by title ascending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<SmartSort>,
    /// Rule tree. The root has no `kind` tag.
    pub root: SmartRuleGroup,
}

impl Default for SmartPlaylistRules {
    fn default() -> Self {
        Self {
            version: SMART_PLAYLIST_RULES_VERSION,
            sort: None,
            root: SmartRuleGroup::default(),
        }
    }
}

impl SmartPlaylistRules {
    /// Parse and validate a stored document.
    pub fn from_json(json: &str) -> Result<Self, String> {
        let rules: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        rules.validate()?;
        Ok(rules)
    }

    /// Canonical storage form (compact JSON, fixed key order).
    ///
    /// Only fails on a serialization bug: every field is a plain value.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| String::from("{}"))
    }

    /// Check the document against the field/operator matrix and struct limits.
    ///
    /// Messages are user-facing fragments; callers wrap them as
    /// `smart playlist rules: {msg}`.
    pub fn validate(&self) -> Result<(), String> {
        if self.version != SMART_PLAYLIST_RULES_VERSION {
            return Err(format!("unsupported rules version {}", self.version));
        }
        let mut conditions = 0usize;
        validate_group(&self.root, 1, &mut conditions)
    }

    /// True when any condition is deck-relative.
    pub fn uses_deck_reference(&self) -> bool {
        fn group_uses(group: &SmartRuleGroup) -> bool {
            group.rules.iter().any(|rule| match rule {
                SmartRule::Condition(condition) => condition.field.is_deck(),
                SmartRule::Group(nested) => group_uses(nested),
            })
        }
        group_uses(&self.root)
    }

    /// Whether `facts` belongs in the playlist.
    ///
    /// A group with no rules matches nothing (an empty document is an empty
    /// playlist, never the whole library). A missing value fails every
    /// operator except `is_empty`; `rating = NULL` is unrated, so rating
    /// conditions never match unrated tracks. Deck-relative conditions match
    /// when any qualifying deck satisfies them — qualifying decks are the
    /// playing ones when `deck` is unset, or exactly that deck otherwise.
    pub fn matches(&self, facts: &SmartTrackFacts, now_unix: i64, decks: &[DeckReference]) -> bool {
        group_matches(&self.root, facts, now_unix, decks)
    }

    /// Sort `facts` in place by the document's sort (title ascending by default).
    pub fn sort_facts(&self, facts: &mut [SmartTrackFacts]) {
        let sort = self.sort.unwrap_or(SmartSort {
            field: SmartSortField::Title,
            ascending: true,
        });
        facts.sort_by(|a, b| {
            present_rank(sort.field, a)
                .cmp(&present_rank(sort.field, b))
                .then_with(|| {
                    let order = compare_field(sort.field, a, b);
                    if sort.ascending {
                        order
                    } else {
                        order.reverse()
                    }
                })
                .then_with(|| a.id.cmp(&b.id))
        });
    }
}

/// One deck's currently sounding values, reported by the host.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeckReference {
    /// Zero-based deck index (deck A is 0).
    pub deck_id: u16,
    /// Whether the deck is sounding; only playing decks are read when a rule
    /// names no deck.
    pub playing: bool,
    /// Sounding BPM (tempo fader applied).
    pub bpm: Option<f64>,
    /// Sounding key (Key Shift / pitch applied).
    pub key: Option<String>,
}

/// Metadata and analysis values a condition is evaluated against.
///
/// Owned, and built by moving fields out of the stored track row, so
/// evaluation adds no copies.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SmartTrackFacts {
    /// Track id.
    pub id: String,
    /// Track title.
    pub title: Option<String>,
    /// Track artist.
    pub artist: Option<String>,
    /// Track album.
    pub album: Option<String>,
    /// Genre tag.
    pub genre: Option<String>,
    /// Comment tag.
    pub comment: Option<String>,
    /// Beats per minute.
    pub bpm: Option<f64>,
    /// Musical key.
    pub key: Option<String>,
    /// Star rating 1–5; `None` is unrated.
    pub rating: Option<u8>,
    /// Colour label.
    pub color_label: Option<ColorLabel>,
    /// Release year.
    pub year: Option<u16>,
    /// Track length.
    pub duration_ms: Option<i32>,
    /// Integrated loudness from analysis.
    pub loudness_lufs: Option<f64>,
    /// Unix seconds the track entered the library.
    pub added_at_unix: Option<i64>,
}

fn validate_group(
    group: &SmartRuleGroup,
    depth: usize,
    conditions: &mut usize,
) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err(format!("group nesting too deep (max {MAX_DEPTH})"));
    }
    for rule in &group.rules {
        match rule {
            SmartRule::Condition(condition) => {
                *conditions += 1;
                if *conditions > MAX_CONDITIONS {
                    return Err(format!("too many rules (max {MAX_CONDITIONS})"));
                }
                validate_condition(condition)?;
            }
            SmartRule::Group(nested) => {
                if nested.rules.is_empty() {
                    return Err("groups must contain at least one rule".into());
                }
                validate_group(nested, depth + 1, conditions)?;
            }
        }
    }
    Ok(())
}

fn validate_condition(condition: &SmartCondition) -> Result<(), String> {
    let field = condition.field.as_str();
    let op = condition.op.as_str();

    if condition.op == SmartOp::HarmonicallyCompatible && condition.field != SmartField::DeckKey {
        return Err("harmonically_compatible applies to deck_key only".into());
    }
    if condition.deck.is_some() && !condition.field.is_deck() {
        return Err("deck only applies to deck rules".into());
    }
    let op_allowed = condition.field.allowed_ops().contains(&condition.op)
        || (!condition.field.is_deck()
            && matches!(condition.op, SmartOp::IsEmpty | SmartOp::IsNotEmpty));
    if !op_allowed {
        return Err(format!("op {op} is not allowed on {field}"));
    }

    match condition.op {
        SmartOp::Between => validate_between(condition, field)?,
        SmartOp::Contains | SmartOp::Equals => {
            if condition.field.kind() != FieldKind::DeckKey {
                validate_single_value(condition, field)?;
            }
        }
        SmartOp::In => match condition.field.kind() {
            FieldKind::TextSet => validate_value_list(condition, field)?,
            FieldKind::Key => {
                if condition.values.is_empty() {
                    return Err(format!("values must not be empty for {op} on {field}"));
                }
                // The closed value sets still allow unbounded duplicates, so the
                // shared list cap applies here too.
                if condition.values.len() > MAX_VALUES {
                    return Err(format!(
                        "values must contain at most {MAX_VALUES} entries for {op} on {field}"
                    ));
                }
                for value in &condition.values {
                    if canonical_key(value).is_none() {
                        return Err(format!("unknown key {value} in key rule"));
                    }
                }
            }
            FieldKind::Color => {
                if condition.values.is_empty() {
                    return Err(format!("values must not be empty for {op} on {field}"));
                }
                if condition.values.len() > MAX_VALUES {
                    return Err(format!(
                        "values must contain at most {MAX_VALUES} entries for {op} on {field}"
                    ));
                }
                for value in &condition.values {
                    if value.trim().parse::<ColorLabel>().is_err() {
                        return Err(format!("unknown color {value} in colour rule"));
                    }
                }
            }
            _ => {}
        },
        SmartOp::IsEmpty | SmartOp::IsNotEmpty | SmartOp::HarmonicallyCompatible => {}
    }
    Ok(())
}

fn validate_between(condition: &SmartCondition, field: &str) -> Result<(), String> {
    if condition.min.is_none() && condition.max.is_none() {
        return Err(format!("between needs min or max on {field}"));
    }
    for bound in [condition.min, condition.max].into_iter().flatten() {
        if !bound.is_finite() {
            return Err(format!("bounds must be finite on {field}"));
        }
    }
    if let (Some(min), Some(max)) = (condition.min, condition.max) {
        if min > max {
            return Err(format!("min must not exceed max on {field}"));
        }
    }
    if condition.field == SmartField::Rating {
        for bound in [condition.min, condition.max].into_iter().flatten() {
            if !(1.0..=5.0).contains(&bound) {
                return Err("rating bounds must be within 1-5".into());
            }
        }
    }
    Ok(())
}

fn validate_single_value(condition: &SmartCondition, field: &str) -> Result<(), String> {
    let op = condition.op.as_str();
    if condition.values.len() != 1 {
        return Err(format!(
            "values must contain exactly one entry for {op} on {field}"
        ));
    }
    if condition.values[0].trim().is_empty() {
        return Err(format!("values must not be empty for {op} on {field}"));
    }
    Ok(())
}

fn validate_value_list(condition: &SmartCondition, field: &str) -> Result<(), String> {
    let op = condition.op.as_str();
    if condition.values.is_empty() {
        return Err(format!("values must not be empty for {op} on {field}"));
    }
    if condition.values.len() > MAX_VALUES {
        return Err(format!(
            "values must contain at most {MAX_VALUES} entries for {op} on {field}"
        ));
    }
    for value in &condition.values {
        if value.trim().is_empty() {
            return Err(format!("values must not be empty for {op} on {field}"));
        }
    }
    Ok(())
}

fn group_matches(
    group: &SmartRuleGroup,
    facts: &SmartTrackFacts,
    now_unix: i64,
    decks: &[DeckReference],
) -> bool {
    if group.rules.is_empty() {
        return false;
    }
    match group.match_mode {
        SmartMatch::All => group
            .rules
            .iter()
            .all(|rule| rule_matches(rule, facts, now_unix, decks)),
        SmartMatch::Any => group
            .rules
            .iter()
            .any(|rule| rule_matches(rule, facts, now_unix, decks)),
    }
}

fn rule_matches(
    rule: &SmartRule,
    facts: &SmartTrackFacts,
    now_unix: i64,
    decks: &[DeckReference],
) -> bool {
    match rule {
        SmartRule::Condition(condition) => condition_matches(condition, facts, now_unix, decks),
        SmartRule::Group(group) => group_matches(group, facts, now_unix, decks),
    }
}

fn condition_matches(
    condition: &SmartCondition,
    facts: &SmartTrackFacts,
    now_unix: i64,
    decks: &[DeckReference],
) -> bool {
    if condition.field.is_deck() {
        return deck_condition_matches(condition, facts, decks);
    }
    match condition.op {
        SmartOp::IsEmpty => field_is_empty(condition.field, facts, now_unix),
        SmartOp::IsNotEmpty => !field_is_empty(condition.field, facts, now_unix),
        SmartOp::Between => numeric_value(condition.field, facts, now_unix)
            .is_some_and(|value| between_includes(condition, value)),
        SmartOp::Contains => {
            let Some(needle) = condition.values.first() else {
                return false;
            };
            text_value(condition.field, facts)
                .is_some_and(|value| contains_ignoring_case(value, needle))
        }
        SmartOp::Equals => match condition.field.kind() {
            FieldKind::Key => {
                let Some(needle) = condition.values.first().and_then(|v| canonical_key(v)) else {
                    return false;
                };
                facts
                    .key
                    .as_deref()
                    .and_then(canonical_key)
                    .is_some_and(|key| key == needle)
            }
            FieldKind::Color => {
                let Some(needle) = condition
                    .values
                    .first()
                    .and_then(|v| v.trim().parse::<ColorLabel>().ok())
                else {
                    return false;
                };
                facts.color_label == Some(needle)
            }
            _ => {
                let Some(needle) = condition.values.first() else {
                    return false;
                };
                text_value(condition.field, facts)
                    .is_some_and(|value| value.eq_ignore_ascii_case(needle.trim()))
            }
        },
        SmartOp::In => match condition.field.kind() {
            FieldKind::TextSet => text_value(condition.field, facts).is_some_and(|value| {
                condition
                    .values
                    .iter()
                    .any(|needle| contains_ignoring_case(value, needle))
            }),
            FieldKind::Key => facts
                .key
                .as_deref()
                .and_then(canonical_key)
                .is_some_and(|key| {
                    condition
                        .values
                        .iter()
                        .filter_map(|value| canonical_key(value))
                        .any(|listed| listed == key)
                }),
            FieldKind::Color => facts.color_label.is_some_and(|label| {
                condition
                    .values
                    .iter()
                    .filter_map(|value| value.trim().parse::<ColorLabel>().ok())
                    .any(|listed| listed == label)
            }),
            _ => false,
        },
        SmartOp::HarmonicallyCompatible => false,
    }
}

fn deck_condition_matches(
    condition: &SmartCondition,
    facts: &SmartTrackFacts,
    decks: &[DeckReference],
) -> bool {
    let track_key = facts.key.as_deref().and_then(canonical_key);
    decks
        .iter()
        .filter(|deck| condition.deck.map_or(deck.playing, |id| deck.deck_id == id))
        .any(|deck| match condition.field {
            SmartField::DeckBpmDelta => {
                let (Some(deck_bpm), Some(bpm)) = (deck.bpm, facts.bpm) else {
                    return false;
                };
                if !deck_bpm.is_finite() || deck_bpm <= 0.0 {
                    return false;
                }
                between_includes(condition, 100.0 * (bpm / deck_bpm - 1.0))
            }
            SmartField::DeckKey => {
                let Some(deck_key) = deck
                    .key
                    .as_deref()
                    .map(str::trim)
                    .filter(|key| !key.is_empty())
                else {
                    return false;
                };
                match condition.op {
                    SmartOp::Equals => {
                        let deck_key = canonical_key(deck_key);
                        track_key.is_some() && track_key == deck_key
                    }
                    SmartOp::HarmonicallyCompatible => track_key.as_deref().is_some_and(|track| {
                        harmonic_family(deck_key)
                            .is_some_and(|family| family.iter().any(|key| key == track))
                    }),
                    _ => false,
                }
            }
            _ => false,
        })
}

fn field_is_empty(field: SmartField, facts: &SmartTrackFacts, now_unix: i64) -> bool {
    match field.kind() {
        FieldKind::Text | FieldKind::TextSet => text_value(field, facts).is_none(),
        FieldKind::Numeric => numeric_value(field, facts, now_unix).is_none(),
        FieldKind::Key => facts.key.as_deref().is_none_or(|key| key.trim().is_empty()),
        FieldKind::Color => facts.color_label.is_none(),
        FieldKind::DeckKey => true,
    }
}

/// Non-blank text of a text field.
fn text_value(field: SmartField, facts: &SmartTrackFacts) -> Option<&str> {
    let raw = match field {
        SmartField::Title => facts.title.as_deref(),
        SmartField::Artist => facts.artist.as_deref(),
        SmartField::Album => facts.album.as_deref(),
        SmartField::Genre => facts.genre.as_deref(),
        SmartField::Comment => facts.comment.as_deref(),
        _ => None,
    }?;
    let trimmed = raw.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn numeric_value(field: SmartField, facts: &SmartTrackFacts, now_unix: i64) -> Option<f64> {
    match field {
        SmartField::Bpm => facts.bpm,
        SmartField::Rating => facts.rating.map(f64::from),
        SmartField::Year => facts.year.map(f64::from),
        SmartField::DurationMs => facts.duration_ms.map(f64::from),
        SmartField::LoudnessLufs => facts.loudness_lufs,
        SmartField::AddedDaysAgo => facts
            .added_at_unix
            .map(|added| (now_unix - added).div_euclid(86_400) as f64),
        _ => None,
    }
}

fn between_includes(condition: &SmartCondition, value: f64) -> bool {
    condition.min.is_none_or(|min| value >= min) && condition.max.is_none_or(|max| value <= max)
}

fn contains_ignoring_case(haystack: &str, needle: &str) -> bool {
    let needle = needle.trim();
    if needle.is_empty() {
        return false;
    }
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

/// `0` when the field has a value for `facts`, `1` when it does not, so missing
/// values sort last in either direction.
fn present_rank(field: SmartSortField, facts: &SmartTrackFacts) -> u8 {
    u8::from(sort_text(field, facts).is_none() && sort_number(field, facts).is_none())
}

fn sort_text(field: SmartSortField, facts: &SmartTrackFacts) -> Option<&str> {
    match field {
        SmartSortField::Title => facts.title.as_deref(),
        SmartSortField::Artist => facts.artist.as_deref(),
        _ => None,
    }
    .map(str::trim)
    .filter(|value| !value.is_empty())
}

fn sort_number(field: SmartSortField, facts: &SmartTrackFacts) -> Option<f64> {
    match field {
        SmartSortField::Bpm => facts.bpm,
        SmartSortField::Rating => facts.rating.map(f64::from),
        SmartSortField::Year => facts.year.map(f64::from),
        SmartSortField::DurationMs => facts.duration_ms.map(f64::from),
        SmartSortField::LoudnessLufs => facts.loudness_lufs,
        SmartSortField::AddedDaysAgo => facts.added_at_unix.map(|added| added as f64),
        SmartSortField::Title | SmartSortField::Artist => None,
    }
}

fn compare_field(
    field: SmartSortField,
    a: &SmartTrackFacts,
    b: &SmartTrackFacts,
) -> std::cmp::Ordering {
    match (sort_text(field, a), sort_text(field, b)) {
        (Some(a), Some(b)) => a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()),
        _ => sort_number(field, a)
            .partial_cmp(&sort_number(field, b))
            .unwrap_or(std::cmp::Ordering::Equal),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/smart_playlist_rules_v1.json");

    fn condition(field: SmartField, op: SmartOp) -> SmartCondition {
        SmartCondition {
            field,
            op,
            min: None,
            max: None,
            values: Vec::new(),
            deck: None,
        }
    }

    fn rules(rules: Vec<SmartRule>) -> SmartPlaylistRules {
        SmartPlaylistRules {
            root: SmartRuleGroup {
                match_mode: SmartMatch::All,
                rules,
            },
            ..Default::default()
        }
    }

    fn facts() -> SmartTrackFacts {
        SmartTrackFacts {
            id: "t1".into(),
            title: Some("Track One".into()),
            artist: Some("Artist".into()),
            genre: Some("House".into()),
            bpm: Some(128.0),
            key: Some("Am".into()),
            rating: Some(4),
            added_at_unix: Some(1_700_000_000),
            ..Default::default()
        }
    }

    #[test]
    fn rules_json_round_trip() {
        let parsed = SmartPlaylistRules::from_json(FIXTURE).expect("fixture parses");
        assert_eq!(parsed.to_json(), FIXTURE.trim_end());
        assert!(parsed.uses_deck_reference());
    }

    #[test]
    fn empty_document_round_trips_and_validates() {
        let rules = SmartPlaylistRules::default();
        rules.validate().expect("empty root is valid");
        let parsed = SmartPlaylistRules::from_json(&rules.to_json()).expect("parses");
        assert_eq!(parsed, rules);
        assert!(!parsed.uses_deck_reference());
    }

    #[test]
    fn between_is_inclusive_and_skips_missing() {
        let mut between = condition(SmartField::Bpm, SmartOp::Between);
        between.min = Some(126.0);
        between.max = Some(130.0);
        let rules = rules(vec![SmartRule::Condition(between)]);

        assert!(rules.matches(&facts(), 0, &[]));
        let mut quiet = facts();
        quiet.bpm = Some(126.0);
        assert!(rules.matches(&quiet, 0, &[]));
        quiet.bpm = Some(130.0);
        assert!(rules.matches(&quiet, 0, &[]));
        quiet.bpm = Some(130.01);
        assert!(!rules.matches(&quiet, 0, &[]));
        quiet.bpm = None;
        assert!(!rules.matches(&quiet, 0, &[]));
    }

    #[test]
    fn contains_and_equals_are_case_insensitive() {
        let mut contains = condition(SmartField::Title, SmartOp::Contains);
        contains.values = vec!["rack on".into()];
        assert!(rules(vec![SmartRule::Condition(contains)]).matches(&facts(), 0, &[]));

        let mut equals = condition(SmartField::Artist, SmartOp::Equals);
        equals.values = vec!["  artist ".into()];
        assert!(rules(vec![SmartRule::Condition(equals)]).matches(&facts(), 0, &[]));

        let mut other = condition(SmartField::Artist, SmartOp::Equals);
        other.values = vec!["artists".into()];
        assert!(!rules(vec![SmartRule::Condition(other)]).matches(&facts(), 0, &[]));
    }

    #[test]
    fn in_on_genre_matches_substring() {
        let mut genre = condition(SmartField::Genre, SmartOp::In);
        genre.values = vec!["techno".into(), "hou".into()];
        assert!(rules(vec![SmartRule::Condition(genre)]).matches(&facts(), 0, &[]));
    }

    #[test]
    fn in_on_key_normalizes_camelot_and_flats() {
        let mut camelot = condition(SmartField::Key, SmartOp::In);
        camelot.values = vec!["8A".into()];
        assert!(rules(vec![SmartRule::Condition(camelot)]).matches(&facts(), 0, &[]));

        let mut flat = facts();
        flat.key = Some("A#".into());
        let mut rule = condition(SmartField::Key, SmartOp::In);
        rule.values = vec!["Bb".into()];
        assert!(rules(vec![SmartRule::Condition(rule)]).matches(&flat, 0, &[]));
    }

    #[test]
    fn in_on_color_label() {
        let mut blue = facts();
        blue.color_label = Some(ColorLabel::Blue);
        let mut rule = condition(SmartField::ColorLabel, SmartOp::In);
        rule.values = vec!["blue".into(), "Aqua".into()];
        assert!(rules(vec![SmartRule::Condition(rule)]).matches(&blue, 0, &[]));

        let mut rule = condition(SmartField::ColorLabel, SmartOp::In);
        rule.values = vec!["pink".into()];
        assert!(!rules(vec![SmartRule::Condition(rule)]).matches(&blue, 0, &[]));
    }

    #[test]
    fn is_empty_selects_unrated() {
        let mut unrated = facts();
        unrated.rating = None;
        let rule = condition(SmartField::Rating, SmartOp::IsEmpty);
        let unrated_rules = rules(vec![SmartRule::Condition(rule)]);
        assert!(unrated_rules.matches(&unrated, 0, &[]));
        assert!(!unrated_rules.matches(&facts(), 0, &[]));

        // A rating window never matches an unrated track.
        let mut between = condition(SmartField::Rating, SmartOp::Between);
        between.min = Some(1.0);
        assert!(!rules(vec![SmartRule::Condition(between)]).matches(&unrated, 0, &[]));
    }

    #[test]
    fn empty_root_matches_nothing() {
        assert!(!SmartPlaylistRules::default().matches(&facts(), 0, &[]));
        assert!(
            !rules(vec![SmartRule::Group(SmartRuleGroup::default())]).matches(&facts(), 0, &[])
        );
    }

    #[test]
    fn nested_empty_group_fails_validation() {
        let error = rules(vec![SmartRule::Group(SmartRuleGroup::default())])
            .validate()
            .expect_err("nested empty group is invalid");
        assert_eq!(error, "groups must contain at least one rule");
    }

    #[test]
    fn any_group_ors_conditions() {
        let mut low = condition(SmartField::Bpm, SmartOp::Between);
        low.max = Some(100.0);
        let mut high = condition(SmartField::Bpm, SmartOp::Between);
        high.min = Some(120.0);
        let group = SmartRuleGroup {
            match_mode: SmartMatch::Any,
            rules: vec![
                SmartRule::Condition(low.clone()),
                SmartRule::Condition(high.clone()),
            ],
        };
        let any = rules(vec![SmartRule::Group(group.clone())]);
        assert!(any.matches(&facts(), 0, &[]));

        let all = rules(vec![SmartRule::Group(SmartRuleGroup {
            match_mode: SmartMatch::All,
            rules: group.rules,
        })]);
        assert!(!all.matches(&facts(), 0, &[]));
    }

    #[test]
    fn rating_bounds_outside_1_5_rejected() {
        let mut between = condition(SmartField::Rating, SmartOp::Between);
        between.max = Some(6.0);
        assert_eq!(
            rules(vec![SmartRule::Condition(between)])
                .validate()
                .expect_err("bounds"),
            "rating bounds must be within 1-5"
        );
    }

    #[test]
    fn sort_nulls_last_ties_by_id() {
        let bpm_120 = SmartTrackFacts {
            id: "b".into(),
            title: Some("Same".into()),
            bpm: Some(120.0),
            ..Default::default()
        };
        let no_bpm = SmartTrackFacts {
            id: "a".into(),
            title: Some("same".into()),
            bpm: None,
            ..Default::default()
        };
        let bpm_140 = SmartTrackFacts {
            id: "c".into(),
            title: Some("Alpha".into()),
            bpm: Some(140.0),
            ..Default::default()
        };
        let mut list = vec![bpm_120.clone(), no_bpm.clone(), bpm_140.clone()];

        // Title ascending, case-insensitive; equal titles fall back to track id.
        let sort = SmartPlaylistRules {
            sort: Some(SmartSort {
                field: SmartSortField::Title,
                ascending: true,
            }),
            ..Default::default()
        };
        sort.sort_facts(&mut list);
        let ids: Vec<&str> = list.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["c", "a", "b"]);

        // Descending keeps the missing BPM last.
        let sort = SmartPlaylistRules {
            sort: Some(SmartSort {
                field: SmartSortField::Bpm,
                ascending: false,
            }),
            ..Default::default()
        };
        sort.sort_facts(&mut list);
        let ids: Vec<&str> = list.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["c", "b", "a"]);
    }

    #[test]
    fn version_mismatch_rejected() {
        let rules = SmartPlaylistRules {
            version: 99,
            ..Default::default()
        };
        assert_eq!(
            rules.validate().expect_err("version"),
            "unsupported rules version 99"
        );
    }

    #[test]
    fn deck_bpm_delta_is_percent() {
        let deck = DeckReference {
            deck_id: 0,
            playing: true,
            bpm: Some(128.0),
            key: None,
        };
        let mut window = condition(SmartField::DeckBpmDelta, SmartOp::Between);
        window.min = Some(-6.0);
        window.max = Some(6.0);
        let rules = rules(vec![SmartRule::Condition(window)]);

        let mut track = facts();
        assert!(
            rules.matches(&track, 0, std::slice::from_ref(&deck)),
            "same BPM is 0 %"
        );
        track.bpm = Some(135.0);
        assert!(
            rules.matches(&track, 0, std::slice::from_ref(&deck)),
            "+5.47 %"
        );
        track.bpm = Some(121.0);
        assert!(
            rules.matches(&track, 0, std::slice::from_ref(&deck)),
            "-5.47 %"
        );
        track.bpm = Some(136.0);
        assert!(
            !rules.matches(&track, 0, std::slice::from_ref(&deck)),
            "+6.25 %"
        );
        track.bpm = Some(120.0);
        assert!(
            !rules.matches(&track, 0, std::slice::from_ref(&deck)),
            "-6.25 %"
        );

        // Percent, not absolute BPM: a 10 BPM gap from 100 is 10 %.
        let slow = DeckReference {
            bpm: Some(100.0),
            ..deck
        };
        track.bpm = Some(90.0);
        assert!(!rules.matches(&track, 0, std::slice::from_ref(&slow)));
    }

    #[test]
    fn deck_bpm_delta_exact_percent() {
        let deck = DeckReference {
            deck_id: 0,
            playing: true,
            bpm: Some(128.0),
            key: None,
        };
        let mut exact = condition(SmartField::DeckBpmDelta, SmartOp::Between);
        exact.min = Some(6.25);
        exact.max = Some(6.25);
        let rules = rules(vec![SmartRule::Condition(exact)]);
        let mut track = facts();
        track.bpm = Some(136.0);
        assert!(rules.matches(&track, 0, std::slice::from_ref(&deck)));
    }

    #[test]
    fn deck_key_harmonic_matches_family() {
        let deck = DeckReference {
            deck_id: 0,
            playing: true,
            bpm: None,
            key: Some("8A".into()),
        };
        let rules = rules(vec![SmartRule::Condition(condition(
            SmartField::DeckKey,
            SmartOp::HarmonicallyCompatible,
        ))]);
        for key in ["Am", "Em", "Dm", "C"] {
            let mut track = facts();
            track.key = Some(key.into());
            assert!(
                rules.matches(&track, 0, std::slice::from_ref(&deck)),
                "{key}"
            );
        }
        let mut track = facts();
        track.key = Some("F#m".into());
        assert!(!rules.matches(&track, 0, std::slice::from_ref(&deck)));
    }

    #[test]
    fn deck_key_equals_normalizes_tokens() {
        let deck = DeckReference {
            deck_id: 0,
            playing: true,
            bpm: None,
            key: Some("Bbm".into()),
        };
        let rules = rules(vec![SmartRule::Condition(condition(
            SmartField::DeckKey,
            SmartOp::Equals,
        ))]);
        let mut track = facts();
        track.key = Some("A#m".into());
        assert!(rules.matches(&track, 0, std::slice::from_ref(&deck)));
    }

    #[test]
    fn deck_rule_reads_any_playing_deck() {
        let rules = rules(vec![SmartRule::Condition(condition(
            SmartField::DeckKey,
            SmartOp::HarmonicallyCompatible,
        ))]);
        let paused = DeckReference {
            deck_id: 0,
            playing: false,
            bpm: Some(128.0),
            key: Some("Am".into()),
        };
        assert!(!rules.matches(&facts(), 0, std::slice::from_ref(&paused)));

        let playing = DeckReference {
            deck_id: 1,
            playing: true,
            ..paused
        };
        assert!(rules.matches(&facts(), 0, std::slice::from_ref(&playing)));
    }

    #[test]
    fn deck_rule_pinned_deck_ignores_paused() {
        let mut pinned = condition(SmartField::DeckBpmDelta, SmartOp::Between);
        pinned.min = Some(-1.0);
        pinned.max = Some(1.0);
        pinned.deck = Some(1);
        let rules = rules(vec![SmartRule::Condition(pinned)]);

        let paused = DeckReference {
            deck_id: 1,
            playing: false,
            bpm: Some(128.0),
            key: None,
        };
        assert!(rules.matches(&facts(), 0, std::slice::from_ref(&paused)));

        let other = DeckReference {
            deck_id: 0,
            playing: true,
            bpm: Some(128.0),
            key: None,
        };
        assert!(!rules.matches(&facts(), 0, std::slice::from_ref(&other)));
    }

    #[test]
    fn deck_rule_without_reference_matches_nothing() {
        let mut windowed = condition(SmartField::DeckBpmDelta, SmartOp::Between);
        windowed.min = Some(-100.0);
        windowed.max = Some(100.0);
        let windowed = rules(vec![SmartRule::Condition(windowed)]);
        assert!(!windowed.matches(&facts(), 0, &[]));

        let playing_without_bpm = DeckReference {
            deck_id: 0,
            playing: true,
            bpm: None,
            key: None,
        };
        assert!(!windowed.matches(&facts(), 0, std::slice::from_ref(&playing_without_bpm)));
    }

    #[test]
    fn deck_selector_rejected_on_track_fields() {
        let mut rule = condition(SmartField::Bpm, SmartOp::Between);
        rule.min = Some(120.0);
        rule.deck = Some(0);
        assert_eq!(
            rules(vec![SmartRule::Condition(rule)])
                .validate()
                .expect_err("deck selector"),
            "deck only applies to deck rules"
        );
    }

    #[test]
    fn harmonically_compatible_rejected_outside_deck_key() {
        let rule = condition(SmartField::Key, SmartOp::HarmonicallyCompatible);
        assert_eq!(
            rules(vec![SmartRule::Condition(rule)])
                .validate()
                .expect_err("harmonic op"),
            "harmonically_compatible applies to deck_key only"
        );
    }

    #[test]
    fn op_matrix_rejects_wrong_operator() {
        let mut rule = condition(SmartField::Title, SmartOp::Between);
        rule.min = Some(1.0);
        assert_eq!(
            rules(vec![SmartRule::Condition(rule)])
                .validate()
                .expect_err("op"),
            "op between is not allowed on title"
        );

        let mut key_equal = condition(SmartField::Key, SmartOp::Equals);
        key_equal.values = vec!["Am".into()];
        assert_eq!(
            rules(vec![SmartRule::Condition(key_equal)])
                .validate()
                .expect_err("op"),
            "op equals is not allowed on key"
        );
    }

    #[test]
    fn value_slot_errors() {
        let mut none = condition(SmartField::Title, SmartOp::Contains);
        assert_eq!(
            rules(vec![SmartRule::Condition(none.clone())])
                .validate()
                .expect_err("needs a value"),
            "values must contain exactly one entry for contains on title"
        );
        none.values = vec!["  ".into()];
        assert_eq!(
            rules(vec![SmartRule::Condition(none)])
                .validate()
                .expect_err("blank value"),
            "values must not be empty for contains on title"
        );

        let mut unknown_key = condition(SmartField::Key, SmartOp::In);
        unknown_key.values = vec!["H".into()];
        assert_eq!(
            rules(vec![SmartRule::Condition(unknown_key)])
                .validate()
                .expect_err("unknown key"),
            "unknown key H in key rule"
        );

        let mut unknown_color = condition(SmartField::ColorLabel, SmartOp::In);
        unknown_color.values = vec!["mauve".into()];
        assert_eq!(
            rules(vec![SmartRule::Condition(unknown_color)])
                .validate()
                .expect_err("unknown colour"),
            "unknown color mauve in colour rule"
        );

        let between = condition(SmartField::Bpm, SmartOp::Between);
        assert_eq!(
            rules(vec![SmartRule::Condition(between)])
                .validate()
                .expect_err("needs a bound"),
            "between needs min or max on bpm"
        );

        let mut inverted = condition(SmartField::Bpm, SmartOp::Between);
        inverted.min = Some(130.0);
        inverted.max = Some(120.0);
        assert_eq!(
            rules(vec![SmartRule::Condition(inverted)])
                .validate()
                .expect_err("inverted"),
            "min must not exceed max on bpm"
        );
    }

    #[test]
    fn nesting_depth_is_capped() {
        fn nest(depth: usize) -> SmartRuleGroup {
            let mut group = SmartRuleGroup {
                match_mode: SmartMatch::All,
                rules: vec![SmartRule::Condition(condition(
                    SmartField::Title,
                    SmartOp::IsNotEmpty,
                ))],
            };
            for _ in 1..depth {
                group = SmartRuleGroup {
                    match_mode: SmartMatch::All,
                    rules: vec![SmartRule::Group(group)],
                };
            }
            group
        }
        rules(vec![SmartRule::Group(nest(3))])
            .validate()
            .expect("root plus three nested groups is depth 4");
        assert_eq!(
            rules(vec![SmartRule::Group(nest(4))])
                .validate()
                .expect_err("root plus four nested groups is depth 5"),
            "group nesting too deep (max 4)"
        );
    }

    #[test]
    fn rule_count_is_capped() {
        let conditions: Vec<SmartRule> = (0..65)
            .map(|_| SmartRule::Condition(condition(SmartField::Title, SmartOp::IsEmpty)))
            .collect();
        assert_eq!(
            rules(conditions).validate().expect_err("too many"),
            "too many rules (max 64)"
        );
    }

    #[test]
    fn added_days_ago_uses_whole_days() {
        let mut rule = condition(SmartField::AddedDaysAgo, SmartOp::Between);
        rule.min = Some(0.0);
        rule.max = Some(30.0);
        let rules = rules(vec![SmartRule::Condition(rule)]);
        let now = 1_700_000_000 + 29 * 86_400 + 100;
        assert!(rules.matches(&facts(), now, &[]));
        let now = 1_700_000_000 + 31 * 86_400;
        assert!(!rules.matches(&facts(), now, &[]));

        let mut missing = facts();
        missing.added_at_unix = None;
        assert!(!rules.matches(&missing, now, &[]));
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let json = r#"{"version":1,"future":true,"root":{"match":"all","rules":[
            {"kind":"condition","field":"bpm","op":"between","min":1.0,"future":2}]}}"#;
        let parsed = SmartPlaylistRules::from_json(json).expect("forward compatible");
        assert_eq!(parsed.root.rules.len(), 1);
    }
}
