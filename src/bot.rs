type OptString = Option<String>;
use ahash::AHashSet;
use aptos_sdk::move_types::account_address::AccountAddress;
use aptos_sdk::rest_client::Client;
use log::{error, warn};
use reqwest::Client as rqwClient;
use sea_orm::{ActiveModelTrait, ActiveValue::*};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use sea_query::Expr;
use std::sync::Arc;
use std::{fmt::Debug, ops::Not, str::FromStr, time::Duration};
use teloxide::RequestError;
use teloxide::{
    dispatching::dialogue::Dialogue,
    dispatching::dialogue::GetChatId,
    dispatching::dialogue::InMemStorage,
    dispatching::UpdateFilterExt,
    dptree::{case, deps},
    prelude::{Requester, *},
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
    utils::command::{BotCommands, ParseError},
};
use tokio::sync::RwLock;

use crate::db::UniqueWallet;
use crate::entities::sea_orm_active_enums::{Place, TrackType};
use crate::entities::{prelude::*, processed_block, token_info, user, wallet_tracked};
use crate::error::{Err, HandleErr};
use crate::tx::{get_tx_detail, TxQueryClient};
use crate::{callback::CallbackCommand, db::WalletTrackedForChat};
const SLEEP: Duration = Duration::from_secs(3);

#[derive(BotCommands, Clone, PartialEq, Eq, Debug)]
#[command(rename_rule = "lowercase", parse_with = "split")]
enum Command {
    Start,
    #[command(parse_with = opt2)]
    Subscribe(ParsedSubscribe),
    #[command(parse_with = opt)]
    Unsubscribe(OptString),
    #[command(parse_with = opt)]
    Test(OptString),
    #[command(parse_with = opt)]
    Tx(OptString),
    Status,
    List,
    #[command(parse_with = opt)]
    Update(OptString),
    #[command(parse_with = opt)]
    Token(OptString),
}

type ChatState = Dialogue<State, InMemStorage<State>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Idle,
    UpdateNickname(u64),
    UpdateFilter(u64),
    // Subscribe,
}
pub async fn run(
    bot: Bot,
    db: Arc<DatabaseConnection>,
    rpc: Arc<Client>,
    tx_client: TxQueryClient,
    new_token: Arc<RwLock<AHashSet<AccountAddress>>>,
) {
    let commands = |bot: Bot,
                    msg: Message,
                    cmd: Command,
                    db: Arc<DatabaseConnection>,
                    rpc: Arc<Client>,
                    tx_client: TxQueryClient,
                    new_token: Arc<RwLock<AHashSet<AccountAddress>>>| async move {
        let chat_id = get_chat_id(bot.clone(), db.as_ref(), msg.chat.id).await?;
        let db = db.clone();
        let handle_not_register = || async {
            let msg2 = bot
                .send_message(
                    chat_id,
                    format!(
                        "User not register yet\nTalk to the bot via @{}",
                        bot.get_me().await.unwrap().username()
                    ),
                )
                .reply_to_message_id(msg.id)
                .await?;
            tokio::time::sleep(SLEEP).await;
            bot.delete_message(chat_id, msg2.id).await?;
            Ok::<_, Err>(())
        };
        let from = match msg.from() {
            None => {
                bot.send_message(chat_id, "Invalid message, not sender provided")
                    .reply_to_message_id(msg.id)
                    .await?;
                return Ok(());
            }
            Some(user) => user,
        };
        let user_check = User::find_by_id(from.id.0).one(db.as_ref()).await?;

        match cmd {
            Command::Status => {
                let all_chat = WalletTracked::find().all(db.as_ref()).await?;
                let unique_wallet = UniqueWallet::fetch_all(db.as_ref()).await?;
                let info = rpc.get_ledger_information().await?;
                let current_block = info.inner().block_height;
                let token = token_info::Model::get_token_hashmap(db.as_ref()).await?;

                if let Some(last_slot) = processed_block::Model::get_last_block(
                    db.as_ref(),
                    None,
                    Some(Place::Old),
                    false,
                )
                .await?
                {
                    bot.send_message(
                        chat_id,
                        format!(
                            "{} Wallet tracking, {} Chats, {} slot behind\n{} tokens",
                            unique_wallet.len(),
                            all_chat.len(),
                            current_block.saturating_sub(last_slot),
                            token.len()
                        ),
                    )
                    .reply_to_message_id(msg.id)
                    .await?;
                } else {
                    return Err(Err::SlotDatabaseEmpty);
                }
            }
            Command::Tx(data) => {
                if let Some(sig) = data {
                    let result = get_tx_detail(&tx_client, u64::from_str(&sig)?).await?;
                    match result {
                        Some(s) => {
                            let response = s
                                .into_iter()
                                .map(|s| format!("{}\n", s.to_string()))
                                .collect::<Vec<String>>()
                                .concat();
                            bot.send_message(chat_id, response)
                                .reply_to_message_id(msg.id)
                                .await?;
                        }
                        None => {
                            let reply = bot
                                .send_message(chat_id, "No tx id provided plz try again")
                                .reply_to_message_id(msg.id)
                                .await?;
                            tokio::time::sleep(SLEEP).await;
                            bot.delete_message(chat_id, reply.id).await?;
                        }
                    }
                }
            }
            Command::Test(_data) => {}
            Command::Token(data) => {
                if let Some(mint) = data {
                    if let Ok(parse) = AccountAddress::from_str(&mint) {
                        let mut update = new_token.write().await;
                        update.insert(parse);
                        drop(update);
                        bot.send_message(chat_id, "Token added").await?;
                    };
                }
            }
            Command::Start => {
                if !msg.chat.is_private() {
                    let msg2 = bot
                        .send_message(
                            chat_id,
                            format!(
                                "/start command is only available in dm\nTalk to the bot via @{}",
                                bot.get_me().await.unwrap().username()
                            ),
                        )
                        .reply_to_message_id(msg.id)
                        .disable_notification(true)
                        .await?;

                    tokio::time::sleep(SLEEP).await;

                    bot.delete_message(chat_id, msg2.id).await?;

                    return Ok(());
                }
                if user_check.is_some() {
                    let msg2= bot.send_message(chat_id, "You are already registered\nuse /subscribe <wallet address> to track wallet update")
                            .reply_to_message_id(msg.id)
                            .await?;
                    tokio::time::sleep(SLEEP).await;
                    bot.delete_message(chat_id, msg2.id).await?;
                } else {
                    let new_user = user::ActiveModel {
                        user_name: sea_orm::ActiveValue::Set(from.username.clone()),
                        user_id: sea_orm::ActiveValue::Set(from.id.0),
                        ..Default::default()
                    };
                    User::insert(new_user).exec(db.as_ref()).await?;
                    bot.send_message(
                        chat_id,
                        "Welcome\nuse /subscribe <wallet address> to track wallet update",
                    )
                    .reply_to_message_id(msg.id)
                    .await?;
                }
            }
            Command::Subscribe(req) => {
                if bot
                    .get_chat_member(chat_id, from.id)
                    .await?
                    .can_manage_chat()
                    .not()
                    && !msg.chat.is_private()
                {
                    let msg2 = bot
                        .send_message(chat_id, "Only group admin can do this")
                        .reply_to_message_id(msg.id)
                        .await?;
                    tokio::time::sleep(SLEEP).await;
                    bot.delete_message(chat_id, msg2.id).await?;
                    return Ok::<_, Err>(());
                }
                let user = match user_check {
                    Some(user) => user,
                    None => {
                        handle_not_register().await?;
                        return Ok(());
                    }
                };

                match req.address {
                    None => {
                        let msg2 = bot
                            .send_message(chat_id, "No address provided, try /subscribe <address>")
                            .reply_to_message_id(msg.id)
                            .await?;
                        tokio::time::sleep(SLEEP).await;
                        bot.delete_message(chat_id, msg2.id).await?;
                    }
                    Some(address) => {
                        match AccountAddress::from_str(&address) {
                            Err(e) => {
                                let message = e.to_string();
                                bot.send_message(
                                    chat_id,
                                    format!("Address Invalid: {:?}", message),
                                )
                                .reply_to_message_id(msg.id)
                                .await?;
                            }
                            Ok(address) => {
                                let track_exist = WalletTracked::find()
                                    .filter(
                                        wallet_tracked::Column::WalletAddress
                                            .eq(address.to_string()),
                                    )
                                    .filter(wallet_tracked::Column::ChatId.eq(chat_id.0))
                                    .one(db.as_ref())
                                    .await?
                                    .is_some();
                                if track_exist {
                                    bot.send_message(
                                        chat_id,
                                        "It's already tracked\nuse /list to see all subscriptions",
                                    )
                                    .reply_to_message_id(msg.id)
                                    .await?;
                                } else {
                                    let new_tracking = wallet_tracked::ActiveModel {
                                        chat_id: sea_orm::ActiveValue::Set(chat_id.0),
                                        wallet_address: sea_orm::ActiveValue::Set(
                                            address.to_string(),
                                        ),
                                        user_id: sea_orm::ActiveValue::set(user.user_id),
                                        nickname: sea_orm::ActiveValue::set(req.nickname),
                                        ..Default::default()
                                    };
                                    WalletTracked::insert(new_tracking)
                                        .exec(db.as_ref())
                                        .await?;
                                    bot.send_message(chat_id, "Subscription added")
                                        .reply_to_message_id(msg.id)
                                        .await?;
                                }
                            }
                        };
                    }
                }
            }
            Command::Unsubscribe(address) => {
                if bot
                    .get_chat_member(chat_id, from.id)
                    .await?
                    .can_manage_chat()
                    .not()
                    && !msg.chat.is_private()
                {
                    let msg2 = bot
                        .send_message(chat_id, "Only group admin can do this")
                        .reply_to_message_id(msg.id)
                        .await?;
                    tokio::time::sleep(SLEEP).await;
                    bot.delete_message(chat_id, msg2.id).await?;
                    return Ok::<_, Err>(());
                };
                match address {
                    None => {
                        let tracked =
                            WalletTrackedForChat::get_by_filter(db.as_ref(), Some(chat_id.0))
                                .await?;
                        if tracked.is_empty() {
                            bot.send_message(
                                chat_id,
                                "No subscription is added for this chat, try /subscribe <address>",
                            )
                            .reply_to_message_id(msg.id)
                            .await?;
                        } else {
                            let item = tracked.into_iter().map(|wallet| {
                                let data = wallet.to_unsub_callback_data();
                                vec![InlineKeyboardButton::callback(wallet.display_name(), data)]
                            });

                            bot.send_message(chat_id, "Select a address to unsub")
                                .reply_to_message_id(msg.id)
                                .reply_markup(InlineKeyboardMarkup::new(item))
                                .await?;
                        }
                    }
                    Some(address) => {
                        match WalletTracked::find()
                            .filter(wallet_tracked::Column::ChatId.eq(chat_id.0))
                            .filter(wallet_tracked::Column::WalletAddress.eq(address.clone()))
                            .one(db.as_ref())
                            .await?
                        {
                            Some(entry) => {
                                wallet_tracked::Entity::delete_by_id(entry.wallet_id)
                                    .exec(db.as_ref())
                                    .await?;
                                bot.send_message(chat_id, format!("{address} Unsubscribed"))
                                    .await?;
                            }
                            None => {
                                bot.send_message(chat_id, "Address Not found").await?;
                            }
                        }
                    }
                }
            }
            Command::List => match user_check {
                None => {
                    handle_not_register().await?;
                    return Ok(());
                }
                Some(_user) => {
                    let tracked =
                        WalletTrackedForChat::get_by_filter(db.as_ref(), Some(chat_id.0)).await?;
                    if tracked.is_empty() {
                        bot.send_message(
                            chat_id,
                            "No subscription is added for this chat, try /subscribe <address>",
                        )
                        .reply_to_message_id(msg.id)
                        .await?;
                    } else {
                        let list = tracked
                            .iter()
                            .map(|wallet| {
                                if msg.chat.is_private() {
                                    return wallet
                                        .nickname
                                        .clone()
                                        .unwrap_or(wallet.wallet_address.clone());
                                }
                                format!(
                                    "{} subscription added by @{}",
                                    wallet
                                        .nickname
                                        .clone()
                                        .unwrap_or(wallet.wallet_address.clone()),
                                    wallet.user_name.clone().unwrap_or(format!(
                                        "[{0}](tg://user?id={0})",
                                        wallet.user_id.clone().to_string()
                                    )),
                                )
                            })
                            .collect::<Vec<String>>()
                            .join("\n");
                        let reply = format!("These are all the wallet you are tracking\n{}", list)
                            .replace('_', r"\_");
                        bot.send_message(chat_id, reply)
                            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                            .reply_to_message_id(msg.id)
                            .await?;
                    }
                }
            },
            Command::Update(address) => {
                if bot
                    .get_chat_member(chat_id, from.id)
                    .await?
                    .can_manage_chat()
                    .not()
                    && !msg.chat.is_private()
                {
                    let msg2 = bot
                        .send_message(chat_id, "Only group admin can do this")
                        .reply_to_message_id(msg.id)
                        .await?;
                    tokio::time::sleep(SLEEP).await;
                    bot.delete_message(chat_id, msg2.id).await?;
                    return Ok::<_, Err>(());
                }
                match address {
                    None => {
                        let tracked =
                            WalletTrackedForChat::get_by_filter(db.as_ref(), Some(chat_id.0))
                                .await?;
                        if tracked.is_empty() {
                            bot.send_message(
                                chat_id,
                                "No subscription is added for this chat, try /subscribe <address>",
                            )
                            .reply_to_message_id(msg.id)
                            .await?;
                        } else {
                            let item = tracked.into_iter().map(|wallet| {
                                let data = wallet.to_update_callback_data();
                                warn!("{:?}", wallet);
                                vec![InlineKeyboardButton::callback(wallet.display_name(), data)]
                            });
                            bot.send_message(chat_id, "Select a address to update")
                                .reply_to_message_id(msg.id)
                                .reply_markup(InlineKeyboardMarkup::new(item))
                                .await?;
                        }
                    }
                    Some(address) => {
                        match WalletTracked::find()
                            .filter(wallet_tracked::Column::ChatId.eq(chat_id.0))
                            .filter(wallet_tracked::Column::WalletAddress.eq(address.clone()))
                            .one(db.as_ref())
                            .await?
                        {
                            Some(info) => {
                                update_handler(bot, chat_id, info.wallet_id, address).await?;
                            }
                            None => {
                                bot.send_message(chat_id, "Address Not found").await?;
                            }
                        }
                    }
                }
            }
        }

        Ok::<_, Err>(())
    };

    let callback_handler = |bot: Bot,
                            db: Arc<DatabaseConnection>,
                            q: CallbackQuery,
                            chat_state: ChatState| async move {
        let from = q.from.id;
        match (q.chat_id(), q.data) {
            (Some(chat), Some(data)) => {
                let is_private = chat.0.is_positive() && u64::try_from(chat.0).unwrap() == from.0;

                if bot
                    .get_chat_member(chat, from)
                    .await?
                    .can_manage_chat()
                    .not()
                    && !is_private
                {
                    bot.answer_callback_query(q.id)
                        .text("Only group admin can do this")
                        .await?;
                    return Ok::<_, Err>(());
                }
                match CallbackCommand::from_string(data)? {
                    CallbackCommand::Unsubscribe(address) => {
                        match WalletTracked::find()
                            .filter(wallet_tracked::Column::ChatId.eq(chat.0))
                            .filter(wallet_tracked::Column::WalletId.eq(address.clone()))
                            .one(db.as_ref())
                            .await?
                        {
                            Some(entry) => {
                                wallet_tracked::Entity::delete_by_id(entry.wallet_id)
                                    .exec(db.as_ref())
                                    .await?;
                                bot.send_message(chat, format!("{address} Unsubscribed"))
                                    .await?;
                            }
                            None => {
                                bot.answer_callback_query(q.id)
                                    .text("Already Unsubscribed")
                                    .await?;
                            }
                        }
                    }
                    CallbackCommand::Update(id) => {
                        match WalletTracked::find_by_id(id).one(db.as_ref()).await? {
                            Some(wallet) => {
                                update_handler(bot.clone(), chat, id, wallet.wallet_address)
                                    .await?;
                                bot.answer_callback_query(q.id).text("200").await?;
                            }
                            None => {
                                bot.answer_callback_query(q.id)
                                    .text("Wallet not found")
                                    .await?;
                            }
                        }
                    }
                    CallbackCommand::UpdateNickname(address) => {
                        chat_state.update(State::UpdateNickname(address)).await?;
                        bot.send_message(chat, "send the nickname to the chat")
                            .await?;
                        bot.answer_callback_query(q.id).text("200").await?;
                    }
                    CallbackCommand::UpdateFilter(address) => {
                        if let Some(msg_id) = q.message {
                            let buttons = vec![
                                vec![
                                    InlineKeyboardButton::callback(
                                        "Every Tx",
                                        CallbackCommand::TrackFull(address.clone())
                                            .to_callback_data(),
                                    ),
                                    InlineKeyboardButton::callback(
                                        "Received",
                                        CallbackCommand::TrackReceive(address.clone())
                                            .to_callback_data(),
                                    ),
                                ],
                                vec![
                                    InlineKeyboardButton::callback(
                                        "Sent",
                                        CallbackCommand::TrackSent(address.clone())
                                            .to_callback_data(),
                                    ),
                                    InlineKeyboardButton::callback(
                                        "Received + Sent",
                                        CallbackCommand::TrackBalance(address.clone())
                                            .to_callback_data(),
                                    ),
                                ],
                            ];

                            let _msg = bot.edit_message_text(
                                chat,
                                msg_id.id,
                                "Select your filter mode\nEvery Tx: No filter\nReceived: Filter with received value in USD\nSent: Filter with sent value in USD\n",
                            ).reply_markup(InlineKeyboardMarkup::new(buttons)).await;
                            bot.answer_callback_query(q.id).text("200").await?;
                        }
                    }
                    CallbackCommand::TrackFull(address) => {
                        let _update = WalletTracked::update_many()
                            .col_expr(
                                wallet_tracked::Column::TrackType,
                                Expr::value(TrackType::Full),
                            )
                            .filter(wallet_tracked::Column::ChatId.eq(chat.0))
                            .filter(wallet_tracked::Column::WalletId.eq(address))
                            .exec(db.as_ref())
                            .await?;
                        bot.answer_callback_query(q.id)
                            .text("Filter Updated")
                            .await?;
                    }
                    CallbackCommand::TrackBalance(address) => {
                        let _update = WalletTracked::update_many()
                            .col_expr(
                                wallet_tracked::Column::TrackType,
                                Expr::value(TrackType::Balance),
                            )
                            .filter(wallet_tracked::Column::ChatId.eq(chat.0))
                            .filter(wallet_tracked::Column::WalletId.eq(address.clone()))
                            .exec(db.as_ref())
                            .await?;
                        bot.send_message(chat, "Send the USD value to chat ex: 46.8")
                            .await?;
                        chat_state.update(State::UpdateFilter(address)).await?;
                        bot.answer_callback_query(q.id).text("200").await?;
                    }
                    CallbackCommand::TrackReceive(address) => {
                        let _update = WalletTracked::update_many()
                            .col_expr(
                                wallet_tracked::Column::TrackType,
                                Expr::value(TrackType::Receive),
                            )
                            .filter(wallet_tracked::Column::ChatId.eq(chat.0))
                            .filter(wallet_tracked::Column::WalletId.eq(address.clone()))
                            .exec(db.as_ref())
                            .await?;
                        bot.send_message(chat, "Send the USD value to chat ex: 37.5")
                            .await?;
                        chat_state.update(State::UpdateFilter(address)).await?;
                        bot.answer_callback_query(q.id).text("200").await?;
                    }
                    CallbackCommand::TrackSent(address) => {
                        let _update = WalletTracked::update_many()
                            .col_expr(
                                wallet_tracked::Column::TrackType,
                                Expr::value(TrackType::Sent),
                            )
                            .filter(wallet_tracked::Column::ChatId.eq(chat.0))
                            .filter(wallet_tracked::Column::WalletId.eq(address.clone()))
                            .exec(db.as_ref())
                            .await?;
                        bot.send_message(chat, "Send the USD value to chat ex: 60.5")
                            .await?;
                        chat_state.update(State::UpdateFilter(address)).await?;
                        bot.answer_callback_query(q.id).text("200").await?;
                    }
                };
            }
            (Some(_chat), None) => {
                bot.answer_callback_query(q.id)
                    .text("Callback Error")
                    .await?;
            }
            (_, _) => {
                error!("Call back from unknown chat");
            }
        };
        Ok::<_, Err>(())
    };
    let update_nickname_handle = |bot: Bot,
                                  msg: Message,
                                  chat_state: ChatState,
                                  db: Arc<DatabaseConnection>,
                                  id: u64| async move {
        let chat_id = get_chat_id(bot.clone(), db.as_ref(), msg.chat.id).await?;
        match WalletTracked::find()
            .filter(wallet_tracked::Column::ChatId.eq(chat_id.0))
            .filter(wallet_tracked::Column::WalletId.eq(id.clone()))
            .one(db.as_ref())
            .await?
        {
            Some(entry) => {
                let mut data: wallet_tracked::ActiveModel = entry.into();
                let nickname = msg.text().map(|s| s.to_string());
                if let Some(nickname) = nickname.clone() {
                    if nickname.len() > 20 {
                        bot.send_message(chat_id, "Nickname to long, send another shorter one")
                            .await?;
                        return Ok::<_, Err>(());
                    }
                }
                data.nickname = Set(nickname);
                let result = data.update(db.as_ref()).await?;
                bot.send_message(
                    chat_id,
                    format!("nickname updated {}", result.wallet_address.clone()),
                )
                .await?;
            }
            None => {
                bot.send_message(chat_id, "Address Not found").await?;
            }
        }

        chat_state.update(State::Idle).await?;
        Ok::<_, Err>(())
    };
    let update_filter_handle = |bot: Bot,
                                msg: Message,
                                chat_state: ChatState,
                                db: Arc<DatabaseConnection>,
                                id: u64| async move {
        let chat_id = get_chat_id(bot.clone(), db.as_ref(), msg.chat.id).await?;
        match msg.text() {
            Some(msg_text) => {
                let parse = f64::from_str(msg_text);
                match parse {
                    Ok(value) => {
                        let _update = WalletTracked::update_many()
                            .col_expr(wallet_tracked::Column::MinimumValue, Expr::value(value))
                            .filter(wallet_tracked::Column::ChatId.eq(chat_id.0))
                            .filter(wallet_tracked::Column::WalletId.eq(id))
                            .exec(db.as_ref())
                            .await?;
                        bot.send_message(chat_id, "Filter updated").await?;
                        chat_state.update(State::Idle).await?;
                    }
                    Err(_err) => {
                        bot.send_message(chat_id, "number parse error, plz send a valid value")
                            .await?;
                    }
                }
            }
            None => {}
        }
        Ok::<_, Err>(())
    };
    let default_handler = |bot: Bot, msg: Message, db: Arc<DatabaseConnection>| async move {
        let chat_id = get_chat_id(bot.clone(), db.as_ref(), msg.clone().chat.id).await?;
        if let Some(text) = msg.text() {
            if text.to_string().contains("屁眼") {
                bot.send_message(chat_id, "哈哈屁眼")
                    .reply_to_message_id(msg.id)
                    .await?;
            }
        }
        Ok::<_, Err>(())
    };
    let admin_handler = |msg: Message, bot: Bot| async move {
        let msg = msg.clone();

        match msg.from() {
            Some(user) => {
                let member = bot.get_chat_member(msg.chat.id, user.id).await;
                if let Ok(member) = member {
                    return member.is_privileged() || msg.chat.is_private();
                }
                msg.chat.is_private()
            }
            None => false,
        }
    };
    let _register_handler = |msg: Message, _bot: Bot, db: Arc<DatabaseConnection>| async move {
        let from = match msg.from() {
            None => {
                return false;
            }
            Some(user) => user,
        };
        let _user_check = User::find_by_id(from.id.0).one(db.as_ref()).await;
        true
    };

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_async(admin_handler)
                .filter_command::<Command>()
                .endpoint(commands),
        )
        .branch(
            Update::filter_callback_query()
                .enter_dialogue::<CallbackQuery, InMemStorage<State>, State>()
                .endpoint(callback_handler),
        )
        .branch(
            Update::filter_message()
                .filter_async(admin_handler)
                .enter_dialogue::<Message, InMemStorage<State>, State>()
                .branch(case![State::Idle].endpoint(default_handler))
                .branch(case![State::UpdateNickname(address)].endpoint(update_nickname_handle))
                .branch(case![State::UpdateFilter(address)].endpoint(update_filter_handle)),
        );
    Dispatcher::builder(bot.clone(), handler)
        .dependencies(deps![
            db.clone(),
            rpc.clone(),
            new_token.clone(),
            tx_client.clone(),
            InMemStorage::<State>::new()
        ])
        .default_handler(|_| async {})
        .error_handler(HandleErr::new(
            db.clone(),
            bot.clone(),
            "bot_handle".to_string(),
        ))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn opt(input: String) -> Result<(Option<String>,), ParseError> {
    match input.split_whitespace().count() {
        0 => Ok((None,)),
        1 => Ok((Some(input.trim().to_owned()),)),
        n => Err(ParseError::TooManyArguments {
            expected: 1,
            found: n,
            message: String::from("Wrong number of arguments"),
        }),
    }
}

async fn update_handler(bot: Bot, chat_id: ChatId, id: u64, address: String) -> Result<(), Err> {
    let item = vec![
        InlineKeyboardButton::callback(
            "nickname",
            CallbackCommand::UpdateNickname(id.clone()).to_callback_data(),
        ),
        InlineKeyboardButton::callback(
            "filter",
            CallbackCommand::UpdateFilter(id.clone()).to_callback_data(),
        ),
    ];
    bot.send_message(chat_id, format!("Settings for {address}"))
        .reply_markup(InlineKeyboardMarkup::new([item]))
        .await?;
    Ok::<_, Err>(())
}

async fn get_chat_id(bot: Bot, db: &DatabaseConnection, chat_id: ChatId) -> Result<ChatId, Err> {
    let result = bot.get_chat(chat_id).await;
    match result {
        Ok(s) => Ok(s.id),
        Err(err) => match err {
            RequestError::MigrateToChatId(id) => {
                let _update = WalletTracked::update_many()
                    .col_expr(wallet_tracked::Column::ChatId, Expr::value(id))
                    .filter(wallet_tracked::Column::ChatId.eq(chat_id.0))
                    .exec(db)
                    .await?;
                Ok(ChatId(id))
            }
            _ => Err(Err::Bot(err)),
        },
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct ParsedSubscribe {
    address: OptString,
    nickname: OptString,
}
fn opt2(input: String) -> Result<(ParsedSubscribe,), ParseError> {
    let mut all = input.split_whitespace();
    Ok((ParsedSubscribe {
        address: all.next().map(str::to_string),
        nickname: all.next().map(str::to_string),
    },))
}
