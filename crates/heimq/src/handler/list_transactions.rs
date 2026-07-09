//! ListTransactions request handler (API Key 66)
//!
//! Returns an empty list — the in-memory backend does not track transaction
//! state beyond what is needed to fence duplicate producers.

use crate::error::Result;
use heimq_protocol::messages::ListTransactionsResponse;

pub fn handle(_api_version: i16, _body: &[u8]) -> Result<ListTransactionsResponse> {
    let mut response = ListTransactionsResponse::default();
    response.error_code = 0;
    response.transaction_states = vec![];
    Ok(response)
}
