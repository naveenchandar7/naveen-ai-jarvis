from __future__ import annotations

import sqlite3
import time
from pathlib import Path


class KnowledgeStore:
    """Provider-independent local document index for the Core's first RAG slice."""

    def __init__(self, connection: sqlite3.Connection) -> None:
        self._connection = connection
        self._connection.execute(
            """
            CREATE TABLE IF NOT EXISTS knowledge_documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                source TEXT NOT NULL,
                updated_at_ms INTEGER NOT NULL
            )
            """
        )
        self._connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_knowledge_source "
            "ON knowledge_documents(source)"
        )
        self._connection.commit()

    def upsert(self, path: str, content: str, *, source: str = "filesystem") -> None:
        if not path.strip():
            raise ValueError("knowledge path is empty")
        if len(content) > 512 * 1024:
            raise ValueError("knowledge document is too large")

        now = int(time.time() * 1000)
        title = Path(path).name or path
        self._connection.execute(
            """
            INSERT INTO knowledge_documents(path, title, content, source, updated_at_ms)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(path) DO UPDATE SET
                title = excluded.title,
                content = excluded.content,
                source = excluded.source,
                updated_at_ms = excluded.updated_at_ms
            """,
            (path, title, content, source, now),
        )
        self._connection.commit()

    def search(self, query: str, *, limit: int = 8) -> list[dict[str, object]]:
        terms = [term for term in query.casefold().split() if len(term) >= 2][:8]
        if not terms:
            return []

        clauses = "(" + " OR ".join(
            ["LOWER(content) LIKE ?" for _ in terms]
            + ["LOWER(path) LIKE ?" for _ in terms]
        ) + ")"
        params = [f"%{term}%" for term in terms] * 2
        limit = max(1, min(limit, 16))

        rows = self._connection.execute(
            f"""
            SELECT path, title, content, source, updated_at_ms
            FROM knowledge_documents
            WHERE {clauses}
            ORDER BY updated_at_ms DESC
            LIMIT ?
            """,
            (*params, limit),
        ).fetchall()

        return [
            {
                "path": row[0],
                "title": row[1],
                "content": row[2],
                "source": row[3],
                "updated_at_ms": row[4],
            }
            for row in rows
        ]

    def remove(self, path: str) -> int:
        cursor = self._connection.execute(
            "DELETE FROM knowledge_documents WHERE path = ?",
            (path,),
        )
        self._connection.commit()
        return cursor.rowcount
