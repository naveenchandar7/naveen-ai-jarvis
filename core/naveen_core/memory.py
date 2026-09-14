from __future__ import annotations

import sqlite3
import time
from pathlib import Path
from typing import Protocol, runtime_checkable


MemoryRecord = dict[str, object]


@runtime_checkable
class MemoryStore(Protocol):
    """Stable Core contract for durable/policy-gated personal memory."""

    def save(
        self,
        content: str,
        *,
        namespace: str = "long-term",
        source: str = "user",
    ) -> int:
        ...

    def recall(
        self,
        query: str = "",
        *,
        namespace: str | None = None,
        limit: int = 8,
    ) -> list[MemoryRecord]:
        ...

    def forget(self, query: str, *, namespace: str | None = None) -> int:
        ...

    def close(self) -> None:
        ...


class SQLiteMemoryStore:
    """Current local implementation of the replaceable MemoryStore contract."""

    name = "sqlite-memory"

    def __init__(self, path: str | Path) -> None:
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._connection = sqlite3.connect(self.path, check_same_thread=False)
        self._connection.execute("PRAGMA journal_mode=WAL")
        self._connection.execute(
            """
            CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                namespace TEXT NOT NULL,
                content TEXT NOT NULL,
                source TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            )
            """
        )
        self._connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_memories_namespace ON memories(namespace)"
        )
        self._connection.commit()

    @property
    def connection(self) -> sqlite3.Connection:
        return self._connection

    def save(
        self,
        content: str,
        *,
        namespace: str = "long-term",
        source: str = "user",
    ) -> int:
        value = " ".join(content.strip().split())
        if not value or len(value) > 4096:
            raise ValueError("memory content is empty or too large")
        namespace = namespace.strip() or "long-term"
        now = int(time.time() * 1000)
        cursor = self._connection.execute(
            """
            INSERT INTO memories(namespace, content, source, created_at_ms, updated_at_ms)
            VALUES (?, ?, ?, ?, ?)
            """,
            (namespace, value, source, now, now),
        )
        self._connection.commit()
        return int(cursor.lastrowid)

    def recall(
        self,
        query: str = "",
        *,
        namespace: str | None = None,
        limit: int = 8,
    ) -> list[MemoryRecord]:
        limit = max(1, min(limit, 32))
        terms = [term for term in query.casefold().split() if len(term) >= 2][:8]
        clauses: list[str] = []
        params: list[object] = []

        if terms:
            clauses.append(
                "(" + " OR ".join(["LOWER(content) LIKE ?" for _ in terms]) + ")"
            )
            params.extend(f"%{term}%" for term in terms)

        if namespace:
            clauses.append("namespace = ?")
            params.append(namespace)

        where = f"WHERE {' AND '.join(clauses)}" if clauses else ""
        rows = self._connection.execute(
            f"""
            SELECT id, namespace, content, source, created_at_ms, updated_at_ms
            FROM memories
            {where}
            ORDER BY updated_at_ms DESC, id DESC
            LIMIT ?
            """,
            (*params, limit),
        ).fetchall()

        return [
            {
                "id": row[0],
                "namespace": row[1],
                "content": row[2],
                "source": row[3],
                "created_at_ms": row[4],
                "updated_at_ms": row[5],
            }
            for row in rows
        ]

    def forget(self, query: str, *, namespace: str | None = None) -> int:
        terms = [term for term in query.casefold().split() if len(term) >= 2][:8]
        if not terms:
            return 0

        clause = "(" + " OR ".join(["LOWER(content) LIKE ?" for _ in terms]) + ")"
        params: list[object] = [f"%{term}%" for term in terms]
        if namespace:
            clause += " AND namespace = ?"
            params.append(namespace)

        cursor = self._connection.execute(
            f"DELETE FROM memories WHERE {clause}",
            params,
        )
        self._connection.commit()
        return cursor.rowcount

    def close(self) -> None:
        self._connection.close()
