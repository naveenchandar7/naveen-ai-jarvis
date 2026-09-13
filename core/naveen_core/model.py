from __future__ import annotations

from dataclasses import dataclass
import os
from typing import Callable, Protocol


CapabilityRequester = Callable[[str, dict], dict]


@dataclass(frozen=True)
class ModelRequest:
    prompt: str


class ModelProvider(Protocol):
    name: str

    def complete(
        self,
        request: ModelRequest,
        capability_requester: CapabilityRequester | None = None,
    ) -> str:
        ...


class TemplateModelProvider:
    name = "template-offline"

    def complete(
        self,
        request: ModelRequest,
        capability_requester: CapabilityRequester | None = None,
    ) -> str:
        del capability_requester
        prompt = request.prompt.strip()
        if not prompt:
            return "Tell me what you want to do."
        return f"I’m running in offline orchestration mode right now. You said: {prompt}"


class HostRoutedModelProvider:
    name = "host-routed-model"

    def complete(
        self,
        request: ModelRequest,
        capability_requester: CapabilityRequester | None = None,
    ) -> str:
        if capability_requester is None:
            raise RuntimeError("host capability requester is unavailable")
        result = capability_requester("model.complete", {"prompt": request.prompt})
        content = result.get("content")
        if not isinstance(content, str) or not content.strip():
            raise RuntimeError("model provider returned no content")
        return content.strip()


class ModelManager:
    def __init__(self, provider: ModelProvider | None = None) -> None:
        if provider is not None:
            self.provider = provider
        elif os.getenv("NAVEEN_MODEL_ENDPOINT") and os.getenv("NAVEEN_MODEL_NAME"):
            self.provider = HostRoutedModelProvider()
        else:
            self.provider = TemplateModelProvider()

    def complete(
        self,
        prompt: str,
        capability_requester: CapabilityRequester | None = None,
    ) -> str:
        try:
            return self.provider.complete(
                ModelRequest(prompt=prompt),
                capability_requester,
            )
        except RuntimeError:
            if isinstance(self.provider, HostRoutedModelProvider):
                return TemplateModelProvider().complete(ModelRequest(prompt=prompt))
            raise
