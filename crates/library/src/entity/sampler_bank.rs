use sea_orm::entity::prelude::*;

/// Bank play mode override. `NULL` / `None` = inherit app settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum SamplerPlayMode {
    #[sea_orm(string_value = "oneshot")]
    Oneshot,
    #[sea_orm(string_value = "hold")]
    Hold,
    #[sea_orm(string_value = "loop")]
    Loop,
}

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sampler_bank")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    /// `None` = inherit app settings play mode.
    pub play_mode: Option<SamplerPlayMode>,
    pub sort_index: i32,
    pub updated_at: String,
}

impl ActiveModelBehavior for ActiveModel {}
