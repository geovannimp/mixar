use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "track_stem")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub track_id: String,
    pub backend: String,
    /// Fingerprint of the source PCM used to produce these stems.
    #[sea_orm(default_value = "")]
    pub source_fingerprint: String,
    pub sample_rate: i32,
    pub vocals_path: String,
    pub drums_path: String,
    pub bass_path: String,
    pub other_path: String,
    pub generated_at: String,
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
