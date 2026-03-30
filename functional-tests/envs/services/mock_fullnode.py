"""Mock Strata fullnode serving checkpoints and blocks over JSON-RPC 2.0."""

import hashlib
import json
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer


def _hex(seed: int, namespace: int = 0) -> str:
    """Deterministic 64-char hex string from a seed and namespace."""
    data = (seed + namespace * 1_000_000).to_bytes(8, "big")
    return hashlib.sha256(data).hexdigest()


class _Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        pass  # suppress per-request logs

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        body = json.loads(self.rfile.read(length))
        method = body.get("method", "")
        params = body.get("params", [])
        rpc_id = body.get("id", 1)

        result = self.server.dispatch(method, params)
        resp_bytes = json.dumps({"jsonrpc": "2.0", "result": result, "id": rpc_id}).encode()

        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(resp_bytes)))
        self.end_headers()
        self.wfile.write(resp_bytes)


class _MockServer(HTTPServer):
    def __init__(self, host: str, port: int, num_checkpoints: int, blocks_per_checkpoint: int):
        super().__init__((host, port), _Handler)
        self._n = num_checkpoints
        self._bpc = blocks_per_checkpoint

    def dispatch(self, method: str, params: list):
        if method == "strata_getLatestCheckpointIndex":
            return self._n - 1
        if method == "strata_getCheckpointInfo":
            return self._checkpoint_info(int(params[0]))
        if method == "strata_getHeadersAtIdx":
            return self._headers_at_idx(int(params[0]))
        return None

    def _checkpoint_info(self, idx: int):
        if idx < 0 or idx >= self._n:
            return None

        l2_start = idx * self._bpc
        l2_end = l2_start + self._bpc - 1
        l1_start = idx * 10
        l1_end = l1_start + 9

        # Older checkpoints are finalized; newest two are confirmed/pending
        if idx < self._n - 2:
            status = "finalized"
        elif idx == self._n - 2:
            status = "confirmed"
        else:
            status = "pending"

        return {
            "idx": idx,
            "l1_range": [
                {"height": l1_start, "blkid": _hex(l1_start, namespace=1)},
                {"height": l1_end, "blkid": _hex(l1_end, namespace=1)},
            ],
            "l2_range": [
                {"slot": l2_start, "blkid": _hex(l2_start, namespace=2)},
                {"slot": l2_end, "blkid": _hex(l2_end, namespace=2)},
            ],
            "l1_reference": {
                "block_height": l1_end,
                "block_id": _hex(l1_end, namespace=1),
                "txid": _hex(idx, namespace=3),
                "wtxid": _hex(idx, namespace=4),
            },
            "confirmation_status": status,
        }

    def _headers_at_idx(self, height: int):
        prev = _hex(height - 1, namespace=2) if height > 0 else "0" * 64
        return [
            {
                "block_idx": height,
                "timestamp": 1_700_000_000 + height * 12,
                "block_id": _hex(height, namespace=2),
                "prev_block": prev,
                "l1_segment_hash": _hex(height, namespace=5),
                "exec_segment_hash": _hex(height, namespace=6),
                "state_root": _hex(height, namespace=7),
            }
        ]


class MockFullnodeService:
    """
    In-process mock Strata fullnode.

    Generates a deterministic set of checkpoints and blocks:
    - Checkpoint i covers L2 blocks [i*blocks_per_checkpoint, (i+1)*blocks_per_checkpoint - 1]
    - Checkpoint i covers L1 blocks [i*10, i*10 + 9]
    - All block hashes are deterministic (sha256-based)
    - Status: finalized (old), confirmed (second-to-last), pending (latest)
    """

    def __init__(self, port: int, num_checkpoints: int = 5, blocks_per_checkpoint: int = 10):
        self.port = port
        self.url = f"http://127.0.0.1:{port}/"
        self._server = _MockServer("127.0.0.1", port, num_checkpoints, blocks_per_checkpoint)
        self._thread: threading.Thread | None = None
        self.num_checkpoints = num_checkpoints
        self.blocks_per_checkpoint = blocks_per_checkpoint

    def start(self):
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)
        self._thread.start()

    def stop(self):
        self._server.shutdown()
        if self._thread:
            self._thread.join(timeout=5)
