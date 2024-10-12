#![allow(clippy::all, warnings)]
pub struct TransactionsQuery;
pub mod transactions_query {
    #![allow(dead_code)]
    use std::result::Result;
    pub const OPERATION_NAME: &str = "TransactionsQuery";
    pub const QUERY : & str = "query TransactionsQuery($_gte: bigint, $_lte: bigint) {\n  fungible_asset_activities(\n    where: { transaction_version: { _gte: $_gte, _lte: $_lte } }\n  ) {\n    amount\n    asset_type\n    metadata {\n      decimals\n      name\n      symbol\n      token_standard\n      asset_type\n    }\n    transaction_version\n    is_transaction_success\n    owner_address\n    type\n    event_index\n    token_standard\n  }\n}\n\nquery TransactionQuery($_eq: bigint) {\n  fungible_asset_activities(\n    where: { transaction_version: { _eq: $_eq } }\n    order_by: { event_index: asc }\n  ) {\n    amount\n    asset_type\n    metadata {\n      decimals\n      name\n      symbol\n      token_standard\n      asset_type\n    }\n    transaction_version\n    is_transaction_success\n    owner_address\n    type\n    event_index\n    token_standard\n  }\n}\n" ;
    use super::*;
    use serde::{Deserialize, Serialize};
    #[allow(dead_code)]
    type Boolean = bool;
    #[allow(dead_code)]
    type Float = f64;
    #[allow(dead_code)]
    type Int = u8;
    #[allow(dead_code)]
    type ID = String;
    type bigint = i128;
    type numeric = i128;
    #[derive(Serialize, Clone, Debug)]
    pub struct Variables {
        #[serde(rename = "_gte")]
        pub gte: Option<bigint>,
        #[serde(rename = "_lte")]
        pub lte: Option<bigint>,
    }
    impl Variables {}
    #[derive(Deserialize, Clone, Debug)]
    pub struct ResponseData {
        pub fungible_asset_activities: Vec<TransactionsQueryFungibleAssetActivities>,
    }
    #[derive(Deserialize, Clone, Debug)]
    pub struct TransactionsQueryFungibleAssetActivities {
        pub amount: Option<numeric>,
        pub asset_type: Option<String>,
        pub metadata: Option<TransactionsQueryFungibleAssetActivitiesMetadata>,
        pub transaction_version: bigint,
        pub is_transaction_success: Boolean,
        pub owner_address: Option<String>,
        #[serde(rename = "type")]
        pub type_: String,
        pub event_index: bigint,
        pub token_standard: String,
    }
    #[derive(Deserialize, Clone, Debug)]
    pub struct TransactionsQueryFungibleAssetActivitiesMetadata {
        pub decimals: Int,
        pub name: String,
        pub symbol: String,
        pub token_standard: String,
        pub asset_type: String,
    }
}
impl graphql_client::GraphQLQuery for TransactionsQuery {
    type Variables = transactions_query::Variables;
    type ResponseData = transactions_query::ResponseData;
    fn build_query(variables: Self::Variables) -> ::graphql_client::QueryBody<Self::Variables> {
        graphql_client::QueryBody {
            variables,
            query: transactions_query::QUERY,
            operation_name: transactions_query::OPERATION_NAME,
        }
    }
}
pub struct TransactionQuery;
pub mod transaction_query {
    #![allow(dead_code)]
    use std::result::Result;
    pub const OPERATION_NAME: &str = "TransactionQuery";
    pub const QUERY : & str = "query TransactionsQuery($_gte: bigint, $_lte: bigint) {\n  fungible_asset_activities(\n    where: { transaction_version: { _gte: $_gte, _lte: $_lte } }\n  ) {\n    amount\n    asset_type\n    metadata {\n      decimals\n      name\n      symbol\n      token_standard\n      asset_type\n    }\n    transaction_version\n    is_transaction_success\n    owner_address\n    type\n    event_index\n    token_standard\n  }\n}\n\nquery TransactionQuery($_eq: bigint) {\n  fungible_asset_activities(\n    where: { transaction_version: { _eq: $_eq } }\n    order_by: { event_index: asc }\n  ) {\n    amount\n    asset_type\n    metadata {\n      decimals\n      name\n      symbol\n      token_standard\n      asset_type\n    }\n    transaction_version\n    is_transaction_success\n    owner_address\n    type\n    event_index\n    token_standard\n  }\n}\n" ;
    use super::*;
    use serde::{Deserialize, Serialize};
    #[allow(dead_code)]
    type Boolean = bool;
    #[allow(dead_code)]
    type Float = f64;
    #[allow(dead_code)]
    type Int = u8;
    #[allow(dead_code)]
    type ID = String;
    type bigint = i128;
    type numeric = i128;
    #[derive(Serialize, Clone, Debug)]
    pub struct Variables {
        #[serde(rename = "_eq")]
        pub eq: Option<bigint>,
    }
    impl Variables {}
    #[derive(Deserialize, Clone, Debug)]
    pub struct ResponseData {
        pub fungible_asset_activities: Vec<TransactionQueryFungibleAssetActivities>,
    }
    #[derive(Deserialize, Clone, Debug)]
    pub struct TransactionQueryFungibleAssetActivities {
        pub amount: Option<numeric>,
        pub asset_type: Option<String>,
        pub metadata: Option<TransactionQueryFungibleAssetActivitiesMetadata>,
        pub transaction_version: bigint,
        pub is_transaction_success: Boolean,
        pub owner_address: Option<String>,
        #[serde(rename = "type")]
        pub type_: String,
        pub event_index: bigint,
        pub token_standard: String,
    }
    #[derive(Deserialize, Clone, Debug)]
    pub struct TransactionQueryFungibleAssetActivitiesMetadata {
        pub decimals: Int,
        pub name: String,
        pub symbol: String,
        pub token_standard: String,
        pub asset_type: String,
    }
}
impl graphql_client::GraphQLQuery for TransactionQuery {
    type Variables = transaction_query::Variables;
    type ResponseData = transaction_query::ResponseData;
    fn build_query(variables: Self::Variables) -> ::graphql_client::QueryBody<Self::Variables> {
        graphql_client::QueryBody {
            variables,
            query: transaction_query::QUERY,
            operation_name: transaction_query::OPERATION_NAME,
        }
    }
}
