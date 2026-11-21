//! The library's SQLite layer: one connection, the schema, the entity models
//! and the migration chain.
//!
//! Everything above this crate (queries, manager, domain services) talks to
//! [`db::Db`]; the schema itself belongs to [`migrations`].

pub mod db;
pub mod entity;
pub mod migrations;
