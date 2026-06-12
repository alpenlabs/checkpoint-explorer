use database::connection::DatabaseWrapper;
use database::services::block_service::BlockService;
use fullnode_client::fetcher::StrataFetcher;
use model::block::RpcBlockHeader;
use model::checkpoint::L2BlockFetchTarget;
use std::sync::Arc;
use tokio::sync::watch::Receiver;
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};

const MAX_HEADERS_RANGE: u64 = 5000;
const BLOCK_FETCH_CONCURRENCY: usize = 20;

pub async fn run_block_fetcher(
    fetcher: Arc<StrataFetcher>,
    database: Arc<DatabaseWrapper>,
    mut rx: Receiver<L2BlockFetchTarget>,
) {
    info!("Starting block fetcher...");
    loop {
        if rx.changed().await.is_err() {
            break;
        }
        let target = *rx.borrow_and_update();
        fetch_blocks(fetcher.clone(), database.clone(), target).await;
    }
}

async fn fetch_blocks(fetcher: Arc<StrataFetcher>, database: Arc<DatabaseWrapper>, target: u64) {
    let block_db = BlockService::new(&database.db);
    let start = block_db
        .get_latest_block_index()
        .await
        .map(|h| h + 1)
        .unwrap_or(0);

    if start > target {
        info!("No blocks to fetch");
        return;
    }

    let ranges = block_fetch_ranges(start, target);
    info!(
        start,
        target,
        chunks = ranges.len(),
        concurrency = BLOCK_FETCH_CONCURRENCY,
        "Fetching blocks"
    );

    let mut cursor = start;
    for wave in ranges.chunks(BLOCK_FETCH_CONCURRENCY) {
        let wave_end = wave.last().map(|(_, end)| *end).unwrap_or(cursor);
        let fetch_result = fetch_header_wave(fetcher.clone(), wave).await;

        let mut prefix = extract_contiguous_prefix(fetch_result.headers, cursor);
        let prefix_len = prefix.len();

        if prefix.is_empty() {
            warn!(
                cursor,
                wave_end,
                failed_chunks = fetch_result.failed_chunks,
                "No contiguous block headers fetched"
            );
            return;
        }

        let last_prefix_slot = prefix.last().map(|header| header.slot).unwrap_or(cursor);
        debug!(
            cursor,
            wave_end,
            last_prefix_slot,
            prefix_len,
            failed_chunks = fetch_result.failed_chunks,
            "Fetched contiguous block header prefix"
        );

        for header in prefix.drain(..) {
            let checkpoint_idx = header.epoch;
            block_db.insert_block(header, checkpoint_idx).await;
        }

        if fetch_result.failed_chunks > 0 || last_prefix_slot < wave_end {
            info!(
                next_start = last_prefix_slot + 1,
                target,
                failed_chunks = fetch_result.failed_chunks,
                "Stopping block fetch after partial progress"
            );
            return;
        }

        cursor = last_prefix_slot + 1;
    }
}

/// Result of fetching block header ranges.
struct HeaderFetchResult {
    /// Headers returned by successful range requests; not guaranteed to be ordered or contiguous.
    headers: Vec<RpcBlockHeader>,

    /// Number of ranges that failed before returning headers.
    failed_chunks: usize,
}

/// Fetches one bounded wave of ranges concurrently and returns all successful headers.
async fn fetch_header_wave(
    fetcher: Arc<StrataFetcher>,
    ranges: &[(u64, u64)],
) -> HeaderFetchResult {
    let mut tasks = JoinSet::new();

    for &(start, end) in ranges {
        let fetcher = fetcher.clone();
        tasks.spawn(async move {
            fetcher
                .fetch_headers_in_range(start, end)
                .await
                .map(|headers| (start, end, headers))
                .map_err(|err| (start, end, err))
        });
    }

    let mut headers = Vec::new();
    let mut failed_chunks = 0;

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok((start, end, mut chunk_headers))) => {
                debug!(
                    start,
                    end,
                    headers = chunk_headers.len(),
                    "Fetched block header chunk"
                );
                headers.append(&mut chunk_headers);
            }
            Ok(Err((start, end, err))) => {
                failed_chunks += 1;
                error!(?err, start, end, "Failed to fetch block headers");
            }
            Err(err) => {
                failed_chunks += 1;
                error!(?err, "Block header fetch task failed");
            }
        }
    }

    HeaderFetchResult {
        headers,
        failed_chunks,
    }
}

/// Splits an inclusive block range into non-overlapping RPC ranges.
fn block_fetch_ranges(start: u64, target: u64) -> Vec<(u64, u64)> {
    let mut ranges = Vec::new();
    let mut cursor = start;

    while cursor <= target {
        let end = (cursor + MAX_HEADERS_RANGE - 1).min(target);
        ranges.push((cursor, end));
        cursor = end + 1;
    }

    ranges
}

/// Extracts the longest ordered block sequence beginning at start.
fn extract_contiguous_prefix(mut headers: Vec<RpcBlockHeader>, start: u64) -> Vec<RpcBlockHeader> {
    headers.sort_by_key(|header| header.slot);

    let mut expected_slot = start;
    let mut prefix = Vec::new();

    for header in headers {
        if header.slot < expected_slot {
            continue;
        }

        if header.slot != expected_slot {
            break;
        }

        expected_slot += 1;
        prefix.push(header);
    }

    prefix
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(slot: u64) -> RpcBlockHeader {
        RpcBlockHeader {
            slot,
            epoch: 0,
            blkid: format!("{slot:064x}"),
            timestamp: 0,
            parent_blkid: format!("{:064x}", slot.saturating_sub(1)),
            state_root: format!("{slot:064x}"),
            body_root: format!("{slot:064x}"),
            logs_root: format!("{slot:064x}"),
            is_terminal: false,
        }
    }

    #[test]
    fn creates_singleton_range() {
        assert_eq!(block_fetch_ranges(0, 0), vec![(0, 0)]);
    }

    #[test]
    fn creates_single_range_below_limit() {
        assert_eq!(block_fetch_ranges(10, 20), vec![(10, 20)]);
    }

    #[test]
    fn splits_range_above_limit() {
        assert_eq!(
            block_fetch_ranges(0, MAX_HEADERS_RANGE),
            vec![
                (0, MAX_HEADERS_RANGE - 1),
                (MAX_HEADERS_RANGE, MAX_HEADERS_RANGE)
            ]
        );
    }

    #[test]
    fn extracts_contiguous_prefix_from_sorted_headers() {
        let prefix = extract_contiguous_prefix(vec![header(5), header(6), header(7)], 5);
        let slots: Vec<_> = prefix.into_iter().map(|header| header.slot).collect();

        assert_eq!(slots, vec![5, 6, 7]);
    }

    #[test]
    fn extracts_contiguous_prefix_from_unsorted_headers() {
        let prefix = extract_contiguous_prefix(vec![header(7), header(5), header(6)], 5);
        let slots: Vec<_> = prefix.into_iter().map(|header| header.slot).collect();

        assert_eq!(slots, vec![5, 6, 7]);
    }

    #[test]
    fn stops_prefix_at_gap() {
        let prefix = extract_contiguous_prefix(vec![header(5), header(6), header(8)], 5);
        let slots: Vec<_> = prefix.into_iter().map(|header| header.slot).collect();

        assert_eq!(slots, vec![5, 6]);
    }

    #[test]
    fn returns_empty_prefix_when_start_is_missing() {
        let prefix = extract_contiguous_prefix(vec![header(6), header(7)], 5);

        assert!(prefix.is_empty());
    }

    #[test]
    fn does_not_require_prefix_to_reach_target() {
        let prefix = extract_contiguous_prefix(vec![header(5), header(6)], 5);
        let slots: Vec<_> = prefix.into_iter().map(|header| header.slot).collect();

        assert_eq!(slots, vec![5, 6]);
    }

    #[test]
    fn skips_duplicate_slots_in_prefix() {
        let prefix = extract_contiguous_prefix(vec![header(5), header(5), header(6)], 5);
        let slots: Vec<_> = prefix.into_iter().map(|header| header.slot).collect();

        assert_eq!(slots, vec![5, 6]);
    }
}
