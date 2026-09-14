from __future__ import annotations

import tempfile
import unittest

from memory import MemoryStore, SQLiteMemoryStore


class MemoryTests(unittest.TestCase):
    def test_save_recall_forget(self):
        with tempfile.TemporaryDirectory() as temp:
            store = SQLiteMemoryStore(f"{temp}/memory.db")
            store.save("Python project uses FastAPI", namespace="project")
            store.save("I like farming", namespace="preferences")

            project = store.recall("fastapi")
            self.assertEqual(len(project), 1)
            self.assertEqual(project[0]["namespace"], "project")

            deleted = store.forget("farming", namespace="preferences")
            self.assertEqual(deleted, 1)
            self.assertEqual(store.recall("farming"), [])
            store.close()

    def test_query_and_namespace_are_both_applied(self):
        with tempfile.TemporaryDirectory() as temp:
            store = SQLiteMemoryStore(f"{temp}/memory.db")
            store.save("Java learning plan", namespace="learning")
            store.save("Java deployment plan", namespace="project")

            rows = store.recall("Java", namespace="learning")
            self.assertEqual(len(rows), 1)
            self.assertEqual(rows[0]["content"], "Java learning plan")
            store.close()

    def test_sqlite_implementation_satisfies_memory_store_contract(self):
        with tempfile.TemporaryDirectory() as temp:
            store = SQLiteMemoryStore(f"{temp}/memory.db")
            self.assertIsInstance(store, MemoryStore)
            store.close()


if __name__ == "__main__":
    unittest.main()
