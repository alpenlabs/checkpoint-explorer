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

/// Inclusive block header range requested from the fullnode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct HeaderRange {
    start: u64,
    end: u64,
}

/// Headers returned for one requested range.
#[derive(Debug)]
struct FetchedHeaderChunk {
    range: HeaderRange,
    headers: Vec<RpcBlockHeader>,
}

/// Requested range that failed before returning headers.
#[derive(Debug)]
struct FailedHeaderChunk {
    range: HeaderRange,
}

/// Result of fetching block header ranges.
#[derive(Debug)]
struct HeaderFetchReport {
    fetched: Vec<FetchedHeaderChunk>,
    failed: Vec<FailedHeaderChunk>,
    task_failures: usize,
}

impl HeaderFetchReport {
    /// Number of failed range requests or failed fetch tasks in this report.
    fn failure_count(&self) -> usize {
        self.failed.len() + self.task_failures
    }
}

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

    let ranges = build_block_fetch_ranges(start, target);
    info!(
        start,
        target,
        chunks = ranges.len(),
        concurrency = BLOCK_FETCH_CONCURRENCY,
        "Fetching blocks"
    );

    let mut cursor = start;
    for wave in ranges.chunks(BLOCK_FETCH_CONCURRENCY) {
        let wave_end = wave.last().map(|range| range.end).unwrap_or(cursor);
        let fetch_report = fetch_header_wave(fetcher.clone(), wave).await;
        let failed_chunks = fetch_report.failure_count();
        let failed_ranges: Vec<_> = fetch_report
            .failed
            .iter()
            .map(|chunk| chunk.range)
            .collect();

        let mut insertable_headers = collect_insertable_headers(fetch_report.fetched, cursor);
        let insertable_header_count = insertable_headers.len();

        if insertable_headers.is_empty() {
            warn!(
                cursor,
                wave_end,
                failed_chunks,
                ?failed_ranges,
                "No insertable block headers fetched"
            );
            return;
        }

        let last_insertable_slot = insertable_headers
            .last()
            .map(|header| header.slot)
            .unwrap_or(cursor);
        debug!(
            cursor,
            wave_end,
            last_insertable_slot,
            insertable_header_count,
            failed_chunks,
            "Fetched insertable block headers"
        );

        for header in insertable_headers.drain(..) {
            let checkpoint_idx = header.epoch;
            block_db.insert_block(header, checkpoint_idx).await;
        }

        if failed_chunks > 0 || last_insertable_slot < wave_end {
            info!(
                next_start = last_insertable_slot + 1,
                target,
                failed_chunks,
                ?failed_ranges,
                "Stopping block fetch after partial progress"
            );
            return;
        }

        cursor = last_insertable_slot + 1;
    }
}

/// Fetches one bounded wave of ranges concurrently and returns all successful headers.
async fn fetch_header_wave(
    fetcher: Arc<StrataFetcher>,
    ranges: &[HeaderRange],
) -> HeaderFetchReport {
    let mut tasks = JoinSet::new();

    for &range in ranges {
        let fetcher = fetcher.clone();
        tasks.spawn(async move {
            fetcher
                .fetch_headers_in_range(range.start, range.end)
                .await
                .map(|headers| FetchedHeaderChunk { range, headers })
                .map_err(|err| (range, err))
        });
    }

    let mut fetched = Vec::new();
    let mut failed = Vec::new();
    let mut task_failures = 0;

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(chunk)) => {
                debug!(
                    start = chunk.range.start,
                    end = chunk.range.end,
                    headers = chunk.headers.len(),
                    "Fetched block header chunk"
                );
                fetched.push(chunk);
            }
            Ok(Err((range, err))) => {
                error!(
                    ?err,
                    start = range.start,
                    end = range.end,
                    "Failed to fetch block headers"
                );
                failed.push(FailedHeaderChunk { range });
            }
            Err(err) => {
                task_failures += 1;
                error!(?err, "Block header fetch task failed");
            }
        }
    }

    HeaderFetchReport {
        fetched,
        failed,
        task_failures,
    }
}

/// Splits an inclusive block range into non-overlapping RPC ranges.
fn build_block_fetch_ranges(start: u64, target: u64) -> Vec<HeaderRange> {
    let mut ranges = Vec::new();
    let mut cursor = start;

    while cursor <= target {
        let end = (cursor + MAX_HEADERS_RANGE - 1).min(target);
        ranges.push(HeaderRange { start: cursor, end });
        cursor = end + 1;
    }

    ranges
}

/// Collects headers from adjacent chunks beginning at start.
fn collect_insertable_headers(
    mut chunks: Vec<FetchedHeaderChunk>,
    start: u64,
) -> Vec<RpcBlockHeader> {
    chunks.sort_by_key(|chunk| chunk.range);

    let mut expected_slot = start;
    let mut insertable_headers = Vec::new();

    for mut chunk in chunks {
        if chunk.range.start != expected_slot {
            break;
        }

        let Some(first_header) = chunk.headers.first() else {
            break;
        };

        if first_header.slot != expected_slot {
            break;
        }

        let last_header_slot = chunk
            .headers
            .last()
            .map(|header| header.slot)
            .unwrap_or(expected_slot);

        if last_header_slot > chunk.range.end {
            break;
        }

        insertable_headers.append(&mut chunk.headers);

        if last_header_slot < chunk.range.end {
            break;
        }

        expected_slot = last_header_slot + 1;
    }

    insertable_headers
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
        assert_eq!(build_block_fetch_ranges(0, 0), vec![range(0, 0)]);
    }

    #[test]
    fn creates_single_range_below_limit() {
        assert_eq!(build_block_fetch_ranges(10, 20), vec![range(10, 20)]);
    }

    #[test]
    fn splits_range_above_limit() {
        assert_eq!(
            build_block_fetch_ranges(0, MAX_HEADERS_RANGE),
            vec![
                range(0, MAX_HEADERS_RANGE - 1),
                range(MAX_HEADERS_RANGE, MAX_HEADERS_RANGE)
            ]
        );
    }

    fn range(start: u64, end: u64) -> HeaderRange {
        HeaderRange { start, end }
    }

    fn chunk(start: u64, end: u64, slots: &[u64]) -> FetchedHeaderChunk {
        FetchedHeaderChunk {
            range: range(start, end),
            headers: slots.iter().map(|slot| header(*slot)).collect(),
        }
    }

    fn slots(headers: Vec<RpcBlockHeader>) -> Vec<u64> {
        headers.into_iter().map(|header| header.slot).collect()
    }

    #[test]
    fn collects_insertable_headers_from_adjacent_chunks() {
        let insertable_headers =
            collect_insertable_headers(vec![chunk(5, 6, &[5, 6]), chunk(7, 8, &[7, 8])], 5);

        assert_eq!(slots(insertable_headers), vec![5, 6, 7, 8]);
    }

    #[test]
    fn collects_insertable_headers_from_chunks_completed_out_of_order() {
        let insertable_headers =
            collect_insertable_headers(vec![chunk(7, 8, &[7, 8]), chunk(5, 6, &[5, 6])], 5);

        assert_eq!(slots(insertable_headers), vec![5, 6, 7, 8]);
    }

    #[test]
    fn stops_before_missing_chunk() {
        let insertable_headers =
            collect_insertable_headers(vec![chunk(5, 6, &[5, 6]), chunk(9, 10, &[9, 10])], 5);

        assert_eq!(slots(insertable_headers), vec![5, 6]);
    }

    #[test]
    fn returns_empty_when_starting_chunk_is_missing() {
        let insertable_headers = collect_insertable_headers(vec![chunk(6, 7, &[6, 7])], 5);

        assert!(insertable_headers.is_empty());
    }

    #[test]
    fn preserves_short_read_insertable_headers() {
        let insertable_headers = collect_insertable_headers(vec![chunk(5, 9, &[5, 6, 7])], 5);

        assert_eq!(slots(insertable_headers), vec![5, 6, 7]);
    }

    #[test]
    fn returns_empty_when_chunk_headers_do_not_start_at_cursor() {
        let insertable_headers = collect_insertable_headers(vec![chunk(5, 7, &[6, 7])], 5);

        assert!(insertable_headers.is_empty());
    }
}
