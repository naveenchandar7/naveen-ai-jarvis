from __future__ import annotations

import io
import json
import tempfile
import unittest

from core_client import CoreClient
from memory import SQLiteMemoryStore
from model import ModelManager
from orchestrator import Orchestrator
from runtime import CoreRuntime


class FakeClient(CoreClient):
    def __init__(self):
        super().__init__(io.BytesIO(), io.BytesIO())
        self.events = []

    def send_event(self, event_id, event_type, payload=None):
        self.events.append((event_id, event_type, payload or {}))

    def wait_for_event(self, event_type, *, predicate=None, timeout=10.0):
        for _event_id, actual_type, payload in self.events:
            if actual_type != event_type:
                continue
            event = {
                "event_id": "fake",
                "event_type": actual_type,
                "sequence": 1,
                "payload": json.dumps(payload).encode(),
            }
            if predicate is None or predicate(event):
                return event
        raise AssertionError(f"no {event_type}")


class RuntimeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        memory = SQLiteMemoryStore(f"{self.temp.name}/memory.db")
        self.client = FakeClient()
        self.runtime = CoreRuntime(self.client, Orchestrator(memory, ModelManager()))

    def tearDown(self):
        self.runtime.orchestrator.memory.close()
        self.temp.cleanup()

    def test_start_emits_ready(self):
        self.runtime.start()
        self.assertTrue(any(kind == "core.status" for _, kind, _ in self.client.events))

    def test_memory_save_and_recall_are_explicit(self):
        self.runtime.start()
        self.runtime._handle_text(
            {"correlation_id": "1", "text": "remember my favorite game is X"}
        )
        self.runtime._handle_text({"correlation_id": "2", "text": "what do you remember"})
        responses = [payload for _, kind, payload in self.client.events if kind == "core.response"]
        self.assertTrue(any("favorite game is X" in payload["message"] for payload in responses))

    def test_system_status_uses_capability_boundary(self):
        requester_calls = []

        def requester(capability_id, input_data):
            requester_calls.append((capability_id, input_data))
            return {
                "cpu_usage": 10.0,
                "memory_used": 3,
                "memory_total": 8,
                "os_name": "Test",
                "os_version": "1",
            }

        response = self.runtime.orchestrator.handle_text(
            "3",
            "system status",
            requester,
        )
        self.assertIn("CPU 10.0%", response.message)
        self.assertEqual(requester_calls[0][0], "system.telemetry.read")

    def test_unknown_host_event_is_reported(self):
        self.runtime.start()
        self.runtime.handle_event(
            {
                "event_type": "unknown.event",
                "payload": b"{}",
            }
        )
        self.assertTrue(any(kind == "core.error" for _, kind, _ in self.client.events))


if __name__ == "__main__":
    unittest.main()
