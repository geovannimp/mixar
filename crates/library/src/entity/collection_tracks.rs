use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "collection_tracks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub collection_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub track_id: String,
    pub position: Option<i32>,
    #[sea_orm(
        belongs_to,
        from = "collection_id",
        to = "id",
        on_delete = "Cascade",
        on_update = "Cascade"
    )]
    pub collection: HasOne<super::collections::Entity>,
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
