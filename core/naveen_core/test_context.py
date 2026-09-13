from __future__ import annotations

import unittest

from context import ConversationContext


class ConversationContextTests(unittest.TestCase):
    def test_context_is_bounded(self):
        context = ConversationContext(max_turns=2)
        context.add("user", "first")
        context.add("assistant", "second")
        context.add("user", "third")

        turns = context.recent()
        self.assertEqual(len(turns), 2)
        self.assertEqual(turns[0].content, "second")
        self.assertEqual(turns[1].content, "third")

    def test_context_render_preserves_role_and_order(self):
        context = ConversationContext()
        context.add("user", "Hello")
        context.add("assistant", "Vanakkam")
        self.assertEqual(context.render(), "user: Hello\nassistant: Vanakkam")

    def test_context_does_not_persist_implicitly(self):
        context = ConversationContext()
        context.add("user", "remember this only for the turn")
        context.clear()
        self.assertEqual(context.recent(), [])
        self.assertEqual(len(context), 0)


if __name__ == "__main__":
    unittest.main()
