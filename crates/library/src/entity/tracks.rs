#![allow(clippy::wildcard_imports)] // sea_orm entity prelude

use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tracks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub source_type: String,
    #[sea_orm(indexed)]
    pub source_ref: String,
    pub provider: Option<String>,
    #[sea_orm(indexed)]
    pub title: Option<String>,
    #[sea_orm(indexed)]
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub bpm: Option<f64>,
    pub bpm_source: Option<String>,
    #[sea_orm(column_name = "key")]
    pub key: Option<String>,
    pub key_source: Option<String>,
    pub loudness_lufs: Option<f64>,
    pub analysis_completed_at: Option<String>,
    pub duration_ms: Option<i32>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub bitrate_kbps: Option<i32>,
    pub replaygain_track_gain_db: Option<f64>,
    pub isrc: Option<String>,
    /// Star rating 1–5; `NULL` means unrated.
    pub rating: Option<i32>,
    pub comment: Option<String>,
    pub year: Option<i32>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    /// Canonical [`library_core::ColorLabel`] name (`"pink"` … `"purple"`).
    pub color_label: Option<String>,
    /// Exact 24-bit `0xRRGGBB` track colour. Additive: it never replaces the
    /// label the editor and exports still read. `NULL` means no raw colour.
    pub color_rgb: Option<i32>,
    /// Set once the editor writes the user-owned metadata columns, making the
    /// row authoritative over stale file tags on later re-analysis.
    pub metadata_edited: Option<bool>,
    /// Unix-seconds string of the successful Serato / Mixed In Key adoption.
    ///
    /// `NULL` means the file's vendor metadata has never been adopted, so a
    /// later lazy touch may still read it. Deleted and recreated tracks get a
    /// fresh adoption; an mtime/content change on a surviving row does not
    /// reset the marker.
    pub serato_imported_at: Option<String>,
    /// Bit mask of the Serato kinds adopted by [`serato_imported_at`].
    ///
    /// `NULL` means "never adopted". A successful adoption always stores an
    /// explicit value, including `0` for a readable file that carries no Serato
    /// frames, so later reloads and rescans cannot resurrect cleared state.
    pub serato_adopted_kinds: Option<i32>,
    pub last_sampler_bank_id: Option<String>,
    pub added_at: String,
    pub updated_at: String,
    #[sea_orm(
        belongs_to,
        from = "last_sampler_bank_id",
        to = "id",
        on_delete = "SetNull",
        on_update = "Cascade"
    )]
    pub last_sampler_bank: HasOne<super::sampler_bank::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
