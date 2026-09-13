from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass(frozen=True)
class CoreResponse:
    correlation_id: str
    message: str
    data: dict[str, Any] = field(default_factory=dict)


@dataclass(frozen=True)
class CapabilityResult:
    request_id: str
    capability_id: str
    ok: bool
    output: dict[str, Any] = field(default_factory=dict)
    error: str | None = None
