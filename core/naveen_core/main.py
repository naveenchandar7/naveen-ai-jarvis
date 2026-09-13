from __future__ import annotations

import os
import sqlite3
import sys
import time

from core_client import CoreClient
from memory import SQLiteMemoryStore
from model import ModelManager
from orchestrator import Orchestrator
from runtime import CoreRuntime

HEARTBEAT_INTERVAL_SECONDS = 2.0


def main() -> int:
    client = CoreClient(sys.stdin.buffer, sys.stdout.buffer)
    memory = None
    try:
        client.authenticate()

        db_path = os.environ.get("NAVEEN_MEMORY_DB")
        if not db_path:
            raise RuntimeError("NAVEEN_MEMORY_DB is not configured")

        memory = SQLiteMemoryStore(db_path)
        runtime = CoreRuntime(client, Orchestrator(memory, ModelManager()))
        runtime.start()

        while True:
            client.request_health()
            time.sleep(HEARTBEAT_INTERVAL_SECONDS)
    except (EOFError, OSError, RuntimeError, ValueError, sqlite3.Error, TimeoutError):
        return 20
    finally:
        if memory is not None:
            memory.close()


if __name__ == "__main__":
    raise SystemExit(main())
