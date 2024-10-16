//! `SeaORM` Entity, @generated by sea-orm-codegen 1.0.0-rc.5

use sea_orm::entity::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "place")]
pub enum Place {
    #[sea_orm(string_value = "head")]
    Head,
    #[sea_orm(string_value = "bottom")]
    Bottom,
    #[sea_orm(string_value = "old")]
    Old,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "status")]
pub enum Status {
    #[sea_orm(string_value = "complete")]
    Complete,
    #[sea_orm(string_value = "processing")]
    Processing,
    #[sea_orm(string_value = "error")]
    Error,
    #[sea_orm(string_value = "skipped")]
    Skipped,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "track_type")]
pub enum TrackType {
    #[sea_orm(string_value = "full")]
    Full,
    #[sea_orm(string_value = "sent")]
    Sent,
    #[sea_orm(string_value = "balance")]
    Balance,
    #[sea_orm(string_value = "receive")]
    Receive,
}
