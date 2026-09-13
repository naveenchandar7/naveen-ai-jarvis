from __future__ import annotations

from collections.abc import Mapping
import os
from typing import Literal, Protocol

from contracts import CoreResponse

CapabilityRequester = callable
ModelTask = Literal["conversation", "fast_response", "reasoning", "research_synthesis"]
MODEL_TASK_CONVERSATION: ModelTask = "conversation"
MODEL_TASK_FAST_RESPONSE: ModelTask = "fast_response"
MODEL_TASK_REASONING: ModelTask = "reasoning"
MODEL_TASK_RESEARCH: ModelTask = "research_synthesis"


class ModelRequest:
    def __init__(self, prompt: str) -> None:
        self.prompt = prompt


class ModelProvider(Protocol):
    name: str

    def complete(
        self,
        request: ModelRequest,
        capability_requester=None,
    ) -> str:
        ...


class TemplateModelProvider:
    name = "template-offline"

    def complete(self, request: ModelRequest, capability_requester=None) -> str:
        del capability_requester
        prompt = request.prompt.strip()
        if not prompt:
            return "Tell me what you want to do."
        return f"I’m running in offline orchestration mode right now. You said: {prompt}"


class HostRoutedModelProvider:
    name = "host-routed-model"

    def complete(self, request: ModelRequest, capability_requester=None) -> str:
        if capability_requester is None:
            raise RuntimeError("host capability requester is unavailable")
        result = capability_requester("model.complete", {"prompt": request.prompt})
        content = result.get("content")
        if not isinstance(content, str) or not content.strip():
            raise RuntimeError("model provider returned no content")
        return content.strip()


class ModelManager:
    def __init__(
        self,
        provider: ModelProvider | None = None,
        providers: Mapping[ModelTask, ModelProvider] | None = None,
    ) -> None:
        if provider is not None:
            default_provider = provider
        elif os.getenv("NAVEEN_MODEL_ENDPOINT") and os.getenv("NAVEEN_MODEL_NAME"):
            default_provider = HostRoutedModelProvider()
        else:
            default_provider = TemplateModelProvider()

        self._providers: dict[ModelTask, ModelProvider] = {
            MODEL_TASK_CONVERSATION: default_provider,
        }
        if providers:
            self._providers.update(providers)
        self.provider = default_provider

    def register(self, task: ModelTask, provider: ModelProvider) -> None:
        self._providers[task] = provider

    def provider_for(self, task: ModelTask) -> ModelProvider:
        return self._providers.get(task, self._providers[MODEL_TASK_CONVERSATION])

    def complete(
        self,
        prompt: str,
        capability_requester=None,
        *,
        task: ModelTask = MODEL_TASK_CONVERSATION,
    ) -> str:
        provider = self.provider_for(task)
        try:
            return provider.complete(
                ModelRequest(prompt=prompt),
                capability_requester,
            )
        except RuntimeError:
            if isinstance(provider, HostRoutedModelProvider):
                return TemplateModelProvider().complete(ModelRequest(prompt=prompt))
            raise
