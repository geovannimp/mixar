use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "collections")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub collection_type: String,
    #[sea_orm(default_value = 1)]
    pub sortable: i32,
    #[sea_orm(unique)]
    pub fs_path: Option<String>,
}

impl ActiveModelBehavior for ActiveModel {}
