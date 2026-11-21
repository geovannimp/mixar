#![allow(clippy::wildcard_imports)] // sea_orm entity prelude

use sea_orm::entity::prelude::*;

/// One constant-tempo run inside a canonical beat grid.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "track_beat_segment")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(indexed)]
    pub track_id: String,
    pub segment_index: i32,
    pub start_time_ms: i64,
    pub bpm: f64,
    pub beats_per_bar: i32,
    pub beat_unit: i32,
    pub start_beat_index: i64,
    pub beats_to_next: Option<i64>,
    pub downbeat_offset_beats: i32,
    #[sea_orm(
        belongs_to,
        from = "track_id",
        to = "track_id",
        on_delete = "Cascade",
        on_update = "Cascade"
    )]
    pub grid: HasOne<super::track_beat_grid::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
