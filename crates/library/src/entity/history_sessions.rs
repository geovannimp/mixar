use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "history_sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(unique)]
    pub xspf_path: String,
    pub title: String,
    pub started_at: String,
    pub last_activity_at: String,
    pub closed: i32,
    pub entry_count: i32,
}

impl ActiveModelBehavior for ActiveModel {}
