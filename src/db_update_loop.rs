use futures::{future::Pending, stream::Abortable};
use log::{error, info};
use sea_orm::{ActiveModelTrait, ActiveValue::*, ColumnTrait, QueryFilter, QueryOrder};
use sea_orm::{DatabaseConnection, EntityTrait};
use sea_query::Expr;
use std::sync::Arc;

use std::time::Duration;

use tokio::task;

use crate::entities::processed_block;
use crate::entities::sea_orm_active_enums::Place;
use crate::{entities::prelude::*, error::Err};

pub async fn db_loop(abort: Abortable<Pending<()>>, db: Arc<DatabaseConnection>) {
    loop {
        let db_new = db.clone();
        task::spawn_blocking(|| async move {
            // match check_update_cleanup(&db_new).await {
            //     Ok(_) => {}
            //     Err(e) => {
            //         error!("{:?}", e)
            //     }
            // };
            match check_slot(&db_new).await {
                Ok(processed) => {
                    let time: u64 = 500_usize
                        .checked_sub(processed * 50)
                        .unwrap_or(1)
                        .try_into()
                        .unwrap_or(0);
                    tokio::time::sleep(Duration::from_millis(time)).await;
                }
                Err(e) => {
                    error!("{:?}", e);
                    tokio::time::sleep(Duration::from_millis(10000)).await;
                }
            }
        })
        .await
        .unwrap()
        .await;
        if abort.is_aborted() {
            info!("abort");
            break;
        }
    }
}

async fn check_slot(db: &DatabaseConnection) -> Result<usize, Err> {
    let first_bottom = processed_block::Model::get_last_block(
        db,
        None,
        Some(crate::entities::sea_orm_active_enums::Place::Bottom),
        true,
    )
    .await?;
    let start_slot = match first_bottom {
        Some(s) => s,
        None => {
            let last_head = processed_block::Model::get_last_block(
                db,
                None,
                Some(crate::entities::sea_orm_active_enums::Place::Head),
                true,
            )
            .await?;
            //let oldest = processed_block::Model::get_oldest_slot(db).await?;
            match last_head {
                Some(s) => s,
                None => return Err(Err::SlotDatabaseEmpty),
            }
        }
    };
    let head_slots = ProcessedBlock::find()
        .filter(processed_block::Column::Block.gte(start_slot))
        .order_by_asc(processed_block::Column::Block)
        .all(db)
        .await?;

    let mut end = start_slot;
    let mut bottom = head_slots.len() - 1;
    for (index, slot) in head_slots.clone().into_iter().enumerate() {
        let index_u64: u64 = match index.try_into() {
            Ok(s) => s,
            Err(_) => {
                return Err(Err::TooMuchSlotEntryInDb);
            }
        };
        if slot.block != (start_slot + index_u64) {
            bottom = index - 1;
            end -= 1;
            break;
        } else {
            end = slot.block;
        }
    }
    if end != start_slot {
        ProcessedBlock::update_many()
            .col_expr(processed_block::Column::Place, Expr::value(Place::Old))
            .filter(processed_block::Column::Block.gte(start_slot))
            .filter(processed_block::Column::Block.lte(end))
            .exec(db)
            .await?;
    }

    let mut update = processed_block::ActiveModel::from(head_slots[bottom].clone());
    update.place = Set(Place::Bottom);
    update.update(db).await?;
    if bottom > 0 {
        info!("bottom updated {:?}", bottom);
    }

    Ok(bottom)
}
