import requests


class ExplorerApiClient:
    """HTTP client for the checkpoint explorer REST API."""

    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip("/")

    def get_checkpoints(self, page: int = 1, page_size: int = 10) -> dict:
        resp = requests.get(
            f"{self.base_url}/api/checkpoints",
            params={"p": page, "ps": page_size},
            timeout=10,
        )
        resp.raise_for_status()
        return resp.json()

    def get_checkpoint(self, idx: int) -> dict:
        resp = requests.get(
            f"{self.base_url}/api/checkpoint",
            params={"p": idx},
            timeout=10,
        )
        resp.raise_for_status()
        return resp.json()

    def search(self, query: str) -> dict:
        resp = requests.get(
            f"{self.base_url}/api/search",
            params={"query": query},
            timeout=10,
        )
        resp.raise_for_status()
        return resp.json()

    def search_raw(self, query: str) -> requests.Response:
        return requests.get(
            f"{self.base_url}/api/search",
            params={"query": query},
            timeout=10,
        )
