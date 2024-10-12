use aptos_sdk::rest_client::Client;
use futures::{future::Pending, stream::Abortable};
use log::*;
use rust_decimal::prelude::ToPrimitive;
use sea_orm::{
    entity::*, query::*, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter,
};
use sea_query::DeleteStatement;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use teloxide::requests::Requester;
use teloxide::types::ChatId;
use teloxide::Bot;
use tokio::sync::RwLock;

use crate::error::Err;
use crate::{entities::prelude::*, entities::processed_block, entities::sea_orm_active_enums::*};
use std::str::FromStr;
struct TimeAndBlock {
    time: SystemTime,
    block: u64,
}
pub async fn health_check_loop(
    abort: Abortable<Pending<()>>,
    rpc: Arc<Client>,
    current_block: Arc<RwLock<u64>>,
    db: Arc<DatabaseConnection>,
    bot: Bot,
) {
    if std::env::var("REPORT_CHAT").is_ok() {
        let info = rpc.get_ledger_information().await;
        let now_block = match info {
            Ok(res) => {
                let height = res.inner().block_height;
                height
            }
            Err(_) => 0,
        };
        let count_and_time = Arc::new(RwLock::new(TimeAndBlock {
            time: SystemTime::now(),
            block: now_block,
        }));
        'outer: loop {
            if let Err(e) = health_check(
                &rpc,
                &db,
                bot.clone(),
                current_block.clone(),
                &count_and_time.clone(),
            )
            .await
            {
                error!("{:?}", e);
            }
            for _ in 0..120 * 1 {
                if abort.is_aborted() {
                    info!("aborted");
                    break 'outer;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

pub async fn db_check(rpc: Arc<Client>, db: &DatabaseConnection, current: u64) -> Result<(), Err> {
    if let None = processed_block::Model::get_last_block(
        db,
        None,
        Some(crate::entities::sea_orm_active_enums::Place::Bottom),
        false,
    )
    .await?
    {
        let block_now_entry = processed_block::ActiveModel {
            block: Set(current.clone()),
            ..Default::default()
        };
        let result = block_now_entry.insert(db).await?;
    }

    Ok(())
}

async fn health_check(
    rpc: &Client,
    db: &DatabaseConnection,
    bot: Bot,
    current_block: Arc<RwLock<u64>>,
    time_and_block: &RwLock<TimeAndBlock>,
) -> Result<(), Err> {
    if let Some(last_block) = processed_block::Model::get_last_block(
        db,
        None,
        Some(crate::entities::sea_orm_active_enums::Place::Old),
        false,
    )
    .await?
    {
        if let Ok(chat_id) = std::env::var("REPORT_CHAT") {
            let backup_block = current_block.read().await;
            let info = rpc.get_ledger_information().await?;
            let current_block = info.inner().block_height;

            let new_time_block = TimeAndBlock {
                block: last_block,
                time: SystemTime::now(),
            };
            let last = time_and_block.read().await;
            if let Some(block_processed) = new_time_block.block.checked_sub(last.block) {
                if let Ok(duration) = new_time_block.time.duration_since(last.time) {
                    let sec = duration.as_secs_f32();
                    if sec >= 0.1 {
                        let block_per_sec = block_processed.to_f32().unwrap_or(0.0) / sec;
                        let id = i64::from_str(&chat_id)?;
                        bot.send_message(
                            ChatId(id),
                            format!("Processing at {} block/sec", block_per_sec),
                        )
                        .await?;
                    }
                }
            }
            drop(last);
            let mut update = time_and_block.write().await;
            *update = new_time_block;
            drop(update);
            if current_block < last_block {
                let id = i64::from_str(&chat_id)?;
                bot.send_message(
                    ChatId(id),
                    format!(
                        "Current block error\nRollback {} block",
                        last_block - current_block
                    ),
                )
                .await?;
            } else if current_block - last_block > 1000 {
                let id = i64::from_str(&chat_id)?;
                bot.send_message(
                    ChatId(id),
                    format!("Tracking is {} block behind", current_block - last_block),
                )
                .await?;
            }

            if backup_block.abs_diff(current_block) > 100 {
                match backup_block.cmp(&current_block) {
                    std::cmp::Ordering::Greater => {
                        bot.send_message(
                            chat_id,
                            format!(
                                "Main RPC is too slow {} block behind backup\nMain at {}, Backup at {}",
                                backup_block.checked_sub(current_block).unwrap_or(0),
                                current_block,
                                backup_block
                            )
                        ).await?;
                    }
                    std::cmp::Ordering::Less => {
                        bot.send_message(
                            chat_id,
                            format!(
                                "Backup RPC is too slow {} block behind main\nMain at {}, Backup at {}",
                                current_block.saturating_sub(*backup_block),
                                current_block,
                                backup_block
                            )
                        ).await?;
                    }
                    std::cmp::Ordering::Equal => {}
                }
            }
        }
    }
    Ok(())
}
