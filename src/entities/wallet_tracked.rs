//! `SeaORM` Entity, @generated by sea-orm-codegen 1.0.0-rc.5

use super::sea_orm_active_enums::TrackType;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "wallet_tracked")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub wallet_id: u64,
    pub chat_id: i64,
    pub user_id: u64,
    pub wallet_address: String,
    pub nickname: Option<String>,
    pub track_type: TrackType,
    pub create_time: DateTimeUtc,
    #[sea_orm(column_type = "Double")]
    pub minimum_value: f64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
