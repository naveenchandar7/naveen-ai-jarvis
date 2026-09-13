from __future__ import annotations

from dataclasses import dataclass
import re


@dataclass(frozen=True)
class Intent:
    name: str
    argument: str = ""


def _clean(text: str) -> str:
    return " ".join(text.strip().split())


def _starts_with_any(value: str, prefixes: tuple[str, ...]) -> str | None:
    lowered = value.casefold()
    for prefix in prefixes:
        if lowered.startswith(prefix.casefold()):
            return value[len(prefix) :].strip()
    return None


def detect_intent(text: str) -> Intent:
    value = _clean(text)
    lowered = value.casefold()

    if not value:
        return Intent("empty")

    if re.search(r"(?:^|\s)(hi|hello|hey|hai|vanakkam|vanakam|வணக்கம்)(?:\s|$)", lowered):
        return Intent("greeting")

    if any(
        phrase in lowered
        for phrase in (
            "who are you",
            "what are you",
            "nee yaaru",
            "neenga yaar",
            "nee yaru",
            "un peru enna",
            "உன் பேர் என்ன",
        )
    ):
        return Intent("identity")

    if lowered in {
        "help",
        "help me",
        "enna panra",
        "enna panna mudiyum",
        "nee enna panna mudiyum",
        "what can you do",
        "what do you do",
        "commands",
        "உன்னால என்ன பண்ண முடியும்",
    }:
        return Intent("help")

    argument = _starts_with_any(
        value,
        (
            "remember this ",
            "remember ",
            "nyabagam vechuko ",
            "nyabagam vachuko ",
            "ninaivil vechuko ",
            "நினைவில் வை ",
            "நியாபகம் வெச்சுக்கோ ",
        ),
    )
    if argument is not None:
        return Intent("memory.save", argument)

    recall_marker = _starts_with_any(
        value,
        (
            "what do you remember about ",
            "what do you remember ",
            "nee enna nyabagam vechuruka ",
            "nee enna nyabagam vachiruka ",
            "nyabagam iruka ",
            "என்ன ஞாபகம் வச்சிருக்க",
        ),
    )
    if recall_marker is not None:
        return Intent("memory.recall", recall_marker)
    if any(
        phrase in lowered
        for phrase in (
            "what do you remember",
            "what all do you remember",
            "nee enna nyabagam vechuruka",
            "nee enna nyabagam vachiruka",
            "என்ன ஞாபகம் வச்சிருக்க",
        )
    ):
        return Intent("memory.recall")

    argument = _starts_with_any(
        value,
        (
            "forget this ",
            "forget ",
            "marnthudu ",
            "maranthudu ",
            "marandhudu ",
            "ithu maranthudu ",
            "இதை மறந்துடு ",
        ),
    )
    if argument is not None:
        return Intent("memory.forget", argument)

    argument = _starts_with_any(
        value,
        (
            "read this file ",
            "read file ",
            "open file ",
            "check file ",
            "file read ",
            "இந்த file படி ",
        ),
    )
    if argument is not None:
        return Intent("file.read", argument)

    argument = _starts_with_any(
        value,
        (
            "search my files ",
            "search my documents ",
            "find in my files ",
            "find in my documents ",
            "search documents ",
            "my files la search pannu ",
            "en files la thedu ",
        ),
    )
    if argument is not None:
        return Intent("knowledge.search", argument)

    if any(
        phrase in lowered
        for phrase in (
            "system status",
            "pc status",
            "computer status",
            "system information",
            "my pc status",
            "system info",
            "how is my pc",
            "pc epdi iruku",
            "computer epdi iruku",
            "system epdi iruku",
            "enoda pc epdi iruku",
            "என் pc எப்படி இருக்கு",
        )
    ):
        return Intent("system.status")

    argument = _starts_with_any(
        value,
        (
            "research ",
            "research this ",
            "do research on ",
            "deep research ",
            "aazhamaga research pannu ",
            "research pannu ",
            "ஆராய்ச்சி செய் ",
        ),
    )
    if argument is not None:
        return Intent("research", argument)

    if any(
        phrase in lowered
        for phrase in (
            "research this",
            "find current information",
            "deep research",
            "do a research",
            "latest information about",
        )
    ):
        return Intent("research", value)

    return Intent("conversation", value)
