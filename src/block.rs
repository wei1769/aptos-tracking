use std::sync::Arc;
use std::time::Duration;

use aptos_sdk::rest_client::Client;
use futures::{future::Pending, stream::Abortable};

use tokio::sync::RwLock;
pub async fn update_block_loop(
    abort: Abortable<Pending<()>>,
    slot: Arc<RwLock<u64>>,
    rpc: Arc<Client>,
) {
    loop {
        if abort.is_aborted() {
            break;
        }
        if let Ok(info) = rpc.get_ledger_information().await {
            let new_slot = info.inner().block_height;
            let mut update = slot.write().await;
            if new_slot > *update {
                *update = new_slot;
            }
            drop(update);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
