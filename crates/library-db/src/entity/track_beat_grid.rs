#![allow(clippy::wildcard_imports)] // sea_orm entity prelude

use sea_orm::entity::prelude::*;

/// One track's canonical beat grid header.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "track_beat_grid")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub track_id: String,
    /// `analysis`, `manual`, or `import`.
    pub source: String,
    /// End of analyzed coverage; `NULL` when the grid covers the whole track.
    pub covered_to_ms: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    #[sea_orm(
        belongs_to,
        from = "track_id",
        to = "id",
        on_delete = "Cascade",
        on_update = "Cascade"
    )]
    pub track: HasOne<super::tracks::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
