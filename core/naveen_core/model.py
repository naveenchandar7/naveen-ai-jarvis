from __future__ import annotations

from dataclasses import dataclass
from typing import Protocol


@dataclass(frozen=True)
class ModelRequest:
    prompt: str


class ModelProvider(Protocol):
    name: str

    def complete(self, request: ModelRequest) -> str:
        ...


class TemplateModelProvider:
    name = "template-offline"

    def complete(self, request: ModelRequest) -> str:
        prompt = request.prompt.strip()
        if not prompt:
            return "Tell me what you want to do."
        return f"I’m running in offline orchestration mode right now. You said: {prompt}"


class ModelManager:
    def __init__(self, provider: ModelProvider | None = None) -> None:
        self.provider = provider or TemplateModelProvider()

    def complete(self, prompt: str) -> str:
        return self.provider.complete(ModelRequest(prompt=prompt))
