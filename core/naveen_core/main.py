from __future__ import annotations

import argparse
import os
import sqlite3
import sys
import time

from core_client import CoreClient
from knowledge import SQLiteKnowledgeStore
from memory import SQLiteMemoryStore
from model import ModelManager
from orchestrator import Orchestrator
from runtime import CoreRuntime

HEARTBEAT_INTERVAL_SECONDS = 2.0


def _memory_db_path() -> str:
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--memory-db")
    args, _ = parser.parse_known_args()
    db_path = args.memory_db or os.environ.get("NAVEEN_MEMORY_DB")
    if not db_path:
        raise RuntimeError("NAVEEN_MEMORY_DB is not configured")
    return db_path


def main() -> int:
    client = CoreClient(sys.stdin.buffer, sys.stdout.buffer)
    memory = None
    try:
        client.authenticate()

        memory = SQLiteMemoryStore(_memory_db_path())
        knowledge = SQLiteKnowledgeStore(memory.connection)
        runtime = CoreRuntime(
            client,
            Orchestrator(
                memory,
                ModelManager(),
                knowledge=knowledge,
            ),
        )
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
