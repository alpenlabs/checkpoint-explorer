import math
import time
from collections.abc import Callable
from typing import Any


def wait_until(
    fn: Callable[[], Any],
    error_with: str = "Timed out",
    timeout: int = 30,
    step: float = 0.5,
):
    """Wait until fn() returns a truthy value."""
    for _ in range(math.ceil(timeout / step)):
        try:
            if fn():
                return
        except Exception:
            pass
        time.sleep(step)
    raise AssertionError(error_with)
