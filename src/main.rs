use crate::db_update_loop::db_loop;

use crate::block::update_block_loop;
use crate::entities::token_info;
use crate::health_check_loop::health_check_loop;
use crate::process_new_block::handle_new_block_loop;
use health_check_loop::db_check;
use reqwest::Client as rqwClient;
use std::{sync::Arc, time::Duration};
use tx::TxQueryClient;
// use crate::token::{passive_update_price_feed, update_token_list};
use ahash::{AHashMap, AHashSet};
use aptos_sdk::move_types::account_address::AccountAddress;
use aptos_sdk::rest_client::Client;
use db::TokenMap;
use futures::future::{self, pending};
use log::*;
use sea_orm::{ConnectOptions, Database};
use std::str::FromStr;
use teloxide::prelude::*;
use tokio::sync::{mpsc, RwLock};
use url::Url;
mod block;
mod bot;
mod callback;
mod db;
mod db_update_loop;
mod entities;
mod error;
mod fetch_loop;
mod health_check_loop;
mod process_new_block;
mod process_new_tx;
mod query;
mod rpc;
// mod token;
mod tx;
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    pretty_env_logger::init_timed();
    info!("start");
    let mut opt = ConnectOptions::new(
        std::env::var("DATABASE_URL")
            .expect("no DATABASE_URL in .env")
            .to_owned(),
    );
    opt.sqlx_logging(false);
    opt.max_connections(50);
    let db = Arc::new(Database::connect(opt).await.expect("db connect error"));
    let bot = Bot::from_env();
    let rpc_url = std::env::var("RPC_URL")
        .expect("no RPC_URL in .env")
        .to_owned()
        .to_string();
    let rpc_connection = Arc::new(Client::new(
        Url::parse(&rpc_url).expect("RPC url parse error"),
    ));

    let back_up_rpc = std::env::var("BLOCK_RPC_URL")
        .unwrap_or(rpc_url)
        .to_owned()
        .to_string();
    let back_up_rpc_connection = Arc::new(Client::new(
        Url::parse(&back_up_rpc).expect("RPC url parse error"),
    ));
    let gq_url = std::env::var("GRAPHQL_URL")
        .expect("Graphql url needed")
        .to_owned()
        .to_string();
    let rqw_client = Arc::new(rqwClient::new());
    let tx_client = TxQueryClient {
        client: rqw_client,
        url: gq_url,
    };
    let (abortable, abort_handle) = future::abortable(pending::<()>());
    let new_token: Arc<RwLock<AHashSet<AccountAddress>>> = Arc::new(RwLock::new(AHashSet::new()));
    let tg_loop = async {
        bot::run(
            bot.clone(),
            db.clone(),
            rpc_connection.clone(),
            tx_client.clone(),
            new_token.clone(),
        )
        .await;
        abort_handle.abort();
    };
    let now_block = rpc_connection
        .get_ledger_information()
        .await
        .expect("RPC get slot error")
        .inner()
        .block_height;
    let current_block = Arc::new(RwLock::new(now_block));
    let token_map = Arc::new(RwLock::new(
        token_info::Model::get_token_hashmap(db.as_ref())
            .await
            .expect("token info fetch failed"),
    ));
    let updated_token_map: Arc<RwLock<TokenMap>> = Arc::new(RwLock::new(AHashMap::new()));
    let (new_price_update_tx, new_price_update_rx) = mpsc::channel::<(u64, Vec<u8>)>(5000);
    let current = current_block.read().await;
    db_check(rpc_connection.clone(), &db, current.clone())
        .await
        .expect("DB check error");
    drop(current);
    let main_thread_count =
        u8::from_str(&std::env::var("THREAD").expect("no THREAD in .env")).unwrap_or(3);
    tokio::join!(
        tg_loop,
        handle_new_block_loop(
            abortable.clone(),
            db.clone(),
            rpc_connection.clone(),
            tx_client.clone(),
            current_block.clone(),
            bot.clone(),
            main_thread_count,
            token_map.clone(),
            new_token.clone(),
            new_price_update_tx
        ),
        // f_loop(
        //     abortable.clone(),
        //     db.clone(),
        //     bot.clone(),
        //     tx_tx.clone(),
        //     rpc_connection.clone()
        // ),
        db_loop(abortable.clone(), db.clone()),
        update_block_loop(
            abortable.clone(),
            current_block.clone(),
            back_up_rpc_connection.clone(),
        ),
        // update_token_list(
        //     abortable.clone(),
        //     db.clone(),
        //     token_map.clone(),
        //     updated_token_map.clone(),
        //     new_token.clone(),
        //     rpc_connection.clone(),
        // ),
        health_check_loop(
            abortable.clone(),
            rpc_connection.clone(),
            current_block.clone(),
            db.clone(),
            bot.clone()
        ),
        // passive_update_price_feed(
        //     token_map.clone(),
        //     updated_token_map.clone(),
        //     new_price_update_rx
        // )
    );
}
