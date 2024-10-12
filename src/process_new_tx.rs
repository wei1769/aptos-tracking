// use crate::tx::{WalletLastTxEntry};
// use crate::{
//     entities::{
//         prelude::*,
//     },
// };

// pub struct TxInfos {
//     pub txs: Vec<(String, u64)>,
//     pub address: String,
//     pub tx_entry: WalletLastTxEntry,
//     pub last_tx: (String, u64),
//     pub all_chat: Vec<WalletTracked>,
// }
// const RETRIES: usize = 5;
// pub async fn new_tx(
//     abort: Abortable<Pending<()>>,
//     db: &DatabaseConnection,
//     bot: Bot,
//     mut tx_sender: Receiver<TxInfos>,
//     rpc: Arc<Client>,
// ) {
//     loop {
//         match tx_sender.recv().await {
//             Some(txs) => {
//                 let mut last_ok_tx: (String, u64) = txs.last_tx;

//                 'outer: for tx in txs.txs.into_iter().rev() {
//                     for index in 0..RETRIES {
//                         match handle_new_tx(
//                             db,
//                             &bot,
//                             &rpc,
//                             tx.clone(),
//                             txs.address.clone(),
//                             txs.all_chat.clone(),
//                         )
//                         .await
//                         {
//                             Ok(_) => {
//                                 last_ok_tx = tx;
//                                 break;
//                             }
//                             Err(e) => {
//                                 if let Err::Db(err) = e {
//                                     warn!("db error, {:?}", err)
//                                 }
//                             }
//                         }
//                         if index == RETRIES - 1 {
//                             for _ in 0..RETRIES {
//                                 let update = wallet_last_tx::ActiveModel {
//                                     wallet_address: Set(tx.clone().0.clone()),
//                                     history_id: Set(txs.txEntry.0),
//                                     tx_sig: Set(Some(last_ok_tx.clone().0)),
//                                     last_slot: Set(Some(last_ok_tx.clone().1)),
//                                     ..Default::default()
//                                 };
//                                 match update.update(db).await {
//                                     Err(err) => {
//                                         warn!("db error, {:?}", err)
//                                     }
//                                     Ok(_) => {
//                                         break;
//                                     }
//                                 };
//                             }

//                             break 'outer;
//                         }
//                     }
//                 }
//             }
//             None => {
//                 if abort.is_aborted() {
//                     info!("abort");
//                     break;
//                 }
//             }
//         }
//     }
// }

// async fn handle_new_tx(
//     db: &DatabaseConnection,
//     bot: &Bot,
//     rpc: &RpcClient,
//     tx: (String, u64),
//     address: String,
//     all_chat: Vec<WalletTracked>,
// ) -> Result<(), Err> {
//     Ok(())
// }
