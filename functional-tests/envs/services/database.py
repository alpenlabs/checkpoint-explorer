"""Starts a MariaDB Docker container for functional tests."""

import subprocess
import time


class DatabaseService:
    """
    Manages a MariaDB Docker container for tests.

    Uses a unique container name per port to allow parallel test runs.
    """

    DB_USER = "explorer"
    DB_PASSWORD = "password"
    DB_NAME = "checkpoint_explorer_test"
    DB_ROOT_PASSWORD = "rootpassword"

    def __init__(self, port: int = 13306):
        self.port = port
        self._container = f"cex-test-mariadb-{port}"
        self.url = (
            f"mysql://{self.DB_USER}:{self.DB_PASSWORD}"
            f"@127.0.0.1:{port}/{self.DB_NAME}"
        )

    def start(self):
        # Remove stale container if it exists
        subprocess.run(
            ["docker", "rm", "-f", self._container],
            capture_output=True,
        )
        subprocess.run(
            [
                "docker", "run", "-d",
                "--name", self._container,
                "-e", f"MARIADB_USER={self.DB_USER}",
                "-e", f"MARIADB_PASSWORD={self.DB_PASSWORD}",
                "-e", f"MARIADB_DATABASE={self.DB_NAME}",
                "-e", f"MARIADB_ROOT_PASSWORD={self.DB_ROOT_PASSWORD}",
                "-p", f"{self.port}:3306",
                "mariadb:11",
            ],
            check=True,
            capture_output=True,
        )
        self._wait_ready()

    def stop(self):
        subprocess.run(
            ["docker", "rm", "-f", self._container],
            capture_output=True,
        )

    def _wait_ready(self, timeout: int = 60):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            # Verify the test user and database are fully initialized, not just that
            # the server process is running (mariadb-admin ping passes too early).
            result = subprocess.run(
                [
                    "docker", "exec", self._container,
                    "mariadb",
                    f"-u{self.DB_USER}", f"-p{self.DB_PASSWORD}",
                    self.DB_NAME,
                    "-e", "SELECT 1",
                ],
                capture_output=True,
            )
            if result.returncode == 0:
                return
            time.sleep(1)
        raise RuntimeError(f"MariaDB container {self._container!r} did not become ready in time")
