use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sampler_slot")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub bank_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub slot_index: i32,
    pub track_id: Option<String>,
    pub path: Option<String>,
    pub label: Option<String>,
    pub updated_at: String,
    #[sea_orm(
        belongs_to,
        from = "bank_id",
        to = "id",
        on_delete = "Cascade",
        on_update = "Cascade"
    )]
    pub bank: HasOne<super::sampler_bank::Entity>,
    #[sea_orm(
        belongs_to,
        from = "track_id",
        to = "id",
        on_delete = "SetNull",
        on_update = "Cascade"
    )]
    pub track: HasOne<super::tracks::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
