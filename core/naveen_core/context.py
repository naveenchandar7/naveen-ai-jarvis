from __future__ import annotations

from collections import deque
from dataclasses import dataclass


MAX_CONTEXT_TURNS = 12
MAX_TURN_TEXT = 4096
MAX_CONTEXT_CHARS = 24_000


@dataclass(frozen=True)
class ConversationTurn:
    role: str
    content: str


class ConversationContext:
    """Bounded working memory for the active Core process.

    This is intentionally separate from durable MemoryStore. It carries only
    recent conversational context and is never persisted automatically.
    """

    def __init__(self, max_turns: int = MAX_CONTEXT_TURNS) -> None:
        self._turns: deque[ConversationTurn] = deque(maxlen=max(1, max_turns))

    def add(self, role: str, content: str) -> None:
        normalized = " ".join(content.strip().split())
        if not normalized:
            return
        self._turns.append(
            ConversationTurn(role=role, content=normalized[:MAX_TURN_TEXT])
        )

    def recent(self) -> list[ConversationTurn]:
        return list(self._turns)

    def render(self) -> str:
        lines: list[str] = []
        total = 0
        for turn in reversed(self._turns):
            line = f"{turn.role}: {turn.content}"
            if total + len(line) > MAX_CONTEXT_CHARS:
                break
            lines.append(line)
            total += len(line)
        lines.reverse()
        return "\n".join(lines)

    def clear(self) -> None:
        self._turns.clear()

    def __len__(self) -> int:
        return len(self._turns)
