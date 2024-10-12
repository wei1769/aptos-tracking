use crate::callback::CallbackCommand;
use crate::entities::sea_orm_active_enums::{Place, TrackType};
use crate::entities::{
    prelude::*,
    wallet_tracked::{self},
};
use crate::entities::{processed_block, token_info, user};
use crate::error::Err;
use ahash::AHashMap;

use sea_orm::entity::prelude::*;
use sea_orm::{FromQueryResult, QueryOrder, QuerySelect};

#[derive(FromQueryResult, Debug, Clone)]
pub struct WalletTrackedForChat {
    pub wallet_id: u64,
    pub chat_id: i64,
    pub user_id: u64,
    pub wallet_address: String,
    pub create_time: DateTimeUtc,
    pub track_type: TrackType,
    pub user_name: Option<String>,
    pub nickname: Option<String>,
}

#[derive(FromQueryResult, Debug, Clone)]
pub struct UniqueWallet {
    pub wallet_address: String,
}

impl WalletTrackedForChat {
    pub fn display_name(self) -> String {
        let name = self.nickname.unwrap_or(self.wallet_address);
        name.to_string()
    }
    pub fn address(&self) -> String {
        return self.wallet_address.as_str().to_string();
    }
    pub fn to_unsub_callback_data(&self) -> String {
        let data = CallbackCommand::Unsubscribe(self.wallet_id);
        data.to_callback_data()
    }
    pub fn to_update_callback_data(&self) -> String {
        let data = CallbackCommand::Update(self.wallet_id);
        data.to_callback_data()
    }
    pub async fn get_by_filter(
        db: &DatabaseConnection,
        chat_id: Option<i64>,
    ) -> Result<Vec<Self>, Err> {
        let select = WalletTracked::find();
        let select = match chat_id {
            Some(chat_id) => select.filter(wallet_tracked::Column::ChatId.eq(chat_id)),
            None => select,
        };
        Ok(select
            .column(user::Column::UserName)
            .join_rev(
                sea_orm::JoinType::InnerJoin,
                user::Entity::belongs_to(wallet_tracked::Entity)
                    .from(user::Column::UserId)
                    .to(wallet_tracked::Column::UserId)
                    .into(),
            )
            .into_model::<WalletTrackedForChat>()
            .all(db)
            .await?)
    }
}

impl wallet_tracked::Model {
    pub fn display_name(self) -> String {
        let name = self.nickname.unwrap_or(self.wallet_address);
        name.to_string()
    }
    pub fn address(&self) -> String {
        return self.wallet_address.as_str().to_string();
    }
    pub fn to_unsub_callback_data(&self) -> String {
        let data = CallbackCommand::Unsubscribe(self.wallet_id);
        data.to_callback_data()
    }
    pub async fn get_filtered(
        db: &DatabaseConnection,
        wallet_address: Option<String>,
    ) -> Result<Vec<Self>, Err> {
        let select = WalletTracked::find();
        let select = match wallet_address {
            Some(wallet_address) => {
                select.filter(wallet_tracked::Column::WalletAddress.eq(wallet_address))
            }
            None => select,
        };

        Ok(select.all(db).await?)
    }
}

impl UniqueWallet {
    pub async fn fetch_all(db: &DatabaseConnection) -> Result<Vec<Self>, Err> {
        Ok(WalletTracked::find()
            .select_only()
            .column(wallet_tracked::Column::WalletAddress)
            .group_by(wallet_tracked::Column::WalletAddress)
            .into_model::<UniqueWallet>()
            .all(db)
            .await?)
    }
    pub async fn fetch_all_hash_map(db: &DatabaseConnection) -> Result<AHashMap<String, ()>, Err> {
        Ok(Self::fetch_all(db)
            .await?
            .into_iter()
            .map(|s| (s.wallet_address, ()))
            .collect())
    }
}
#[derive(FromQueryResult, Debug, Clone)]
pub struct LastBlock {
    pub block: u64,
}

impl processed_block::Model {
    pub async fn get_last_block(
        db: &DatabaseConnection,
        modulo: Option<(u8, u8)>,
        place: Option<Place>,
        last: bool,
    ) -> Result<Option<u64>, Err> {
        let last_entry_select = ProcessedBlock::find()
            .select_only()
            .column(processed_block::Column::Block);
        let last_entry_select = match last {
            true => last_entry_select.order_by_asc(processed_block::Column::Block),
            false => last_entry_select.order_by_desc(processed_block::Column::Block),
        };

        let last_entry_select = match place {
            Some(place) => last_entry_select.filter(processed_block::Column::Place.eq(place)),
            None => last_entry_select,
        };
        let last_entry_select = match modulo {
            Some((m, b)) => last_entry_select
                .filter(processed_block::Column::Modulo.eq(m))
                .filter(processed_block::Column::Remainder.eq(b)),
            None => last_entry_select,
        };

        if let Some(entry) = last_entry_select.into_model::<LastBlock>().one(db).await? {
            return Ok(Some(entry.block));
        }
        Ok(None)
    }
    pub async fn get_oldest_block(db: &DatabaseConnection) -> Result<Option<u64>, Err> {
        let last_entry_select = ProcessedBlock::find()
            .select_only()
            .order_by_asc(processed_block::Column::Block)
            .filter(processed_block::Column::Place.eq(Place::Head))
            .column(processed_block::Column::Block);
        if let Some(entry) = last_entry_select.into_model::<LastBlock>().one(db).await? {
            return Ok(Some(entry.block));
        }
        Ok(None)
    }
    pub async fn get_block(db: &DatabaseConnection, block: u64) -> Result<Option<Self>, Err> {
        Ok(ProcessedBlock::find()
            .filter(processed_block::Column::Block.eq(block))
            .one(db)
            .await?)
    }
}
pub type TokenMap = ahash::AHashMap<String, token_info::Model>;
impl token_info::Model {
    pub async fn get_token_hashmap(db: &DatabaseConnection) -> Result<TokenMap, Err> {
        let all = TokenInfo::find().all(db).await?;
        let map: ahash::AHashMap<String, Self> = all
            .into_iter()
            .map(|entry| (entry.clone().mint, entry.clone()))
            .collect();
        Ok(map)
    }
}
