from __future__ import annotations

from collections.abc import Callable
import uuid

from capabilities import SYSTEM_TELEMETRY_READ
from contracts import CoreResponse
from intent import detect_intent
from memory import SQLiteMemoryStore
from model import ModelManager

CapabilityRequester = Callable[[str, dict], dict]


class Orchestrator:
    def __init__(
        self,
        memory: SQLiteMemoryStore,
        models: ModelManager | None = None,
    ) -> None:
        self.memory = memory
        self.models = models or ModelManager()

    def handle_text(
        self,
        correlation_id: str,
        text: str,
        capability_requester: CapabilityRequester,
    ) -> CoreResponse:
        intent = detect_intent(text)

        if intent.name == "empty":
            return CoreResponse(correlation_id, "Tell me what you want to do.")

        if intent.name == "greeting":
            return CoreResponse(
                correlation_id,
                "Vanakkam. NAVEEN AI is online. Tell me what you need.",
            )

        if intent.name == "identity":
            return CoreResponse(
                correlation_id,
                "I’m NAVEEN AI — your personal assistant and second brain.",
            )

        if intent.name == "help":
            return CoreResponse(
                correlation_id,
                "I can handle text commands, explicit memory, host status, and capability workflows.",
                {
                    "available": [
                        "memory.save",
                        "memory.recall",
                        "memory.forget",
                        "system.status",
                    ]
                },
            )

        if intent.name == "memory.save":
            memory_id = self.memory.save(intent.argument)
            return CoreResponse(
                correlation_id,
                "Saved that to long-term memory.",
                {"memory_id": memory_id},
            )

        if intent.name == "memory.recall":
            rows = self.memory.recall("", limit=8)
            if not rows:
                return CoreResponse(correlation_id, "I don’t have any saved memories yet.")
            summary = "\n".join(f"- {row['content']}" for row in rows)
            return CoreResponse(
                correlation_id,
                f"Here’s what I currently remember:\n{summary}",
                {"memories": rows},
            )

        if intent.name == "memory.forget":
            count = self.memory.forget(intent.argument)
            if count:
                return CoreResponse(
                    correlation_id,
                    f"Forgot {count} matching memory item(s).",
                    {"deleted": count},
                )
            return CoreResponse(
                correlation_id,
                "I couldn’t find a matching memory to forget.",
                {"deleted": 0},
            )

        if intent.name == "system.status":
            request_id = f"cap-{uuid.uuid4().hex}"
            result = capability_requester(
                SYSTEM_TELEMETRY_READ,
                {"request_id": request_id},
            )
            cpu = float(result.get("cpu_usage", 0.0))
            memory_used = result.get("memory_used", 0)
            memory_total = result.get("memory_total", 0)
            return CoreResponse(
                correlation_id,
                (
                    f"CPU {cpu:.1f}% · memory {memory_used} / {memory_total} · "
                    f"OS {result.get('os_name', 'Unknown')} {result.get('os_version', '')}"
                ),
                {"system": result},
            )

        if intent.name == "research":
            topic = intent.argument or "the requested topic"
            return CoreResponse(
                correlation_id,
                (
                    f"I’ve understood the research request for “{topic}”. "
                    "Current-source retrieval is not enabled in this build, "
                    "so I won’t pretend that I fetched live sources."
                ),
                {"topic": topic, "available": False},
            )

        return CoreResponse(correlation_id, self.models.complete(intent.argument))
