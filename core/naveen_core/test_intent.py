from __future__ import annotations

import unittest

from intent import detect_intent


class IntentTests(unittest.TestCase):
    def test_greeting(self):
        self.assertEqual(detect_intent("Vanakkam").name, "greeting")

    def test_memory_save(self):
        intent = detect_intent("remember my favorite editor is VS Code")
        self.assertEqual(intent.name, "memory.save")
        self.assertEqual(intent.argument, "my favorite editor is VS Code")

    def test_system_status(self):
        self.assertEqual(detect_intent("what is my pc status?").name, "system.status")

    def test_research(self):
        intent = detect_intent("research current Java releases")
        self.assertEqual(intent.name, "research")
        self.assertEqual(intent.argument, "current Java releases")

    def test_fallback_conversation(self):
        intent = detect_intent("tell me something useful")
        self.assertEqual(intent.name, "conversation")


if __name__ == "__main__":
    unittest.main()
