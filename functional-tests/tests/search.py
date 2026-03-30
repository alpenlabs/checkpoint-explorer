import flexitest

from envs import testenv


@flexitest.register
class SearchTest(testenv.ExplorerTestBase):
    def __init__(self, ctx: flexitest.InitContext):
        ctx.set_env("explorer")

    def main(self, ctx: flexitest.RunContext):
        client = self.get_client(ctx)

        # Search by a known checkpoint idx — returns the page number
        resp = client.get_checkpoints(page=1, page_size=1)
        items = resp["result"]["items"]
        if items:
            idx = items[0]["idx"]
            search_result = client.search(str(idx))
            assert "result" in search_result, "search must return 'result'"
            assert isinstance(search_result["result"], int), "result should be a page number"

        # Unknown query should return an error (not a crash)
        err_result = client.search_raw("nonexistent_query_xyz")
        assert err_result.status_code in (200, 400, 404), (
            f"unexpected status: {err_result.status_code}"
        )
        if err_result.status_code == 200:
            body = err_result.json()
            assert "error" in body, "expected 'error' key for unknown query"

        return True
