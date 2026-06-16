import flexitest

from envs import testenv
from utils.wait import wait_until

EXPECTED_HEADER_CHUNKS = 3


@flexitest.register
class ParallelBlockFetchConcurrencyTest(testenv.ExplorerTestBase):
    def __init__(self, ctx: flexitest.InitContext):
        ctx.set_env("explorer_parallel_blocks")

    def main(self, ctx: flexitest.RunContext):
        client = self.get_client(ctx)
        fullnode = self.get_fullnode(ctx)

        # 10002 blocks / backend MAX_HEADERS_RANGE=5000 gives 3 chunks in one wave.
        wait_until(
            lambda: fullnode.max_concurrent_header_requests() >= EXPECTED_HEADER_CHUNKS,
            error_with="Block header range requests did not fan out to all chunks",
            timeout=30,
            step=0.1,
        )

        latest_slot = fullnode.num_checkpoints * fullnode.blocks_per_checkpoint - 1
        # insert_block requires predecessors, so indexing the latest slot proves continuity.
        wait_until(
            lambda: "result" in client.search(str(latest_slot)),
            error_with=f"Latest block {latest_slot} was not indexed",
            timeout=60,
            step=0.5,
        )
        wait_until(
            lambda: "result" in client.search(fullnode.block_hash(latest_slot)),
            error_with=f"Latest block hash for slot {latest_slot} was not indexed",
            timeout=60,
            step=0.5,
        )

        return True
