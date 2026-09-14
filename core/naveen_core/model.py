from __future__ import annotations

from collections.abc import Callable, Mapping
from dataclasses import dataclass
import os
from typing import Literal, Protocol

CapabilityRequester = Callable[[str, dict], dict]
ModelTask = Literal["conversation", "fast_response", "reasoning", "research_synthesis"]
MODEL_TASK_CONVERSATION: ModelTask = "conversation"
MODEL_TASK_FAST_RESPONSE: ModelTask = "fast_response"
MODEL_TASK_REASONING: ModelTask = "reasoning"
MODEL_TASK_RESEARCH: ModelTask = "research_synthesis"

MODEL_PROVIDER_TEMPLATE_OFFLINE = "template-offline"
MODEL_PROVIDER_HOST_ROUTED = "host-routed-model"
DEFAULT_MODEL_PROVIDER = MODEL_PROVIDER_TEMPLATE_OFFLINE


@dataclass(frozen=True)
class ModelConfig:
    """Provider-independent runtime configuration for model execution."""

    provider: str
    endpoint: str | None = None
    model: str | None = None
    api_style: str = "openai_compatible"
    credential_ref: str | None = None

    def __post_init__(self) -> None:
        if not self.provider.strip():
            raise ValueError("model provider must not be empty")
        if not self.api_style.strip():
            raise ValueError("model API style must not be empty")

    @classmethod
    def from_environment(cls, environ: Mapping[str, str] | None = None) -> "ModelConfig":
        """Build runtime configuration from the process environment."""
        values = os.environ if environ is None else environ
        provider = values.get("NAVEEN_MODEL_PROVIDER", DEFAULT_MODEL_PROVIDER).strip()
        endpoint = values.get("NAVEEN_MODEL_ENDPOINT", "").strip() or None
        model = values.get("NAVEEN_MODEL_NAME", "").strip() or None
        api_style = values.get("NAVEEN_MODEL_API_STYLE", "openai_compatible").strip()
        credential_ref = values.get("NAVEEN_MODEL_CREDENTIAL_REF", "").strip() or None

        return cls(
            provider=provider or DEFAULT_MODEL_PROVIDER,
            endpoint=endpoint,
            model=model,
            api_style=api_style,
            credential_ref=credential_ref,
        )


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
    name = MODEL_PROVIDER_TEMPLATE_OFFLINE

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
    name = MODEL_PROVIDER_HOST_ROUTED

    def complete(
        self,
        request: ModelRequest,
        capability_requester: CapabilityRequester | None = None,
    ) -> str:
        if capability_requester is None:
            raise RuntimeError("host capability requester is unavailable")

        result = capability_requester(
            "model.complete",
            {"prompt": request.prompt},
        )
        content = result.get("content")

        if not isinstance(content, str) or not content.strip():
            raise RuntimeError("model provider returned no content")

        return content.strip()


ModelProviderFactory = Callable[[ModelConfig], ModelProvider]


class ModelProviderRegistry:
    """Named registry for instantiated, replaceable model providers."""

    def __init__(self) -> None:
        self._providers: dict[str, ModelProvider] = {}

    def register(self, provider: ModelProvider) -> None:
        name = provider.name.strip()

        if not name:
            raise ValueError("model provider name must not be empty")

        if name in self._providers:
            raise ValueError(f"model provider already registered: {name}")

        self._providers[name] = provider

    def get(self, name: str) -> ModelProvider:
        try:
            return self._providers[name]
        except KeyError as error:
            raise KeyError(
                f"model provider is not registered: {name}"
            ) from error

    def names(self) -> tuple[str, ...]:
        return tuple(sorted(self._providers))


class ModelProviderResolver:
    """Resolve runtime provider configuration through registered factories."""

    def __init__(self) -> None:
        self._factories: dict[str, ModelProviderFactory] = {}

    def register(self, provider_name: str, factory: ModelProviderFactory) -> None:
        name = provider_name.strip()
        if not name:
            raise ValueError("model provider name must not be empty")
        if name in self._factories:
            raise ValueError(f"model provider factory already registered: {name}")
        self._factories[name] = factory

    def resolve(self, config: ModelConfig) -> ModelProvider:
        try:
            factory = self._factories[config.provider]
        except KeyError as error:
            raise KeyError(
                f"model provider factory is not registered: {config.provider}"
            ) from error
        return factory(config)

    def names(self) -> tuple[str, ...]:
        return tuple(sorted(self._factories))


def create_default_model_provider_resolver() -> ModelProviderResolver:
    """Create the built-in provider factory registry."""
    resolver = ModelProviderResolver()
    resolver.register(
        TemplateModelProvider.name,
        lambda config: TemplateModelProvider(),
    )
    resolver.register(
        HostRoutedModelProvider.name,
        lambda config: HostRoutedModelProvider(),
    )
    return resolver


class ModelManager:
    """Task-aware model router; provider construction remains registry-driven."""

    def __init__(
        self,
        provider: ModelProvider | None = None,
        providers: Mapping[ModelTask, ModelProvider] | None = None,
        config: ModelConfig | None = None,
        resolver: ModelProviderResolver | None = None,
    ) -> None:
        runtime_config = config or ModelConfig.from_environment()
        provider_resolver = resolver or create_default_model_provider_resolver()

        default_provider = (
            provider
            if provider is not None
            else provider_resolver.resolve(runtime_config)
        )

        self.config = runtime_config
        self.resolver = provider_resolver
        self.registry = ModelProviderRegistry()
        self.register_provider(default_provider)

        self._providers: dict[ModelTask, ModelProvider] = {
            MODEL_TASK_CONVERSATION: default_provider,
        }

        if providers:
            for task, task_provider in providers.items():
                self.register_provider(task_provider)
                self.bind_task(task, task_provider.name)

        self.provider = default_provider

    def register_provider(self, provider: ModelProvider) -> None:
        """Add a provider to the named registry without assigning any task."""
        name = provider.name.strip()

        if not name:
            raise ValueError("model provider name must not be empty")

        try:
            existing = self.registry.get(name)
        except KeyError:
            self.registry.register(provider)
            return

        if existing is not provider:
            raise ValueError(f"model provider already registered: {name}")

    def bind_task(self, task: ModelTask, provider_name: str) -> None:
        """Route a task to an already registered provider by stable name."""
        self._providers[task] = self.registry.get(provider_name)

    def register(self, task: ModelTask, provider: ModelProvider) -> None:
        """Compatibility helper that registers and binds a provider to a task."""
        self.register_provider(provider)
        self.bind_task(task, provider.name)

    def provider_for(self, task: ModelTask) -> ModelProvider:
        return self._providers.get(
            task,
            self._providers[MODEL_TASK_CONVERSATION],
        )

    def complete(
        self,
        prompt: str,
        capability_requester: CapabilityRequester | None = None,
        *,
        task: ModelTask = MODEL_TASK_CONVERSATION,
    ) -> str:
        provider = self.provider_for(task)
        return provider.complete(
            ModelRequest(prompt=prompt),
            capability_requester,
        )
