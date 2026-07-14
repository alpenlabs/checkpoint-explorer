import flexitest

from envs import testenv


@flexitest.register
class CheckpointDetailTest(testenv.ExplorerTestBase):
    def __init__(self, ctx: flexitest.InitContext):
        ctx.set_env("explorer")

    def main(self, ctx: flexitest.RunContext):
        client = self.get_client(ctx)

        # Get a checkpoint that exercises the nullable l2_start path.
        resp = client.get_checkpoints(page=1, page_size=10)
        items = resp["result"]["items"]
        if not items:
            return True

        null_l2_start_items = [item for item in items if item["l2_start"] is None]
        known_l2_start_items = [item for item in items if item["l2_start"] is not None]
        assert null_l2_start_items, "expected at least one checkpoint with null l2_start"
        assert known_l2_start_items, "expected at least one checkpoint with known l2_start"

        idx = null_l2_start_items[0]["idx"]
        detail_resp = client.get_checkpoint(idx)
        assert "result" in detail_resp, "response must have 'result' key"

        result = detail_resp["result"]
        assert "items" in result, "result must have 'items'"
        detail_items = result["items"]
        assert len(detail_items) >= 1, "expected at least one item"

        cp = detail_items[0]
        assert cp["idx"] == idx, f"expected idx={idx}, got {cp['idx']}"
        assert "l1_range" in cp, "checkpoint must have 'l1_range'"
        assert "l2_start" in cp, "checkpoint must have 'l2_start'"
        assert "l2_end" in cp, "checkpoint must have 'l2_end'"

        l1 = cp["l1_range"]
        assert len(l1) == 2, "l1_range must be 2 elements"
        assert l1[0] <= l1[1], "l1_range start must be <= end"
        assert cp["l2_start"] is None, "null l2_start must round-trip through the API"
        assert isinstance(cp["l2_end"], int), "l2_end must be an integer"

        return True
