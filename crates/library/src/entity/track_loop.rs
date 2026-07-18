use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "track_loop")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub track_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub slot_index: i32,
    pub in_secs: f64,
    pub out_secs: f64,
    pub label: Option<String>,
    pub color: Option<String>,
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
