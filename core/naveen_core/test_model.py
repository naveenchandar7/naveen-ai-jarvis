from __future__ import annotations

import unittest

from model import (
    MODEL_TASK_CONVERSATION,
    MODEL_TASK_FAST_RESPONSE,
    ModelManager,
    ModelRequest,
)


class RecordingProvider:
    def __init__(self, name: str) -> None:
        self.name = name
        self.calls: list[str] = []

    def complete(self, request: ModelRequest, capability_requester=None) -> str:
        del capability_requester
        self.calls.append(request.prompt)
        return self.name


class ModelRoutingTests(unittest.TestCase):
    def test_task_specific_provider_is_selected(self):
        conversation = RecordingProvider("conversation-provider")
        fast = RecordingProvider("fast-provider")
        manager = ModelManager(provider=conversation)
        manager.register(MODEL_TASK_FAST_RESPONSE, fast)

        self.assertEqual(manager.complete("hello"), "conversation-provider")
        self.assertEqual(
            manager.complete("status", task=MODEL_TASK_FAST_RESPONSE),
            "fast-provider",
        )
        self.assertEqual(conversation.calls, ["hello"])
        self.assertEqual(fast.calls, ["status"])

    def test_unregistered_task_falls_back_to_conversation_provider(self):
        conversation = RecordingProvider("conversation-provider")
        manager = ModelManager(provider=conversation)

        self.assertEqual(manager.complete("reason", task="reasoning"), "conversation-provider")
        self.assertEqual(conversation.calls, ["reason"])


if __name__ == "__main__":
    unittest.main()
