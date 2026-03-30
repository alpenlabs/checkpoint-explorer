"""Starts the checkpoint-explorer backend binary as a subprocess."""

import os
import subprocess
import time
from pathlib import Path

import requests


def _find_binary() -> str:
    """Locate the compiled backend binary relative to this file."""
    here = Path(__file__).resolve()
    # functional-tests/envs/services/ -> functional-tests/ -> repo root
    repo_root = here.parents[3]
    binary = repo_root / "backend" / "target" / "debug" / "checkpoint-explorer"
    if not binary.exists():
        raise FileNotFoundError(
            f"Backend binary not found at {binary}. Run 'cargo build' in backend/ first."
        )
    return str(binary)


class BackendService:
    """
    Runs the checkpoint-explorer binary with test configuration.

    Points at a mock Strata fullnode and a test database.
    Fast polling intervals (1s) so data syncs quickly during tests.
    """

    def __init__(
        self,
        port: int,
        fullnode_url: str,
        database_url: str,
        fetch_interval: int = 1,
        status_update_interval: int = 1,
    ):
        self.port = port
        self.base_url = f"http://127.0.0.1:{port}"
        self._env = {
            **os.environ,
            "APP_SERVER_PORT": str(port),
            "STRATA_FULLNODE": fullnode_url,
            "APP_DATABASE_URL": database_url,
            "APP_FETCH_INTERVAL": str(fetch_interval),
            "APP_STATUS_UPDATE_INTERVAL": str(status_update_interval),
            "RUST_LOG": "warn",
        }
        self._proc: subprocess.Popen | None = None

    def start(self):
        binary = _find_binary()
        self._proc = subprocess.Popen(
            [binary],
            env=self._env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        self._wait_ready()

    def stop(self):
        if self._proc and self._proc.poll() is None:
            self._proc.terminate()
            try:
                self._proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self._proc.kill()

    def _wait_ready(self, timeout: int = 30):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                r = requests.get(f"{self.base_url}/api/checkpoints", timeout=1)
                if r.status_code == 200:
                    return
            except requests.RequestException:
                pass
            time.sleep(0.5)
        raise RuntimeError(f"Backend on port {self.port} did not start in time")
