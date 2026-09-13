from __future__ import annotations

import sys
import time

from core_client import CoreClient

HEARTBEAT_INTERVAL_SECONDS = 2.0


def main() -> int:
    client = CoreClient(sys.stdin.buffer, sys.stdout.buffer)
    try:
        client.authenticate()
        while True:
            client.request_health()
            time.sleep(HEARTBEAT_INTERVAL_SECONDS)
    except (EOFError, OSError, RuntimeError, ValueError):
        return 20


if __name__ == "__main__":
    raise SystemExit(main())
