import flexitest

from envs import testenv


@flexitest.register
class CheckpointsPaginationTest(testenv.ExplorerTestBase):
    def __init__(self, ctx: flexitest.InitContext):
        ctx.set_env("explorer")

    def main(self, ctx: flexitest.RunContext):
        client = self.get_client(ctx)

        resp = client.get_checkpoints(page=1, page_size=10)
        assert "result" in resp, "response must have 'result' key"
        result = resp["result"]
        assert "items" in result, "result must have 'items'"
        assert "current_page" in result, "result must have 'current_page'"

        items = result["items"]
        assert isinstance(items, list), "'items' must be a list"

        if not items:
            return True

        null_l2_start_items = [item for item in items if item["l2_start"] is None]
        known_l2_start_items = [item for item in items if item["l2_start"] is not None]
        assert null_l2_start_items, "at least one checkpoint should preserve null l2_start"
        assert known_l2_start_items, "at least one checkpoint should preserve a known l2_start"

        # Validate checkpoint shape
        first = items[0]
        assert "idx" in first, "checkpoint must have 'idx'"
        assert "l1_range" in first, "checkpoint must have 'l1_range'"
        assert "l2_start" in first, "checkpoint must have 'l2_start'"
        assert "l2_end" in first, "checkpoint must have 'l2_end'"
        assert len(first["l1_range"]) == 2, "l1_range must be a 2-element list"
        assert isinstance(first["l2_end"], int), "l2_end must be an integer"

        # Verify page 2 returns different results when enough checkpoints exist
        if len(items) == 10:
            page2 = client.get_checkpoints(page=2, page_size=10)
            assert page2["result"]["items"] != items, "page 2 should differ from page 1"

        return True
