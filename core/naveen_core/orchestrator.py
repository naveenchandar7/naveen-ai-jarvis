from __future__ import annotations

from collections.abc import Callable
import uuid

from capabilities import SYSTEM_TELEMETRY_READ
from contracts import CoreResponse
from intent import detect_intent
from knowledge import KnowledgeStore
from memory import SQLiteMemoryStore
from model import ModelManager

CapabilityRequester = Callable[[str, dict], dict]


class Orchestrator:
    def __init__(
        self,
        memory: SQLiteMemoryStore,
        models: ModelManager | None = None,
        knowledge: KnowledgeStore | None = None,
    ) -> None:
        self.memory = memory
        self.models = models or ModelManager()
        self.knowledge = knowledge or KnowledgeStore(self.memory.connection)

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
                "I can handle text commands, explicit memory, approved workspace files, host status, and capability workflows.",
                {
                    "available": [
                        "memory.save",
                        "memory.recall",
                        "memory.forget",
                        "system.status",
                        "file.read",
                        "knowledge.search",
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
            rows = self.memory.recall(intent.argument, limit=8)
            if not rows:
                return CoreResponse(correlation_id, "I don’t have a matching saved memory.")
            summary = "\n".join(f"- {row['content']}" for row in rows)
            return CoreResponse(
                correlation_id,
                f"Here’s what I remember:\n{summary}",
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

        if intent.name == "file.read":
            path = intent.argument
            if not path:
                return CoreResponse(correlation_id, "Tell me which file to read.")

            request_id = f"cap-{uuid.uuid4().hex}"
            result = capability_requester(
                "filesystem.read_text",
                {
                    "request_id": request_id,
                    "path": path,
                    "max_bytes": 512 * 1024,
                },
            )
            relative_path = str(result.get("path", path))
            content = str(result.get("content", ""))
            self.knowledge.upsert(relative_path, content)
            preview = content[:4000]
            if len(content) > len(preview):
                preview += "\n... (preview truncated)"
            return CoreResponse(
                correlation_id,
                f"Read and indexed {relative_path}.\n\n{preview}",
                {
                    "path": relative_path,
                    "size_bytes": result.get(
                        "size_bytes", len(content.encode("utf-8"))
                    ),
                    "indexed": True,
                },
            )

        if intent.name == "knowledge.search":
            query = intent.argument
            rows = self.knowledge.search(query, limit=8)
            if not rows:
                return CoreResponse(
                    correlation_id,
                    "I couldn’t find that in the indexed documents.",
                    {"results": []},
                )

            snippets = []
            for row in rows:
                content = str(row["content"])
                snippets.append(f"{row['path']}: {content[:1200]}")
            return CoreResponse(
                correlation_id,
                "Here are the most relevant indexed results:\n"
                + "\n\n".join(snippets),
                {"results": rows},
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
