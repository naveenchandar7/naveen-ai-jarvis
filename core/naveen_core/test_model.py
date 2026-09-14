from __future__ import annotations

import unittest

from model import (
    MODEL_TASK_CONVERSATION,
    MODEL_TASK_FAST_RESPONSE,
    MODEL_TASK_REASONING,
    HostRoutedModelProvider,
    ModelManager,
    ModelProviderRegistry,
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


class ModelProviderRegistryTests(unittest.TestCase):
    def test_register_and_get_provider(self):
        provider = RecordingProvider("test-provider")
        registry = ModelProviderRegistry()

        registry.register(provider)

        self.assertIs(registry.get("test-provider"), provider)
        self.assertEqual(registry.names(), ("test-provider",))

    def test_duplicate_provider_name_is_rejected(self):
        registry = ModelProviderRegistry()
        registry.register(RecordingProvider("same-provider"))

        with self.assertRaises(ValueError):
            registry.register(RecordingProvider("same-provider"))

    def test_empty_provider_name_is_rejected(self):
        registry = ModelProviderRegistry()

        with self.assertRaises(ValueError):
            registry.register(RecordingProvider("   "))

    def test_missing_provider_is_rejected(self):
        registry = ModelProviderRegistry()

        with self.assertRaises(KeyError):
            registry.get("missing-provider")

    def test_provider_names_are_sorted(self):
        registry = ModelProviderRegistry()
        registry.register(RecordingProvider("z-provider"))
        registry.register(RecordingProvider("a-provider"))

        self.assertEqual(
            registry.names(),
            ("a-provider", "z-provider"),
        )


class HostRoutedModelProviderTests(unittest.TestCase):
    def test_host_provider_uses_model_complete_capability(self):
        calls = []

        def capability_requester(capability_id, input_data):
            calls.append((capability_id, input_data))
            return {"content": "model response"}

        provider = HostRoutedModelProvider()

        result = provider.complete(
            ModelRequest(prompt="hello"),
            capability_requester,
        )

        self.assertEqual(result, "model response")
        self.assertEqual(
            calls,
            [("model.complete", {"prompt": "hello"})],
        )

    def test_host_provider_rejects_missing_capability_requester(self):
        provider = HostRoutedModelProvider()

        with self.assertRaises(RuntimeError):
            provider.complete(ModelRequest(prompt="hello"))

    def test_host_provider_rejects_empty_model_content(self):
        def capability_requester(capability_id, input_data):
            del capability_id, input_data
            return {"content": "   "}

        provider = HostRoutedModelProvider()

        with self.assertRaises(RuntimeError):
            provider.complete(
                ModelRequest(prompt="hello"),
                capability_requester,
            )


class ModelRoutingTests(unittest.TestCase):
    def test_register_provider_does_not_bind_a_task(self):
        conversation = RecordingProvider("conversation-provider")
        reasoning = RecordingProvider("reasoning-provider")
        manager = ModelManager(provider=conversation)

        manager.register_provider(reasoning)

        self.assertEqual(
            manager.provider_for(MODEL_TASK_REASONING).name,
            "conversation-provider",
        )
        self.assertIs(
            manager.registry.get("reasoning-provider"),
            reasoning,
        )

    def test_bind_task_selects_registered_provider(self):
        conversation = RecordingProvider("conversation-provider")
        reasoning = RecordingProvider("reasoning-provider")
        manager = ModelManager(provider=conversation)

        manager.register_provider(reasoning)
        manager.bind_task(
            MODEL_TASK_REASONING,
            "reasoning-provider",
        )

        self.assertIs(
            manager.provider_for(MODEL_TASK_REASONING),
            reasoning,
        )

    def test_register_provider_rejects_different_provider_with_same_name(self):
        conversation = RecordingProvider("conversation-provider")
        duplicate = RecordingProvider("conversation-provider")
        manager = ModelManager(provider=conversation)

        with self.assertRaises(ValueError):
            manager.register_provider(duplicate)

    def test_task_specific_provider_is_selected(self):
        conversation = RecordingProvider("conversation-provider")
        fast = RecordingProvider("fast-provider")
        manager = ModelManager(provider=conversation)

        manager.register(MODEL_TASK_FAST_RESPONSE, fast)

        self.assertEqual(
            manager.complete("hello"),
            "conversation-provider",
        )
        self.assertEqual(
            manager.complete(
                "status",
                task=MODEL_TASK_FAST_RESPONSE,
            ),
            "fast-provider",
        )
        self.assertEqual(conversation.calls, ["hello"])
        self.assertEqual(fast.calls, ["status"])

    def test_unregistered_task_falls_back_to_conversation_provider(self):
        conversation = RecordingProvider("conversation-provider")
        manager = ModelManager(provider=conversation)

        self.assertEqual(
            manager.complete(
                "reason",
                task=MODEL_TASK_REASONING,
            ),
            "conversation-provider",
        )
        self.assertEqual(conversation.calls, ["reason"])

    def test_task_registration_adds_provider_to_registry(self):
        conversation = RecordingProvider("conversation-provider")
        reasoning = RecordingProvider("reasoning-provider")
        manager = ModelManager(provider=conversation)

        manager.register(MODEL_TASK_REASONING, reasoning)

        self.assertIs(
            manager.registry.get("reasoning-provider"),
            reasoning,
        )

    def test_initial_provider_is_registered(self):
        conversation = RecordingProvider("conversation-provider")
        manager = ModelManager(provider=conversation)

        self.assertIs(
            manager.registry.get("conversation-provider"),
            conversation,
        )


if __name__ == "__main__":
    unittest.main()