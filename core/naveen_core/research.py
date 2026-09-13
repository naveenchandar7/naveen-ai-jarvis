from __future__ import annotations

from dataclasses import dataclass
import os
import re
from typing import Callable, Protocol
from urllib.parse import quote_plus


CapabilityRequester = Callable[[str, dict], dict]


@dataclass(frozen=True)
class ResearchSource:
    url: str
    content: str


@dataclass(frozen=True)
class ResearchResult:
    topic: str
    sources: list[ResearchSource]
    available: bool
    error: str | None = None


class ResearchProvider(Protocol):
    name: str

    def research(
        self,
        topic: str,
        capability_requester: CapabilityRequester,
    ) -> ResearchResult:
        ...


class HostNetworkResearchProvider:
    """Research adapter that can only fetch through the host capability gateway."""

    name = "host-network-research"
    _URL_RE = re.compile(r"https?://[^\s<>]+")

    def __init__(self, endpoint_template: str | None = None) -> None:
        self.endpoint_template = endpoint_template or os.getenv("NAVEEN_RESEARCH_URL_TEMPLATE")

    def research(
        self,
        topic: str,
        capability_requester: CapabilityRequester,
    ) -> ResearchResult:
        clean_topic = " ".join(topic.strip().split())
        if not clean_topic:
            return ResearchResult(clean_topic, [], False, "research topic is empty")

        urls = [url.rstrip(".,;:!?)]}") for url in self._URL_RE.findall(clean_topic)]
        if urls:
            return self._fetch_sources(urls[:8], capability_requester, clean_topic)

        if not self.endpoint_template:
            return ResearchResult(
                clean_topic,
                [],
                False,
                "research endpoint is not configured",
            )

        url = self.endpoint_template.replace("{query}", quote_plus(clean_topic))
        return self._fetch_sources([url], capability_requester, clean_topic)

    @staticmethod
    def _fetch_sources(
        urls: list[str],
        capability_requester: CapabilityRequester,
        topic: str,
    ) -> ResearchResult:
        sources: list[ResearchSource] = []
        for url in urls:
            try:
                result = capability_requester(
                    "network.fetch_text",
                    {"url": url, "request_id": f"research-{quote_plus(url)}"},
                )
                content = result.get("content")
                if isinstance(content, str) and content.strip():
                    sources.append(ResearchSource(url=url, content=content[:512 * 1024]))
            except (RuntimeError, ValueError, OSError):
                continue

        if sources:
            return ResearchResult(topic, sources, True, None)
        return ResearchResult(
            topic,
            [],
            False,
            "network retrieval failed for the configured source(s)",
        )
