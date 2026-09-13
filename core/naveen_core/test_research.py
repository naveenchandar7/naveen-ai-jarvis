from __future__ import annotations

import unittest

from research import HostNetworkResearchProvider


class ResearchTests(unittest.TestCase):
    def test_explicit_urls_are_fetched_through_host_capability(self):
        calls = []

        def requester(capability_id, payload):
            calls.append((capability_id, payload))
            return {"content": "verified source content"}

        provider = HostNetworkResearchProvider()
        result = provider.research(
            "compare https://example.com/a and https://example.org/b",
            requester,
        )

        self.assertTrue(result.available)
        self.assertEqual(len(result.sources), 2)
        self.assertEqual(calls[0][0], "network.fetch_text")
        self.assertEqual(calls[0][1]["url"], "https://example.com/a")

    def test_missing_endpoint_is_explicitly_unavailable(self):
        provider = HostNetworkResearchProvider(endpoint_template=None)
        result = provider.research("current Java news", lambda *_: {})
        self.assertFalse(result.available)
        self.assertIn("endpoint", result.error or "")

    def test_failed_source_does_not_become_fake_evidence(self):
        def requester(*_args):
            raise RuntimeError("denied")

        provider = HostNetworkResearchProvider()
        result = provider.research("https://example.com", requester)
        self.assertFalse(result.available)
        self.assertEqual(result.sources, [])


if __name__ == "__main__":
    unittest.main()
