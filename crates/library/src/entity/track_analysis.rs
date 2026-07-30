use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "track_analysis")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub track_id: String,
    pub backend: String,
    pub backend_version: String,
    pub analyzed_at: String,
    pub bpm: Option<f64>,
    pub bpm_confidence: Option<f64>,
    #[sea_orm(column_name = "key")]
    pub key: Option<String>,
    pub key_confidence: Option<f64>,
    pub key_clarity: Option<f64>,
    pub grid_stability: Option<f64>,
    pub sample_rate: i32,
    pub duration_analyzed_ms: i32,
    pub loudness_lufs: Option<f64>,
    pub beat_grid_json: Option<String>,
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
