use crate::db::TokenMap;
use crate::error::Err;
use crate::query::transaction_query::{self, ResponseData};
use crate::query::transactions_query::{self, TransactionsQueryFungibleAssetActivities, Variables};
use crate::query::{TransactionQuery, TransactionsQuery};
use aptos_sdk::crypto::HashValue;
use aptos_sdk::move_types::account_address::AccountAddress;
use aptos_sdk::rest_client::aptos_api_types::{TransactionData, TransactionOnChainData};
use aptos_sdk::rest_client::Client;
use aptos_sdk::types::bytes;
use graphql_client::{reqwest::post_graphql, GraphQLQuery};
use log::warn;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use reqwest::Client as rqwClient;
use sqlx::Transaction;
use std::collections::HashMap;
use std::ops::Sub;
use std::option::Option;
use std::str::FromStr;
use std::sync::Arc;
#[derive(Debug, Clone)]
pub struct TxQueryClient {
    pub client: Arc<rqwClient>,
    pub url: String,
}
impl TxQueryClient {
    pub async fn get_tx_by_version(
        &self,
        version: u64,
    ) -> Result<graphql_client::Response<transaction_query::ResponseData>, reqwest::Error> {
        let version_bytes = version.to_le_bytes();

        let version = i128::from_le_bytes(concat_arrays(version_bytes, 0u64.to_le_bytes()));
        post_graphql::<TransactionQuery, _>(
            self.client.as_ref(),
            self.url.clone(),
            transaction_query::Variables { eq: Some(version) },
        )
        .await
    }
    pub async fn get_tx_in_range(
        &self,
        variables: Variables,
    ) -> Result<graphql_client::Response<transactions_query::ResponseData>, reqwest::Error> {
        post_graphql::<TransactionsQuery, _>(self.client.as_ref(), self.url.clone(), variables)
            .await
    }
}

// pub type WalletHistory = HashMap<String, WalletLastTxEntry>;

// pub type WalletLastTxEntry = (u64, Option<String>, Option<u64>);

// pub async fn fetch_txs(
//     address: String,
//     rpc: Arc<Client>,
//     amount: u16,
//     before: Option<String>,
//     until: Option<String>,
// ) -> Result<Vec<(String, u64)>, Err> {
//     Ok(rpc
//         .get_signatures_for_address_with_config(
//             &AccountAddress::from_str(&address)?,
//             GetConfirmedSignaturesForAddress2Config {
//                 before: match before {
//                     Some(s) => Some(Signature::from_str(&s)?),
//                     None => None,
//                 },
//                 until: match until {
//                     Some(s) => Some(Signature::from_str(&s)?),
//                     None => None,
//                 },
//                 limit: Some(amount.into()),
//                 commitment: Some(CommitmentConfig {
//                     commitment: CommitmentLevel::Finalized,
//                 }),
//                 ..Default::default()
//             },
//         )
//         .await?
//         .into_iter()
//         .filter(|r| r.err.is_none())
//         .map(|r| (r.signature, r.slot))
//         .collect::<Vec<(String, u64)>>())
// }

pub async fn get_tx_detail(
    conn: &TxQueryClient,
    version: u64,
) -> Result<Option<Vec<BalanceChange>>, Err> {
    let info = conn.get_tx_by_version(version).await?;
    match info.data {
        Some(tx) => Ok(BalanceChange::from_indexer_response(tx)),
        None => {
            return Ok(None);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BalanceChange {
    change: i128,
    address: AccountAddress,
    token: Token,
}

impl Default for BalanceChange {
    fn default() -> Self {
        BalanceChange {
            address: AccountAddress::ZERO,
            change: i128::default(),
            token: Token::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Token {
    #[default]
    Native,
    TA(TokenAccount),
}
#[derive(Debug, Clone, Copy)]
pub struct TokenAccount {
    token_address: AccountAddress,
    decimal: u8,
    owner: AccountAddress,
}

impl Default for TokenAccount {
    fn default() -> Self {
        TokenAccount {
            token_address: AccountAddress::ZERO,
            decimal: u8::default(),
            owner: AccountAddress::ZERO,
        }
    }
}

impl BalanceChange {
    pub fn to_usd_change(&self, token_map: &TokenMap) -> Option<f64> {
        if let Some(entry) = token_map.get(&self.to_token_address()) {
            if let Ok(change) = f64::from_str(&self.change.to_string()) {
                let decimal = 10_f64.powi(6);
                let value = entry.value * change / decimal;
                return Some(value);
            };
        }
        None
    }
    pub fn to_priced_string(&self, token_map: &TokenMap) -> String {
        match token_map.get(&self.to_token_address()) {
            Some(entry) => {
                let token_name = match entry.clone().name {
                    Some(s) => s,
                    None => self.to_short_token_address(),
                };
                match self.change.is_positive() {
                    true => {
                        format!(
                            "{} Receive {} {}",
                            self.to_short_owner(),
                            self.to_ui_amount(),
                            token_name
                        )
                    }
                    false => {
                        format!(
                            "{} Sent {} {}",
                            self.to_short_owner(),
                            self.to_ui_amount(),
                            token_name
                        )
                    }
                }
            }
            None => self.to_string(),
        }
    }
    pub fn to_string(&self) -> String {
        match self.change.is_positive() {
            true => {
                format!(
                    "{} Receive {} {}",
                    self.to_short_owner(),
                    self.to_ui_amount(),
                    self.to_short_token_address()
                )
            }
            false => {
                format!(
                    "{} Sent {} {}",
                    self.to_short_owner(),
                    self.to_ui_amount(),
                    self.to_short_token_address()
                )
            }
        }
    }
    pub fn to_ui_amount(&self) -> String {
        match self.token {
            Token::Native => Decimal {
                amount: self.change.abs(),
                decimal: 8,
            },
            Token::TA(a) => Decimal {
                amount: self.change.abs(),
                decimal: a.decimal,
            },
        }
        .to_string()
    }
    pub fn to_token_address(&self) -> String {
        match self.token {
            Token::Native => "0x1::aptos_coin::AptosCoin".to_string(),
            Token::TA(s) => {
                format!("{}", s.token_address)
            }
        }
    }
    pub fn to_short_token_address(&self) -> String {
        match self.token {
            Token::Native => "APT".to_string(),
            Token::TA(s) => {
                let address = s.token_address.to_string();
                let first = address[..5].to_string();
                let last = address[address.len() - 5..].to_string();
                format!("{first}...{last}",)
            }
        }
    }
    pub fn _to_owner(&self) -> String {
        match self.token {
            Token::Native => {
                format!("{}", self.address)
            }
            Token::TA(s) => {
                format!("{}", s.owner)
            }
        }
    }
    pub fn to_short_owner(&self) -> String {
        let address = match self.token {
            Token::Native => {
                format!("{}", self.address)
            }
            Token::TA(s) => {
                format!("{}", s.owner)
            }
        };
        let first = address[..5].to_string();
        let last = address[address.len() - 5..].to_string();
        format!("{first}...{last}",)
    }

    pub fn from_indexer_response(res: ResponseData) -> Option<Vec<BalanceChange>> {
        if res.fungible_asset_activities.len() == 0 {
            return None;
        }
        let result: Vec<BalanceChange> = res
            .fungible_asset_activities
            .clone()
            .into_par_iter()
            .filter_map(|transfer| {
                match (transfer.metadata, transfer.owner_address, transfer.amount) {
                    (Some(metadata), Some(owner), Some(amount)) => Self::from_event(
                        transfer.type_,
                        amount,
                        metadata.decimals,
                        AccountAddress::from_str(&owner).unwrap(),
                        metadata.asset_type,
                    ),
                    _ => return None,
                }
            })
            .collect();
        if result.len() == 0 {
            return None;
        }
        Some(result)
    }
    pub fn from_events(
        events: Vec<TransactionsQueryFungibleAssetActivities>,
    ) -> Option<Vec<BalanceChange>> {
        let result: Vec<BalanceChange> = events
            .into_par_iter()
            .filter_map(|transfer| {
                match (transfer.metadata, transfer.owner_address, transfer.amount) {
                    (Some(metadata), Some(owner), Some(amount)) => Self::from_event(
                        transfer.type_,
                        amount,
                        metadata.decimals,
                        AccountAddress::from_str(&owner).unwrap(),
                        metadata.asset_type,
                    ),
                    _ => return None,
                }
            })
            .collect();
        if result.len() == 0 {
            return None;
        }
        Some(result)
    }

    pub fn from_event(
        event_type: String,
        amount: i128,
        decimal: u8,
        owner: AccountAddress,
        asset_type: String,
    ) -> Option<Self> {
        match event_type.as_str() {
            "0x1::aptos_coin::GasFeeEvent" => Some(BalanceChange {
                address: owner,
                change: amount * -1,
                token: Token::Native,
            }),
            "0x1::coin::WithdrawEvent" => Some(BalanceChange {
                address: owner,
                change: amount * -1,
                token: Token::Native,
            }),
            "0x1::coin::DepositEvent" => Some(BalanceChange {
                address: owner,
                change: amount,
                token: Token::Native,
            }),
            // "0x1::fungible_asset::Deposit" => {}
            // "0x1::fungible_asset::Withdraw" => {}
            _ => None,
        }
    }
}
pub trait Filter {
    fn get_by_key(self, key: &AccountAddress) -> Self;
}
impl Filter for Vec<BalanceChange> {
    fn get_by_key(self, key: &AccountAddress) -> Self {
        self.into_iter()
            .filter(|i| match (i.token, i.address.eq(key)) {
                (_, true) => true,
                (Token::TA(t), false) => t.owner.eq(key),
                (Token::Native, false) => false,
            })
            .collect::<Self>()
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct ReplyInfo {
    pub value: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Decimal {
    amount: i128,
    decimal: u8,
}
impl Decimal {
    pub fn to_string(&self) -> String {
        let decimal: usize = self.decimal.into();
        //9
        let lead_len = decimal + 1;
        //10
        let mut filled = format!("{:0>lead_len$}", self.amount);

        filled.insert(filled.len() - decimal, '.');

        filled
    }
}

fn concat_arrays<T, const A: usize, const B: usize, const C: usize>(
    a: [T; A],
    b: [T; B],
) -> [T; C] {
    assert_eq!(A + B, C);
    let mut iter = a.into_iter().chain(b);
    std::array::from_fn(|_| iter.next().unwrap())
}
pub fn u64_to_i128(t: u64) -> i128 {
    let bytes = t.to_le_bytes();
    i128::from_le_bytes(concat_arrays(bytes, 0u64.to_le_bytes()))
}
