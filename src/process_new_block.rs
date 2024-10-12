use ahash::{AHashMap, AHashSet};
use aptos_sdk::move_types::account_address::AccountAddress;
use aptos_sdk::rest_client::Client;
use aptos_sdk::types::account_config::new_block;
use base58::FromBase58;
use futures::{future::Pending, stream::Abortable};
use graphql_client::GraphQLQuery;
use log::{error, info, warn};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use reqwest::Client as rqwClient;
use sea_orm::ActiveValue::Set;
use sea_orm::{entity::*, query::*, ActiveModelTrait, QueryFilter};
use sea_orm::{DatabaseConnection, EntityTrait, QuerySelect};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use teloxide::payloads::SendMessageSetters;
use teloxide::requests::Requester;
use teloxide::types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::{ApiError, Bot, RequestError};
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;

use tokio::task;

use crate::db::{LastBlock, TokenMap};
use crate::entities::sea_orm_active_enums::TrackType;
use crate::entities::{prelude::*, sea_orm_active_enums::Status, *};
use crate::query::transaction_query::ResponseData;
use crate::query::transactions_query::{TransactionsQueryFungibleAssetActivities, Variables};
use crate::query::TransactionsQuery;
use crate::tx::{u64_to_i128, BalanceChange, Filter, TxQueryClient};
use crate::{
    db::UniqueWallet,
    entities::wallet_tracked::{self},
    error::Err,
};
pub async fn handle_new_block_loop(
    abort: Abortable<Pending<()>>,
    db: Arc<DatabaseConnection>,
    rpc: Arc<Client>,
    tx_client: TxQueryClient,
    current_block: Arc<RwLock<u64>>,
    bot: Bot,
    modulo: u8,
    token: Arc<RwLock<TokenMap>>,
    new_token: Arc<RwLock<AHashSet<AccountAddress>>>,
    new_price_update_tx: Sender<(u64, Vec<u8>)>,
) {
    let mut futures = Vec::new();
    let all_wallet: Arc<RwLock<AHashMap<String, ()>>> = Arc::new(RwLock::new(AHashMap::new()));
    for remainder in 0..modulo {
        futures.push(tokio::spawn(handle_new_block_loop_inner(
            abort.clone(),
            db.clone(),
            rpc.clone(),
            tx_client.clone(),
            current_block.clone(),
            bot.clone(),
            Arc::new(modulo),
            Arc::new(remainder),
            all_wallet.clone(),
            token.clone(),
            new_token.clone(),
            new_price_update_tx.clone(),
        )))
    }
    futures.push(tokio::spawn(update_unique_loop(
        abort.clone(),
        all_wallet,
        db.clone(),
    )));
    futures::future::join_all(futures).await;
}
pub async fn update_unique_loop(
    abort: Abortable<Pending<()>>,
    all_wallet: Arc<RwLock<AHashMap<String, ()>>>,
    db: Arc<DatabaseConnection>,
) {
    'outer: loop {
        if let Ok(new) = UniqueWallet::fetch_all_hash_map(db.as_ref()).await {
            let mut update = all_wallet.write().await;
            *update = new;
            drop(update);
        };

        for _ in 0..5 {
            if abort.is_aborted() {
                info!("aborted");
                break 'outer;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

pub async fn handle_new_block_loop_inner(
    abort: Abortable<Pending<()>>,
    db: Arc<DatabaseConnection>,
    rpc: Arc<Client>,
    tx_client: TxQueryClient,
    current_block: Arc<RwLock<u64>>,
    bot: Bot,
    modulo: Arc<u8>,
    remainder: Arc<u8>,
    all_wallet: Arc<RwLock<AHashMap<String, ()>>>,
    token: Arc<RwLock<TokenMap>>,
    new_token: Arc<RwLock<AHashSet<AccountAddress>>>,
    new_price_update_tx: Sender<(u64, Vec<u8>)>,
) {
    loop {
        let db_new = db.clone();
        let rpc_new = rpc.clone();
        let bot_new = bot.clone();
        let modulo_new = modulo.clone();
        let remainder_new = remainder.clone();
        let current_block_new = current_block.clone();
        let all_wallet_new = all_wallet.clone();
        let token_new = token.clone();
        let new_token_new = new_token.clone();
        let new_price_update_tx = new_price_update_tx.clone();
        let tx_client_new = tx_client.clone();
        if let Err(err) = task::spawn(async move {
            match handle_new_block(
                &db_new.clone(),
                rpc_new.clone(),
                tx_client_new.clone(),
                bot_new.clone(),
                &modulo_new.clone(),
                &remainder_new.clone(),
                &current_block_new.clone(),
                &all_wallet_new.clone(),
                &token_new.clone(),
                &new_token_new.clone(),
                new_price_update_tx,
            )
            .await
            {
                Ok(_) => {}
                Err(e) => {
                    if let Err::SlotDatabaseEmpty = e {
                        tokio::time::sleep(Duration::from_millis(300)).await;
                    }
                    error!("({}, {}) {:?}", modulo_new, remainder_new, e)
                }
            };
            // tokio::time::sleep(Duration::from_secs(2)).await;
        })
        .await
        {
            warn!("Join error: {:?}", err);
        };
        if abort.is_aborted() {
            info!("abort");
            break;
        }
    }
}

async fn handle_new_block(
    db: &DatabaseConnection,
    rpc: Arc<Client>,
    tx_client: TxQueryClient,
    bot: Bot,
    modulo: &u8,
    remainder: &u8,
    current_block: &RwLock<u64>,
    all_wallet: &RwLock<AHashMap<String, ()>>,
    token: &RwLock<TokenMap>,
    new_token: &RwLock<AHashSet<AccountAddress>>,
    new_price_update_tx: Sender<(u64, Vec<u8>)>,
) -> Result<(), Err> {
    let this_block = get_block_to_process(db, modulo, remainder).await?;
    let block_now = current_block.read().await;
    if this_block > *block_now {
        let time: u64 = this_block.saturating_sub(*block_now) * 250;
        warn!("Too fast, {} > {}", this_block, *block_now);
        drop(block_now);
        tokio::time::sleep(Duration::from_millis(time)).await;
        return Ok(());
    }

    let new_block = match rpc.get_block_by_height_bcs(this_block, false).await {
        Ok(s) => s.inner().clone(),
        Err(err) => match err {
            _ => {
                return Err(Err::Rpc(err));
            }
        },
    };
    let all_tracked = all_wallet.read().await;
    let new_entry = processed_block::ActiveModel {
        block: Set(this_block),
        modulo: Set(*modulo),
        remainder: Set(*remainder),
        ..Default::default()
    };
    let result = new_entry.insert(db).await?;
    if all_tracked.len() == 0 {
        return Ok(());
    }
    let new_block_txs_query = Variables {
        gte: Some(u64_to_i128(new_block.first_version)),
        lte: Some(u64_to_i128(new_block.last_version)),
    };
    let txs = tx_client.get_tx_in_range(new_block_txs_query).await?;

    match txs.data {
        Some(txs) => {
            let mut txs_sorted = txs.clone();
            txs_sorted
                .fungible_asset_activities
                .sort_by(|a, b| a.transaction_version.cmp(&b.transaction_version));
            let mut txs_by_version: Vec<(i128, Vec<TransactionsQueryFungibleAssetActivities>)> =
                vec![];
            txs_sorted
                .fungible_asset_activities
                .into_iter()
                .for_each(|event| {
                    match txs_by_version
                        .binary_search_by(|(version, _)| version.cmp(&event.transaction_version))
                    {
                        Ok(index) => {
                            if let Some((version, events)) = txs_by_version.get_mut(index) {
                                events.push(event);
                            };
                        }
                        Err(_) => {
                            txs_by_version.push((event.transaction_version, vec![event]));
                        }
                    };
                });
            let len = txs_by_version.len();

            for (version, tx) in txs_by_version.into_iter() {
                match tx.get(0) {
                    Some(info) => {
                        if !info.is_transaction_success {
                            continue;
                        }
                    }
                    None => {
                        continue;
                    }
                }

                let mut keys: Vec<_> = tx
                    .clone()
                    .into_par_iter()
                    .filter_map(|info| info.owner_address)
                    .collect();
                keys.dedup();
                for key in keys.clone() {
                    if all_tracked.contains_key(&key) {
                        if let Some(balance_changes) = BalanceChange::from_events(tx.clone()) {
                            let token = token.read().await;
                            for change in balance_changes.clone() {
                                if !token.contains_key(&change.to_token_address()) {
                                    if let Ok(address) =
                                        AccountAddress::from_str(&change.to_token_address())
                                    {
                                        let mut update = new_token.write().await;
                                        update.insert(address);
                                        drop(update)
                                    }
                                }
                            }
                            let pubkey = AccountAddress::from_str(&key)?;
                            let filtered = balance_changes.clone().get_by_key(&pubkey);
                            if filtered.is_empty() {
                                continue;
                            }

                            let all_chat =
                                wallet_tracked::Model::get_filtered(db, Some(key)).await?;
                            let response = filtered
                                .clone()
                                .into_iter()
                                .map(|s| {
                                    format!(
                                        "{} $({:.3})\n",
                                        s.to_priced_string(&(token)),
                                        s.to_usd_change(&(token)).unwrap_or(0.0)
                                    )
                                })
                                .collect::<Vec<String>>()
                                .concat();
                            let value_changes = filtered
                                .clone()
                                .into_iter()
                                .map(|s| {
                                    let net = s.to_usd_change(&(token)).unwrap_or(0.0);
                                    (net.is_sign_negative(), net.abs())
                                })
                                .collect::<Vec<(bool, f64)>>();
                            let full_change =
                                value_changes.clone().into_iter().map(|c| c.1).sum::<f64>();
                            let sent_value = value_changes
                                .clone()
                                .into_iter()
                                .filter_map(|c| match c.0 {
                                    true => Some(c.1),
                                    false => None,
                                })
                                .sum::<f64>();
                            let received_value = value_changes
                                .clone()
                                .into_iter()
                                .filter_map(|c| match c.0 {
                                    false => Some(c.1),
                                    true => None,
                                })
                                .sum::<f64>();
                            let tx_id = match tx.get(0) {
                                Some(s) => s.transaction_version,
                                None => {
                                    return Err(Err::TxHasNoSig);
                                }
                            };
                            let url = match reqwest::Url::parse(&format!(
                                "https://explorer.aptoslabs.com/txn/{}?network=mainnet",
                                tx_id
                            )) {
                                Ok(s) => s,
                                Err(_) => {
                                    return Err(Err::UrlParseError);
                                }
                            };

                            for chat in all_chat.into_iter() {
                                if match chat.track_type {
                                    TrackType::Full => true,
                                    TrackType::Balance => full_change >= chat.minimum_value,
                                    TrackType::Receive => received_value >= chat.minimum_value,
                                    TrackType::Sent => sent_value >= chat.minimum_value,
                                } {
                                    let item = vec![
                                        InlineKeyboardButton::url("TX detail", url.clone()),
                                        InlineKeyboardButton::callback(
                                            "Unsubscribe",
                                            chat.to_unsub_callback_data(),
                                        ),
                                    ];
                                    match bot
                                        .send_message(ChatId(chat.chat_id), response.clone())
                                        .reply_markup(InlineKeyboardMarkup::new([item.clone()]))
                                        .await
                                    {
                                        Ok(_s) => {}
                                        Err(err) => match err {
                                            RequestError::RetryAfter(_) => {}
                                            RequestError::Api(err) => {
                                                if let ApiError::BotBlocked = err {
                                                    let _wallet_blocked =
                                                        WalletTracked::delete_many()
                                                            .filter(
                                                                wallet_tracked::Column::ChatId
                                                                    .eq(chat.chat_id),
                                                            )
                                                            .exec(db)
                                                            .await?;
                                                    warn!(
                                                        "{:?} is blocked deleted all ",
                                                        chat.chat_id
                                                    );
                                                }
                                            }
                                            RequestError::MigrateToChatId(id) => {
                                                let mut update = chat.into_active_model();
                                                update.chat_id = Set(id);
                                                if let Err(err) = update.update(db).await {
                                                    error!("Error Migrating chat: {:?}", err);
                                                };
                                            }
                                            _ => {
                                                error!("{:?}", err);
                                            }
                                        },
                                    };
                                };
                            }
                        }
                    }
                }
                // const JUP_ROUTER: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
                // const JUP_EVENT: &str = "D8cy77BBepLMngZx6ZukaTff5hCt1HrWyKk3Hnd9oitf";
                // let router_index = keys.clone().into_iter().position(|key| key == JUP_ROUTER);
                // let event_index = keys.into_iter().position(|key| key == JUP_EVENT);
                // if router_index.is_some() && event_index.is_some() {
                //     if let Some(meta) = tx.meta {
                //         let inner_ixs = match meta.inner_instructions {
                //             OptionSerializer::Some(ixs) => ixs,
                //             _ => vec![],
                //         };
                //         for ixs in inner_ixs {
                //             for ix in ixs.instructions {
                //                 if let UiInstruction::Compiled(ix_detail) = ix {
                //                     let program_id_index: usize = ix_detail.program_id_index.into();
                //                     let first = match ix_detail.accounts.first() {
                //                         Some(index) => {
                //                             let index: usize = (*index).into();
                //                             Some(index)
                //                         }
                //                         None => None,
                //                     };

                //                     if Some(program_id_index) == router_index
                //                         && event_index == first
                //                     {
                //                         match ix_detail.data.from_base58() {
                //                             Ok(data_byte) => {
                //                                 if let Err(result) = new_price_update_tx
                //                                     .send((this_block, data_byte))
                //                                     .await
                //                                 {
                //                                     error!(
                //                                         "Error sending event data: {:?}",
                //                                         result
                //                                     );
                //                                 }
                //                             }
                //                             Err(e) => {
                //                                 error!(
                //                                     "base58 decode error: {:?}, data: {}",
                //                                     e, ix_detail.data
                //                                 );
                //                             }
                //                         }
                //                     }
                //                 }
                //             }
                //         }
                //     }
                // }
            }

            let update = processed_block::ActiveModel {
                block: Set(result.block),
                status: Set(Status::Complete),
                tx_count: Set(len.try_into().unwrap_or(u32::MAX)),
                ..Default::default()
            };
            update.update(db).await?;
        }
        None => {
            warn!("None");
            let update = processed_block::ActiveModel {
                block: Set(result.block),
                status: Set(Status::Complete),
                tx_count: Set(0),
                ..Default::default()
            };
            update.update(db).await?;
        }
    }

    Ok(())
}

async fn get_block_to_process(
    db: &DatabaseConnection,
    modulo: &u8,
    remainder: &u8,
) -> Result<u64, Err> {
    let bottom_last = match processed_block::Model::get_last_block(
        db,
        None,
        Some(sea_orm_active_enums::Place::Bottom),
        true,
    )
    .await?
    {
        Some(s) => s,
        None => {
            match processed_block::Model::get_last_block(
                db,
                None,
                Some(sea_orm_active_enums::Place::Old),
                false,
            )
            .await?
            {
                Some(s) => s,
                None => return Err(Err::SlotDatabaseEmpty),
            }
        }
    };
    let modulo: u64 = (*modulo).into();
    let remainder: u64 = (*remainder).into();
    let new_blocks = ProcessedBlock::find()
        .select_only()
        .column(processed_block::Column::Block)
        .filter(processed_block::Column::Block.gte(bottom_last))
        .order_by_asc(processed_block::Column::Block)
        .limit(Some(modulo + 1))
        .into_model::<LastBlock>()
        .all(db)
        .await?
        .into_iter()
        .map(|s| s.block)
        .collect::<Vec<u64>>();
    match new_blocks.first() {
        Some(s) => {
            if *s != bottom_last {
                return Err(Err::SlotDatabaseEmpty);
            };
        }
        None => {
            return Err(Err::SlotDatabaseEmpty);
        }
    }
    let last = match new_blocks.last() {
        Some(s) => *s,
        None => bottom_last,
    };
    let len: usize = match (last - bottom_last).try_into() {
        Ok(s) => s,
        Err(_err) => return Err(Err::OverFlow),
    };
    let mut check_vec = vec![0_u8; len + 1];
    for block in new_blocks {
        match block.checked_sub(bottom_last) {
            Some(s) => {
                let index: usize = match s.try_into() {
                    Ok(s) => s,
                    Err(_err) => return Err(Err::OverFlow),
                };
                check_vec[index] = 1;
            }
            None => {}
        };
    }

    let mut to_check: Option<u64> = None;
    for (index, check) in check_vec.into_iter().enumerate() {
        if check == 0 {
            let to_add: u64 = match index.try_into() {
                Ok(s) => s,
                Err(_err) => return Err(Err::OverFlow),
            };
            if (bottom_last + to_add) % modulo == remainder {
                to_check = Some(bottom_last + to_add);
                break;
            }
        }
    }
    let to_check = to_check.unwrap_or(last + ((modulo + remainder - (last % modulo)) % modulo));

    for index in 0..20 * modulo {
        let to_check = to_check + index * modulo;
        if processed_block::Model::get_block(db, to_check)
            .await?
            .is_none()
        {
            return Ok(to_check);
        }
    }
    Err(Err::WaitTillBottomUpdate)
}
