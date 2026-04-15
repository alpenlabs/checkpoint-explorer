use anyhow::Result;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::rpc_params;
use model::block::RpcBlockHeader;
use model::checkpoint::RpcOLChainStatus;
use tracing::error;

/// `StrataFetcher` struct for fetching checkpoint and block data
pub struct StrataFetcher {
    client: HttpClient,
}

impl StrataFetcher {
    /// Creates a new `StrataFetcher` instance.
    pub fn new(endpoint: String) -> Self {
        let client = HttpClientBuilder::default()
            .build(&endpoint)
            .expect("Failed to build HTTP client");
        Self { client }
    }

    /// Fetches detailed information for a given index (checkpoint or block).
    pub async fn fetch_data<T>(&self, method: &str, idx: u64) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.client
            .request(method, rpc_params![idx])
            .await
            .map_err(|e| {
                error!(method, idx, ?e, "RPC call failed");
                anyhow::anyhow!("RPC call failed: {:?}", e)
            })
    }

    /// Fetches the current OL chain status via strata_getChainStatus.
    pub async fn get_chain_status(&self) -> Result<RpcOLChainStatus> {
        self.client
            .request("strata_getChainStatus", rpc_params![])
            .await
            .map_err(|e| {
                error!(?e, "strata_getChainStatus failed");
                anyhow::anyhow!("strata_getChainStatus failed: {:?}", e)
            })
    }

    /// Fetches block headers for a slot range via strata_getHeadersInRange.
    pub async fn fetch_headers_in_range(
        &self,
        start: u64,
        end: u64,
    ) -> Result<Vec<RpcBlockHeader>> {
        self.client
            .request("strata_getHeadersInRange", rpc_params![start, end])
            .await
            .map_err(|e| {
                error!(start, end, ?e, "strata_getHeadersInRange failed");
                anyhow::anyhow!("strata_getHeadersInRange failed: {:?}", e)
            })
    }
}
