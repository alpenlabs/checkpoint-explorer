import time

import flexitest

from envs.services.mock_fullnode import MockFullnodeService
from envs.services.database import DatabaseService
from envs.services.backend import BackendService
from utils.api_client import ExplorerApiClient
from utils.wait import wait_until

# Ports used by the test environment (chosen to avoid clashes with dev stack)
_FULLNODE_PORT = 18000
_DB_PORT = 13306
_BACKEND_PORT = 13000

# Number of checkpoints and blocks the mock fullnode will serve
_NUM_CHECKPOINTS = 5
_BLOCKS_PER_CHECKPOINT = 10


class ExplorerLiveEnv(flexitest.LiveEnv):
    """
    Live environment: mock fullnode + MariaDB + backend binary all started in-process/subprocess.
    Tests hit the backend REST API; the backend syncs from the mock fullnode.
    """

    def __init__(
        self,
        client: ExplorerApiClient,
        fullnode: MockFullnodeService,
        database: DatabaseService,
        backend: BackendService,
    ):
        super().__init__({})  # no flexitest-managed services
        self._client = client
        self._fullnode = fullnode
        self._database = database
        self._backend = backend

    def get_explorer_client(self) -> ExplorerApiClient:
        return self._client

    def shutdown(self):
        self._backend.stop()
        self._database.stop()
        self._fullnode.stop()


class ExplorerEnvConfig(flexitest.EnvConfig):
    """
    Starts the full test stack:
      1. Mock Strata fullnode (in-process HTTP server)
      2. MariaDB container (via Docker)
      3. checkpoint-explorer backend binary

    Then waits until the backend has synced all checkpoints from the mock.
    """

    def __init__(
        self,
        num_checkpoints: int = _NUM_CHECKPOINTS,
        blocks_per_checkpoint: int = _BLOCKS_PER_CHECKPOINT,
    ):
        super().__init__()
        self.num_checkpoints = num_checkpoints
        self.blocks_per_checkpoint = blocks_per_checkpoint

    def init(self, ectx: flexitest.EnvContext) -> ExplorerLiveEnv:
        fullnode = MockFullnodeService(
            port=_FULLNODE_PORT,
            num_checkpoints=self.num_checkpoints,
            blocks_per_checkpoint=self.blocks_per_checkpoint,
        )
        fullnode.start()

        database = DatabaseService(port=_DB_PORT)
        database.start()

        backend = BackendService(
            port=_BACKEND_PORT,
            fullnode_url=fullnode.url,
            database_url=database.url,
        )
        backend.start()

        client = ExplorerApiClient(backend.base_url)

        # Wait until the backend has synced all checkpoints from the mock
        expected = self.num_checkpoints
        wait_until(
            lambda: _synced_count(client) >= expected,
            error_with=f"Backend did not sync {expected} checkpoints in time",
            timeout=30,
            step=1,
        )

        return ExplorerLiveEnv(client, fullnode, database, backend)


class ExplorerTestBase(flexitest.Test):
    """Base class for all checkpoint-explorer functional tests."""

    def get_client(self, ctx: flexitest.RunContext) -> ExplorerApiClient:
        env: ExplorerLiveEnv = ctx.env
        return env.get_explorer_client()


def _synced_count(client: ExplorerApiClient) -> int:
    try:
        resp = client.get_checkpoints(page=1, page_size=100)
        return len(resp.get("result", {}).get("items", []))
    except Exception:
        return 0
