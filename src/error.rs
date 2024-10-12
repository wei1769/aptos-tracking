use crate::callback::ParseCallbackErr;
use aptos_sdk::crypto::hash::HashValueParseError;
use aptos_sdk::move_types::account_address::AccountAddressParseError;
use log::error;
use std::str::FromStr;
use teloxide::requests::Requester;
use teloxide::types::ChatId;

use aptos_sdk::rest_client::error::RestError;
use sea_orm::{DatabaseConnection, DbErr};
use std::sync::Arc;
use std::{fmt::Debug, num::ParseIntError};
use teloxide::error_handlers::ErrorHandler;

use teloxide::Bot;
use teloxide::{dispatching::dialogue::InMemStorageError, RequestError};

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
pub enum Err {
    Bot(RequestError),
    Db(DbErr),
    Callback(ParseCallbackErr),
    InMem(InMemStorageError),
    // Pubkey(ParsePubkeyError),
    Rqw(reqwest::Error),
    Sig(HashValueParseError),
    Rpc(RestError),
    ParseInt(ParseIntError),
    ParseAddress(AccountAddressParseError),
    ParseEventError,
    TxHasNoSig,
    UrlParseError,
    SlotDatabaseEmpty,
    TooMuchSlotEntryInDb,
    WaitTillBottomUpdate,
    OverFlow,
    TxPending,
}

pub struct HandleErr {
    db: Arc<DatabaseConnection>,
    bot: Bot,
    from: String,
}

impl HandleErr {
    pub fn new(db: Arc<DatabaseConnection>, bot: Bot, from: String) -> Arc<Self> {
        Arc::new(Self { db, bot, from })
    }
}
impl ErrorHandler<Err> for HandleErr {
    fn handle_error(self: Arc<Self>, error: Err) -> futures::future::BoxFuture<'static, ()> {
        let msg = match error {
            Err::Db(err) => {
                error!("Database ERROR {:?}", err);
                Some(err.to_string())
            }
            Err::Rpc(err) => {
                error!("RPC ERROR {:?}", err);
                Some(err.to_string())
            }
            _ => {
                error!("Error from {} err: {:?}", self.from, error);
                Some(error.to_string())
            }
        };
        Box::pin(async move {
            match msg {
                Some(msg) => {
                    if let Ok(chat_id) = std::env::var("REPORT_CHAT") {
                        if let Ok(id) = i64::from_str(&chat_id) {
                            let _result = self
                                .bot
                                .send_message(ChatId(id), format!("Error: {}", msg))
                                .await;
                        }
                    }
                }
                None => {}
            }
        })
    }
}
