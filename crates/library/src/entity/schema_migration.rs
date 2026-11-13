#![allow(clippy::wildcard_imports)] // sea_orm entity prelude

use sea_orm::entity::prelude::*;

/// Applied library migrations.
///
/// The table is named `seaql_migrations` so it matches SeaORM's conventional
/// migration ledger; the runner in [`crate::migrations`] owns it.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "seaql_migrations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub version: String,
    pub applied_at: String,
}

impl ActiveModelBehavior for ActiveModel {}
