import flexitest

from envs import testenv


@flexitest.register
class CheckpointDetailTest(testenv.ExplorerTestBase):
    def __init__(self, ctx: flexitest.InitContext):
        ctx.set_env("explorer")

    def main(self, ctx: flexitest.RunContext):
        client = self.get_client(ctx)

        # Get a valid idx from the list
        resp = client.get_checkpoints(page=1, page_size=1)
        items = resp["result"]["items"]
        if not items:
            return True

        idx = items[0]["idx"]
        detail_resp = client.get_checkpoint(idx)
        assert "result" in detail_resp, "response must have 'result' key"

        result = detail_resp["result"]
        assert "items" in result, "result must have 'items'"
        detail_items = result["items"]
        assert len(detail_items) >= 1, "expected at least one item"

        cp = detail_items[0]
        assert cp["idx"] == idx, f"expected idx={idx}, got {cp['idx']}"
        assert "l1_range" in cp, "checkpoint must have 'l1_range'"
        assert "l2_range" in cp, "checkpoint must have 'l2_range'"

        l1 = cp["l1_range"]
        assert len(l1) == 2, "l1_range must be 2 elements"
        assert l1[0] <= l1[1], "l1_range start must be <= end"

        return True
