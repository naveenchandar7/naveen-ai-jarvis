from __future__ import annotations

from dataclasses import dataclass
import re


@dataclass(frozen=True)
class Intent:
    name: str
    argument: str = ""


def _clean(text: str) -> str:
    return " ".join(text.strip().split())


def detect_intent(text: str) -> Intent:
    value = _clean(text)
    lowered = value.casefold()

    if not value:
        return Intent("empty")

    if re.search(r"\b(hi|hello|hey|vanakkam|வணக்கம்)\b", lowered):
        return Intent("greeting")

    if "who are you" in lowered or "nee yaaru" in lowered or "neenga yaar" in lowered:
        return Intent("identity")

    if lowered in {"help", "help me", "enna panra", "what can you do"}:
        return Intent("help")

    for prefix in (
        "remember ",
        "remember this ",
        "nyabagam vechuko ",
        "நினைவில் வை ",
    ):
        if lowered.startswith(prefix.casefold()):
            return Intent("memory.save", value[len(prefix) :].strip())

    if "what do you remember" in lowered or "nee enna nyabagam vechuruka" in lowered:
        about = ""
        for marker in ("what do you remember about ", "nee enna nyabagam vechuruka "):
            if lowered.startswith(marker):
                about = value[len(marker) :].strip()
                break
        return Intent("memory.recall", about)

    for prefix in ("forget ", "forget this ", "marnthudu ", "maranthudu "):
        if lowered.startswith(prefix):
            return Intent("memory.forget", value[len(prefix) :].strip())

    for prefix in (
        "read file ",
        "open file ",
        "check file ",
        "read this file ",
    ):
        if lowered.startswith(prefix):
            return Intent("file.read", value[len(prefix) :].strip())

    for prefix in (
        "search my files ",
        "search my documents ",
        "find in my files ",
        "find in my documents ",
    ):
        if lowered.startswith(prefix):
            return Intent("knowledge.search", value[len(prefix) :].strip())

    if any(
        phrase in lowered
        for phrase in (
            "system status",
            "pc status",
            "computer status",
            "system information",
            "my pc status",
            "system info",
            "pc epdi iruku",
            "computer epdi iruku",
        )
    ):
        return Intent("system.status")

    if any(
        phrase in lowered
        for phrase in (
            "research ",
            "research this",
            "find current information",
            "deep research",
            "do a research",
        )
    ):
        argument = value
        for prefix in ("research ", "Research "):
            if value.startswith(prefix):
                argument = value[len(prefix) :].strip()
                break
        return Intent("research", argument)

    return Intent("conversation", value)
