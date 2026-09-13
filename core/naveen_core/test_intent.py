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

    def test_thanglish_memory_save(self):
        intent = detect_intent("nyabagam vechuko en favorite editor VS Code")
        self.assertEqual(intent.name, "memory.save")
        self.assertEqual(intent.argument, "en favorite editor VS Code")

    def test_thanglish_system_status(self):
        self.assertEqual(detect_intent("enoda pc epdi iruku").name, "system.status")

    def test_thanglish_file_search(self):
        intent = detect_intent("en files la thedu Java notes")
        self.assertEqual(intent.name, "knowledge.search")
        self.assertEqual(intent.argument, "Java notes")

    def test_memory_recall(self):
        intent = detect_intent("what do you remember about my work")
        self.assertEqual(intent.name, "memory.recall")
        self.assertEqual(intent.argument, "my work")

    def test_system_status(self):
        self.assertEqual(detect_intent("what is my pc status?").name, "system.status")

    def test_research(self):
        intent = detect_intent("research current Java releases")
        self.assertEqual(intent.name, "research")
        self.assertEqual(intent.argument, "current Java releases")

    def test_tamil_research(self):
        intent = detect_intent("ஆராய்ச்சி செய் Java 25")
        self.assertEqual(intent.name, "research")
        self.assertEqual(intent.argument, "Java 25")

    def test_fallback_conversation(self):
        intent = detect_intent("tell me something useful")
        self.assertEqual(intent.name, "conversation")


if __name__ == "__main__":
    unittest.main()
