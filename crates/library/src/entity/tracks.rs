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
    #[sea_orm(column_name = "key")]
    pub key: Option<String>,
    pub duration_secs: Option<f64>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub bitrate_kbps: Option<i32>,
    pub replaygain_track_gain_db: Option<f64>,
    pub added_at: String,
    pub updated_at: String,
}

impl ActiveModelBehavior for ActiveModel {}
