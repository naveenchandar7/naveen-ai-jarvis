from __future__ import annotations

from dataclasses import dataclass
from typing import Callable, Protocol


TranscriptHandler = Callable[[str], None]
ErrorHandler = Callable[[str], None]


@dataclass(frozen=True)
class VoiceConfig:
    language: str = "auto"
    sample_rate_hz: int = 16_000
    channels: int = 1


class VadProvider(Protocol):
    name: str

    def reset(self) -> None:
        ...

    def accept_level(self, rms: float, now_ms: int) -> bool:
        ...


class SttProvider(Protocol):
    name: str

    def transcribe(self, audio_path: str, config: VoiceConfig) -> str:
        ...


class TtsProvider(Protocol):
    name: str

    def speak(self, text: str) -> None:
        ...

    def stop(self) -> None:
        ...


class EnergyVad:
    """Small host-fed VAD state machine, intentionally provider-independent."""

    name = "energy-vad"

    def __init__(
        self,
        threshold: float = 0.028,
        start_ms: int = 120,
        end_ms: int = 500,
    ) -> None:
        self.threshold = threshold
        self.start_ms = start_ms
        self.end_ms = end_ms
        self.reset()

    def reset(self) -> None:
        self._speaking = False
        self._above_since: int | None = None
        self._below_since: int | None = None

    def accept_level(self, rms: float, now_ms: int) -> bool:
        above = rms >= self.threshold
        if above:
            self._below_since = None
            if self._above_since is None:
                self._above_since = now_ms
            if not self._speaking and now_ms - self._above_since >= self.start_ms:
                self._speaking = True
        else:
            self._above_since = None
            if self._below_since is None:
                self._below_since = now_ms
            if self._speaking and now_ms - self._below_since >= self.end_ms:
                self._speaking = False
        return self._speaking


class DisabledSttProvider:
    name = "disabled-stt"

    def transcribe(self, audio_path: str, config: VoiceConfig) -> str:
        del audio_path, config
        raise RuntimeError("no STT provider is configured")


class DisabledTtsProvider:
    name = "disabled-tts"

    def speak(self, text: str) -> None:
        del text
        raise RuntimeError("no TTS provider is configured")

    def stop(self) -> None:
        return None
