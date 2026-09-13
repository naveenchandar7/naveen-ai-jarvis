from __future__ import annotations

import sqlite3
import unittest

from knowledge import EmbeddingProvider, KnowledgeProvider, SQLiteKnowledgeStore


class FakeEmbeddingProvider:
    name = "fake-embeddings"

    def embed(self, text: str) -> list[float]:
        return [float(len(text))]


class KnowledgeProviderTests(unittest.TestCase):
    def test_sqlite_implementation_satisfies_knowledge_contract(self):
        connection = sqlite3.connect(":memory:")
        store = SQLiteKnowledgeStore(connection)
        self.assertIsInstance(store, KnowledgeProvider)
        store.upsert("notes.txt", "NAVEEN provider boundary")
        rows = store.search("provider")
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["path"], "notes.txt")
        self.assertEqual(store.remove("notes.txt"), 1)
        connection.close()

    def test_embedding_provider_is_a_separate_contract(self):
        provider = FakeEmbeddingProvider()
        self.assertEqual(provider.embed("NAVEEN"), [6.0])
        self.assertEqual(provider.name, "fake-embeddings")
        self.assertTrue(callable(provider.embed))
        self.assertTrue(hasattr(EmbeddingProvider, "embed"))


if __name__ == "__main__":
    unittest.main()
