from __future__ import annotations

import unittest

from voice import EnergyVad


class EnergyVadTests(unittest.TestCase):
    def test_voice_starts_only_after_start_window(self):
        vad = EnergyVad(threshold=0.1, start_ms=100, end_ms=300)
        self.assertFalse(vad.accept_level(0.2, 0))
        self.assertFalse(vad.accept_level(0.2, 99))
        self.assertTrue(vad.accept_level(0.2, 100))

    def test_voice_ends_only_after_silence_window(self):
        vad = EnergyVad(threshold=0.1, start_ms=100, end_ms=300)
        vad.accept_level(0.2, 0)
        self.assertTrue(vad.accept_level(0.2, 100))
        self.assertTrue(vad.accept_level(0.0, 399))
        self.assertFalse(vad.accept_level(0.0, 400))

    def test_noise_does_not_enter_speaking_state(self):
        vad = EnergyVad(threshold=0.1, start_ms=100, end_ms=300)
        self.assertFalse(vad.accept_level(0.02, 0))
        self.assertFalse(vad.accept_level(0.08, 1000))

    def test_reset_clears_state(self):
        vad = EnergyVad(threshold=0.1, start_ms=0, end_ms=300)
        self.assertTrue(vad.accept_level(0.2, 0))
        vad.reset()
        self.assertFalse(vad.accept_level(0.0, 1))


if __name__ == "__main__":
    unittest.main()
